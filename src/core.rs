use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware, state::InMemoryState,
    state::NotKeyed,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;

use super::config::{EdgarConfig, EdgarUrls};
use super::error::{EdgarError, Result};

const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 1000; // 1 second

/// Consecutive successful requests required before the adaptive limiter raises
/// its rate by one request per second.
///
/// Recovery is deliberately slower than the decrease — that asymmetry is what
/// makes AIMD stable — but not so slow that a run which overshot on the way down
/// spends the rest of its life at the floor. Halving is unavoidably coarse: a
/// descent from 32 req/s passes through 16, 8, 4, 2 and can land at 1 before the
/// server stops complaining, and only additive increase brings it back.
const SUCCESSES_PER_RATE_INCREASE: u32 = 10;

/// Minimum interval between two rate reductions.
///
/// A single congestion event rejects every request in flight at once, and each
/// rejection reports back. Halving per report collapses the rate far below the
/// server's actual capacity — eight concurrent 429s take 20 req/s to 1 in one
/// round trip. Reductions are therefore collapsed into one per window, so the
/// rate halves once per congestion event rather than once per victim.
const RATE_DECREASE_COOLDOWN: Duration = Duration::from_millis(500);

type Governor = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// A token-bucket limiter that lowers its own rate when the server pushes back.
///
/// # Why a plain limiter is not enough
///
/// Exponential backoff reschedules *one* request. It does nothing about the rate
/// the other in-flight requests are still being issued at, so under a shared
/// server-side quota a backing-off request wakes into exactly the conditions
/// that rejected it. Measured against SEC EDGAR from a fleet of concurrent
/// workers, that produced a 0 % recovery rate: every throttled request burned
/// all five retries and then failed.
///
/// This limiter closes the loop by treating a 429 as a signal to slow down, not
/// merely to wait — standard AIMD congestion control:
///
/// * **Multiplicative decrease** — a 429 halves the permitted rate immediately.
/// * **Additive increase** — every [`SUCCESSES_PER_RATE_INCREASE`] consecutive
///   successes add one request per second back, up to the configured maximum.
///
/// Halving overshoots by design, so the steady-state rate is set by the increase
/// side: the limiter climbs until the server pushes back, halves, and climbs
/// again, oscillating around the capacity actually on offer.
///
/// The rate never drops below 1 req/s, so progress is always possible.
///
/// Cloning an [`Edgar`] shares one limiter, so every clone in a process
/// participates in the same control loop.
#[derive(Debug)]
pub(crate) struct AdaptiveLimiter {
    /// Ceiling from [`EdgarConfig::rate_limit`]; the rate never exceeds this.
    max_rate: u32,

    /// Rate currently being enforced, in requests per second.
    current_rate: AtomicU32,

    /// Successes observed since the last increase or decrease.
    consecutive_successes: AtomicU32,

    /// Rebuilt whenever `current_rate` changes. Behind a lock rather than
    /// swapped atomically because changes are rare — one per throttle event or
    /// per [`SUCCESSES_PER_RATE_INCREASE`] successes — while reads happen on
    /// every request.
    limiter: RwLock<Arc<Governor>>,

    /// When the rate was last reduced, for [`RATE_DECREASE_COOLDOWN`].
    last_decrease: RwLock<Instant>,
}

impl AdaptiveLimiter {
    fn governor_for(rate: u32) -> Arc<Governor> {
        // `rate` is clamped to >= 1 by every caller, so the NonZeroU32 is sound.
        let quota = Quota::per_second(NonZeroU32::new(rate.max(1)).expect("rate >= 1"));
        Arc::new(RateLimiter::direct(quota))
    }

    pub(crate) fn new(max_rate: u32) -> Self {
        Self {
            max_rate,
            current_rate: AtomicU32::new(max_rate),
            consecutive_successes: AtomicU32::new(0),
            limiter: RwLock::new(Self::governor_for(max_rate)),
            // Far enough in the past that the first push-back is acted on.
            last_decrease: RwLock::new(Instant::now() - RATE_DECREASE_COOLDOWN),
        }
    }

    /// Waits until the current rate allows another request.
    pub(crate) async fn until_ready(&self) {
        // The Arc is cloned out and the guard dropped before awaiting: holding a
        // std lock across an await point would make the future non-Send and
        // could deadlock against a concurrent rate change.
        let limiter = {
            let guard = self.limiter.read().expect("limiter lock poisoned");
            Arc::clone(&guard)
        };
        limiter.until_ready().await;
    }

    /// The rate currently being enforced, in requests per second.
    pub(crate) fn current_rate(&self) -> u32 {
        self.current_rate.load(Ordering::Relaxed)
    }

    fn set_rate(&self, rate: u32) {
        let rate = rate.clamp(1, self.max_rate);
        if rate == self.current_rate.swap(rate, Ordering::Relaxed) {
            return;
        }
        *self.limiter.write().expect("limiter lock poisoned") = Self::governor_for(rate);
    }

    /// Records server-side push-back (HTTP 429 or a retryable 5xx), halving the
    /// permitted rate at most once per [`RATE_DECREASE_COOLDOWN`].
    pub(crate) fn on_throttled(&self) {
        self.consecutive_successes.store(0, Ordering::Relaxed);

        // Collapse a burst of rejections from one congestion event into a single
        // halving. Checked and stamped under one write guard so concurrent
        // reporters cannot both pass the test.
        {
            let mut last = self.last_decrease.write().expect("cooldown lock poisoned");
            if last.elapsed() < RATE_DECREASE_COOLDOWN {
                return;
            }
            *last = Instant::now();
        }

        let current = self.current_rate.load(Ordering::Relaxed);
        let reduced = (current / 2).max(1);
        if reduced != current {
            tracing::warn!(
                "Rate limited by server — reducing request rate from {} to {} req/s",
                current,
                reduced
            );
        }
        self.set_rate(reduced);
    }

    /// Records a successful request, easing the rate back up over time.
    pub(crate) fn on_success(&self) {
        let current = self.current_rate.load(Ordering::Relaxed);
        if current >= self.max_rate {
            return;
        }
        let successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
        if successes >= SUCCESSES_PER_RATE_INCREASE {
            self.consecutive_successes.store(0, Ordering::Relaxed);
            self.set_rate(current + 1);
            tracing::debug!("Recovered — raising request rate to {} req/s", current + 1);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Edgar {
    /// HTTP client for making requests
    pub(crate) client: reqwest::Client,

    /// Token bucket rate limiter for SEC compliance. Adapts downward when the
    /// server signals it is being overrun; see [`AdaptiveLimiter`].
    pub(crate) rate_limiter: Arc<AdaptiveLimiter>,

    /// Base URL for EDGAR archives
    pub(crate) edgar_archives_url: String,

    /// Base URL for EDGAR data API
    pub(crate) edgar_data_url: String,

    /// Base URL for EDGAR files
    pub(crate) edgar_files_url: String,

    /// Base URL for EDGAR search endpoint
    pub(crate) edgar_search_url: String,
}

/// HTTP client for accessing the SEC EDGAR API with built-in rate limiting and retry logic.
///
/// The `Edgar` client serves as the main entry point for interacting with the SEC's Electronic
/// Data Gathering, Analysis, and Retrieval (EDGAR) system. It provides a safe, compliant way to
/// access company filings, financial data, search capabilities, RSS feeds, and filing indices.
///
/// This client automatically handles SEC.gov's fair access requirements by implementing rate
/// limiting, respects server-side rate limit responses with exponential backoff, and includes
/// retry logic for transient network failures. All operations are async and designed to work
/// seamlessly with tokio or other async runtimes.
///
/// # Rate Limiting
///
/// The SEC requires that automated systems respect fair access guidelines, limiting requests to
/// no more than 10 per second. This client uses a token bucket algorithm to enforce this limit:
///
/// ```text
/// Token Bucket (capacity: 10 tokens)
/// ┌──────────────────────────┐
/// │ ████████████████████████ │  ← Tokens refill at 10/sec
/// └──────────────────────────┘
///      ↓ consume on request
/// ```
///
/// When the bucket is empty, requests automatically wait until tokens become available. This
/// ensures compliance without requiring manual throttling in your application code.
///
/// The configured rate is a **ceiling, not a promise**. When EDGAR pushes back — a
/// 429, or a 503 as it sheds load — the limiter halves its own rate, and it climbs
/// back one request per second at a time once requests start succeeding again.
/// This matters whenever more than one client shares the quota: several processes
/// each obeying 10 req/s locally still add up to more than EDGAR will accept, and
/// backoff alone cannot fix that because it reschedules a request without slowing
/// the stream behind it. [`Edgar::current_rate_limit`] reports the rate in force.
///
/// # Error Handling
///
/// The client gracefully handles various error conditions including network failures, rate limit
/// responses (HTTP 429), transient server errors (500, 502, 503, 504), resource not found
/// (HTTP 404), and invalid responses. Retryable errors trigger automatic retries with exponential
/// backoff and full jitter to prevent thundering herd issues. HTTP 403 and 404 are returned
/// immediately, since neither is changed by trying again.
///
/// # Examples
///
/// Basic client initialization:
///
/// ```rust
/// # use edgarkit::Edgar;
/// let edgar = Edgar::new("my_app/1.0 (my@email.com)")?;
/// # Ok::<(), edgarkit::EdgarError>(())
/// ```
///
/// With custom configuration:
///
/// ```rust
/// # use edgarkit::{Edgar, EdgarConfig, EdgarUrls};
/// # use std::time::Duration;
/// let config = EdgarConfig {
///     user_agent: "custom_app/2.0".to_string(),
///     rate_limit: 5,
///     timeout: Duration::from_secs(60),
///     base_urls: EdgarUrls::default(),
/// };
/// let edgar = Edgar::with_config(config)?;
/// # Ok::<(), edgarkit::EdgarError>(())
/// ```
impl Edgar {
    /// Creates a new Edgar client with sensible defaults for most use cases.
    ///
    /// This constructor initializes the client with a rate limit of 10 requests per second
    /// (as required by SEC.gov), a 30-second timeout for HTTP requests, and the standard
    /// SEC.gov base URLs. The user agent you provide will be sent with every request to
    /// identify your application to the SEC.
    ///
    /// # Arguments
    ///
    /// * `user_agent` - A descriptive identifier for your application, following the format
    ///   "AppName/Version (contact@email.com)". The SEC requires this to contact you if
    ///   your application causes issues. Be honest and provide valid contact information.
    ///
    /// # Returns
    ///
    /// Returns a configured `Edgar` client ready to make requests, or an error if the
    /// user agent string is invalid or the HTTP client cannot be constructed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use edgarkit::Edgar;
    /// let edgar = Edgar::new("my_app/1.0 (email@example.com)")?;
    /// ```
    pub fn new(user_agent: &str) -> Result<Self> {
        let config = EdgarConfig {
            user_agent: user_agent.to_string(),
            rate_limit: 10,
            timeout: Duration::from_secs(30),
            base_urls: EdgarUrls::default(),
        };
        Self::with_config(config)
    }

    /// Creates an Edgar client with custom configuration settings.
    ///
    /// Use this constructor when you need to customize the rate limit, timeout duration,
    /// or base URLs. This is useful for testing with mock servers, adjusting performance
    /// characteristics for your use case, or complying with different rate limit policies.
    ///
    /// # Arguments
    ///
    /// * `config` - An `EdgarConfig` struct containing your custom settings including user
    ///   agent, rate limit (requests per second), HTTP timeout, and base URLs for the
    ///   various EDGAR services.
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::ConfigError` if the user agent is malformed, the rate limit
    /// is zero, or the HTTP client cannot be built with the provided configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use edgarkit::{Edgar, EdgarConfig, EdgarUrls};
    /// use std::time::Duration;
    ///
    /// let config = EdgarConfig {
    ///     user_agent: "research_tool/1.0".to_string(),
    ///     rate_limit: 5,  // More conservative rate
    ///     timeout: Duration::from_secs(60),
    ///     base_urls: EdgarUrls::default(),
    /// };
    /// let edgar = Edgar::with_config(config)?;
    /// ```
    pub fn with_config(config: EdgarConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&config.user_agent)
                .map_err(|e| EdgarError::ConfigError(format!("Invalid user agent: {}", e)))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            .build()
            .map_err(|e| EdgarError::ConfigError(format!("Failed to build HTTP client: {}", e)))?;

        if config.rate_limit == 0 {
            return Err(EdgarError::ConfigError(
                "Rate limit must be greater than zero".to_string(),
            ));
        }
        let rate_limiter = Arc::new(AdaptiveLimiter::new(config.rate_limit));

        Ok(Edgar {
            client,
            rate_limiter,
            edgar_archives_url: config.base_urls.archives,
            edgar_data_url: config.base_urls.data,
            edgar_files_url: config.base_urls.files,
            edgar_search_url: config.base_urls.search,
        })
    }

    /// The request rate currently being enforced, in requests per second.
    ///
    /// Starts at [`EdgarConfig::rate_limit`] and moves with server push-back: a
    /// 429 or a retryable 5xx halves it, sustained success raises it back toward
    /// the configured ceiling. Useful for logging how hard EDGAR is pushing back
    /// during a large ingestion run.
    ///
    /// # Example
    ///
    /// ```
    /// # use edgarkit::Edgar;
    /// let edgar = Edgar::new("MyApp contact@example.com").unwrap();
    /// assert_eq!(edgar.current_rate_limit(), 10); // the default ceiling
    /// ```
    pub fn current_rate_limit(&self) -> u32 {
        self.rate_limiter.current_rate()
    }

    /// Calculates the wait duration for retry attempts using exponential backoff
    /// with **full jitter**.
    ///
    /// The ceiling doubles per attempt — 1s, 2s, 4s, 8s, 16s — and the actual wait
    /// is drawn uniformly from `[0, ceiling]`.
    ///
    /// Full jitter rather than a narrow band around the ceiling: the point of the
    /// jitter is to decorrelate clients that were all rejected at the same instant,
    /// and a ±20 % band leaves them retrying inside a window narrow enough to
    /// collide again. Spreading over the whole interval is what actually breaks up
    /// the herd, at the cost of a shorter average wait — which the adaptive rate
    /// limiter compensates for by lowering the request rate itself.
    ///
    /// # Arguments
    ///
    /// * `retry` - The retry attempt number (0-indexed, so first retry is 0)
    ///
    /// # Returns
    ///
    /// A `Duration` indicating how long to wait before the next retry attempt.
    fn calculate_backoff(retry: u32) -> Duration {
        let ceiling_ms = INITIAL_BACKOFF_MS.saturating_mul(2_u64.saturating_pow(retry));
        Duration::from_millis(fastrand::u64(0..=ceiling_ms))
    }

    /// Returns `true` for status codes worth retrying: transient server-side
    /// failures that carry no information about the request itself.
    ///
    /// SEC EDGAR sheds load with 503 far more often than it does with 429 — in one
    /// production run 503s outnumbered exhausted rate limits almost two to one —
    /// and treating them as fatal discarded filings that a second attempt would
    /// have fetched. 500, 502 and 504 are included on the same reasoning.
    ///
    /// Client errors are deliberately excluded: 403 means the User-Agent was
    /// rejected and 404 means the document is not there, neither of which a retry
    /// changes.
    fn is_retryable_server_error(status: reqwest::StatusCode) -> bool {
        matches!(status.as_u16(), 500 | 502 | 503 | 504)
    }

    /// Fetches binary data from a URL with automatic rate limiting and retry logic.
    ///
    /// This method is designed for downloading binary files like zip archives or PDF documents
    /// from the SEC EDGAR system. It respects rate limits, automatically retries on transient
    /// failures and rate limit responses (HTTP 429), and returns the raw bytes for further
    /// processing by your application.
    ///
    /// The method will retry up to 5 times for rate limit errors (429) or network failures,
    /// using exponential backoff with jitter between attempts. Other HTTP errors like 404
    /// or 403 are returned immediately without retry.
    ///
    /// # Arguments
    ///
    /// * `url` - The fully-qualified URL to fetch
    ///
    /// # Returns
    ///
    /// Returns a `Vec<u8>` containing the response body, or an `EdgarError` if the request
    /// fails after all retries or encounters a non-retryable error.
    ///
    /// # Errors
    ///
    /// * `EdgarError::NotFound` - The resource doesn't exist (HTTP 404)
    /// * `EdgarError::RateLimitExceeded` - Rate limit responses persisted after max retries
    /// * `EdgarError::RequestError` - Network failure or other HTTP errors
    /// * `EdgarError::InvalidResponse` - Unexpected HTTP status code
    pub async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut retries = 0;

        loop {
            self.rate_limiter.until_ready().await;

            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(EdgarError::RequestError)?;

            match response.status() {
                reqwest::StatusCode::OK => {
                    self.rate_limiter.on_success();
                    return response
                        .bytes()
                        .await
                        .map(|b| b.to_vec())
                        .map_err(EdgarError::RequestError);
                }
                reqwest::StatusCode::NOT_FOUND => {
                    return Err(EdgarError::NotFound);
                }
                reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    // Slow the limiter before sleeping. Waiting alone leaves the
                    // request rate unchanged, so the retry lands in the same
                    // congestion that produced this response.
                    self.rate_limiter.on_throttled();
                    if retries >= MAX_RETRIES {
                        return Err(EdgarError::RateLimitExceeded);
                    }
                    let retry_after = Self::calculate_backoff(retries);
                    sleep(retry_after).await;
                    retries += 1;
                    continue;
                }
                status if Self::is_retryable_server_error(status) => {
                    if retries >= MAX_RETRIES {
                        return Err(EdgarError::InvalidResponse(format!(
                            "Unexpected status code: {} after {} retries",
                            status, MAX_RETRIES
                        )));
                    }
                    // A 503 is EDGAR shedding load, so it is push-back like a 429
                    // and the rate comes down the same way.
                    self.rate_limiter.on_throttled();
                    let retry_after = Self::calculate_backoff(retries);
                    tracing::warn!(
                        "Server error ({}) for {}. Attempt {}/{}. Waiting for {:?} before retry.",
                        status,
                        url,
                        retries + 1,
                        MAX_RETRIES + 1,
                        retry_after
                    );
                    sleep(retry_after).await;
                    retries += 1;
                    continue;
                }
                status => {
                    return Err(EdgarError::InvalidResponse(format!(
                        "Unexpected status code: {}",
                        status
                    )));
                }
            }
        }
    }

    /// Fetches text content from a URL with rate limiting, retries, and content-type validation.
    ///
    /// This is the primary method for retrieving text-based resources from the SEC EDGAR system,
    /// including JSON data, HTML filings, and XML feeds. It automatically enforces rate limits,
    /// retries failed requests with exponential backoff, and validates content types for JSON
    /// endpoints to catch server errors early.
    ///
    /// # Content-Type Validation
    ///
    /// For URLs ending in `.json`, the method validates that the response isn't HTML (which
    /// typically indicates an error page). The SEC sometimes returns JSON with a `text/html`
    /// content-type header, so the method also checks if the body looks like JSON. If it's
    /// actually HTML, an `UnexpectedContentType` error is returned with a preview of the
    /// content for debugging.
    ///
    /// # Retry Behavior
    ///
    /// - **Rate limits (429)**: Retries up to 5 times, respecting `Retry-After` headers when
    ///   present, otherwise using exponential backoff
    /// - **Network errors**: Retries up to 5 times with exponential backoff  
    /// - **Other HTTP errors**: No retry, returns immediately
    /// - **Content-type mismatches**: No retry, returns immediately
    ///
    /// # Arguments
    ///
    /// * `url` - The fully-qualified URL to fetch
    ///
    /// # Returns
    ///
    /// Returns the response body as a `String`, or an error if the request fails.
    ///
    /// # Errors
    ///
    /// * `EdgarError::UnexpectedContentType` - JSON URL returned HTML content
    /// * `EdgarError::NotFound` - Resource doesn't exist (HTTP 404)
    /// * `EdgarError::RateLimitExceeded` - Max retries exhausted for rate limits
    /// * `EdgarError::RequestError` - Network or HTTP errors
    /// * `EdgarError::InvalidResponse` - Unexpected status codes with content preview
    pub async fn get(&self, url: &str) -> Result<String> {
        self.fetch(url).await
    }

    /// Executes an HTTP GET request against the SEC EDGAR API with rate limiting, retries,
    /// and content-type validation.
    ///
    /// This is the single choke point through which all outbound requests flow. It:
    /// - Waits for a token from the rate limiter before every attempt (≤ 10 req/s).
    /// - For `.json` URLs, guards against SEC occasionally returning an HTML error page
    ///   with a `text/html` content-type; if the body still parses as JSON it is accepted,
    ///   otherwise [`EdgarError::UnexpectedContentType`] is returned.
    /// - Retries on HTTP 429 (`Too Many Requests`) up to [`MAX_RETRIES`] times, honouring
    ///   the `Retry-After` header when present and falling back to exponential backoff.
    /// - Maps `404` to [`EdgarError::NotFound`] and any other non-200 status to
    ///   [`EdgarError::InvalidResponse`] with a body preview for easier debugging.
    async fn fetch(&self, url: &str) -> Result<String> {
        let mut retries = 0;

        loop {
            // Wait for rate limiter
            self.rate_limiter.until_ready().await;

            let response_result = self.client.get(url).send().await;

            match response_result {
                Ok(response) => {
                    let status = response.status();
                    let headers = response.headers().clone();

                    // **Primary Check: If JSON was expected but HTML is received (regardless of status for client/server errors)**
                    if url.ends_with(".json") && status.is_success() {
                        if let Some(ct) = headers
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|val| val.to_str().ok())
                        {
                            if ct.to_lowercase().contains("text/html") {
                                // SEC sometimes returns JSON with text/html content-type
                                // Try to get the body and check if it's actually JSON
                                let body_text = response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Failed to read response body".to_string());

                                // Try to parse as JSON - if successful, it's valid JSON despite wrong content-type
                                if body_text.trim_start().starts_with('{')
                                    || body_text.trim_start().starts_with('[')
                                {
                                    tracing::warn!(
                                        "Received text/html content-type for .json URL, but content appears to be JSON: {}",
                                        url
                                    );
                                    return Ok(body_text);
                                }

                                // If it's actually HTML, return error
                                let body_preview = body_text.chars().take(200).collect::<String>();
                                return Err(EdgarError::UnexpectedContentType {
                                    url: url.to_string(),
                                    expected_pattern: "application/json".to_string(),
                                    got_content_type: ct.to_string(),
                                    content_preview: body_preview,
                                });
                            }
                        }
                        // If content-type wasn't text/html, or header was missing, proceed to normal status handling.
                        // This means if it's a non-200 status but the content might be a valid JSON error (e.g., from SEC API),
                        // it will be handled by the match status block below.
                    }

                    // **Standard Status Handling**
                    match status {
                        reqwest::StatusCode::OK => {
                            // If it's a .json URL, the check above ensures Content-Type wasn't text/html.
                            // If it's not a .json URL, we just get the text.
                            self.rate_limiter.on_success();
                            return response.text().await.map_err(EdgarError::RequestError);
                        }
                        reqwest::StatusCode::NOT_FOUND => {
                            return Err(EdgarError::NotFound);
                        }
                        reqwest::StatusCode::TOO_MANY_REQUESTS => {
                            // Slow the limiter before sleeping — see `get_bytes`.
                            self.rate_limiter.on_throttled();
                            if retries >= MAX_RETRIES {
                                return Err(EdgarError::RateLimitExceeded);
                            }

                            // Get retry-after header if available
                            let retry_after_duration = headers
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .map(Duration::from_secs)
                                .unwrap_or_else(|| Self::calculate_backoff(retries));

                            tracing::warn!(
                                "Rate limit hit (429) for {}. Attempt {}/{}. Waiting for {:?} before retry.",
                                url,
                                retries + 1,
                                MAX_RETRIES + 1, // Display as 1/6, 2/6, ..., 6/6 for MAX_RETRIES = 5
                                retry_after_duration
                            );
                            sleep(retry_after_duration).await;
                            retries += 1;
                            continue; // Retry the loop
                        }
                        other_status if Self::is_retryable_server_error(other_status) => {
                            if retries >= MAX_RETRIES {
                                let error_body = response
                                    .text()
                                    .await
                                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                                return Err(EdgarError::InvalidResponse(format!(
                                    "Unexpected status code: {} for URL: {} after {} retries. Response preview: {}",
                                    other_status,
                                    url,
                                    MAX_RETRIES,
                                    error_body.chars().take(200).collect::<String>()
                                )));
                            }
                            // EDGAR sheds load with 503 far more often than 429, so
                            // this is push-back too and the rate comes down for it.
                            self.rate_limiter.on_throttled();
                            let retry_after_duration = headers
                                .get("retry-after")
                                .and_then(|h| h.to_str().ok())
                                .and_then(|s| s.parse::<u64>().ok())
                                .map(Duration::from_secs)
                                .unwrap_or_else(|| Self::calculate_backoff(retries));

                            tracing::warn!(
                                "Server error ({}) for {}. Attempt {}/{}. Waiting for {:?} before retry.",
                                other_status,
                                url,
                                retries + 1,
                                MAX_RETRIES + 1,
                                retry_after_duration
                            );
                            sleep(retry_after_duration).await;
                            retries += 1;
                            continue;
                        }
                        other_status => {
                            // Handles other client errors like 403.
                            // If we reached here for a .json URL, it means the Content-Type wasn't text/html (or was missing).
                            // The body might be a JSON-formatted error from SEC, or some other non-HTML error page.
                            let error_body = response
                                .text()
                                .await
                                .unwrap_or_else(|_| "Failed to read error body".to_string());

                            return Err(EdgarError::InvalidResponse(format!(
                                "Unexpected status code: {} for URL: {}. Response preview: {}",
                                other_status,
                                url,
                                error_body.chars().take(200).collect::<String>()
                            )));
                        }
                    }
                }
                Err(e) => {
                    // Network or other reqwest error before getting a response status
                    if retries >= MAX_RETRIES {
                        return Err(EdgarError::RequestError(e));
                    }
                    let backoff_duration = Self::calculate_backoff(retries);
                    tracing::warn!(
                        "Request failed for {}: {:?}. Attempt {}/{}. Retrying in {:?}.",
                        url,
                        e,
                        retries + 1,
                        MAX_RETRIES + 1, // Display as 1/6, 2/6, ..., 6/6 for MAX_RETRIES = 5
                        backoff_duration
                    );
                    sleep(backoff_duration).await;
                    retries += 1;
                    continue; // Retry the loop
                }
            }
        }
    }

    /// Returns the base URL for EDGAR archives.
    ///
    /// # Returns
    ///
    /// A string slice containing the base URL for accessing EDGAR archive endpoints.
    pub fn archives_url(&self) -> &str {
        &self.edgar_archives_url
    }

    /// Returns the base URL for EDGAR data.
    ///
    /// # Returns
    ///
    /// A string slice containing the base URL for accessing EDGAR data endpoints.
    pub fn data_url(&self) -> &str {
        &self.edgar_data_url
    }

    /// Returns the base URL for EDGAR files.
    ///
    /// # Returns
    ///
    /// A string slice containing the base URL for accessing EDGAR file endpoints.
    pub fn files_url(&self) -> &str {
        &self.edgar_files_url
    }

    /// Returns the base URL for EDGAR search.
    ///
    /// # Returns
    ///
    /// A string slice containing the base URL for accessing EDGAR search endpoints.
    pub fn search_url(&self) -> &str {
        &self.edgar_search_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_backoff() {
        // Full jitter: each wait is drawn from [0, ceiling], and the ceiling
        // doubles per attempt. Individual samples are therefore *not* ordered —
        // attempt 2 can legitimately return less than attempt 0 — so the ceiling
        // is what gets asserted, over enough samples to pin it down.
        for (retry, ceiling_ms) in [(0u32, 1_000u128), (1, 2_000), (2, 4_000)] {
            let samples: Vec<u128> = (0..500)
                .map(|_| Edgar::calculate_backoff(retry).as_millis())
                .collect();

            assert!(
                samples.iter().all(|&ms| ms <= ceiling_ms),
                "attempt {retry} exceeded its {ceiling_ms}ms ceiling"
            );
            // The whole interval must be in play, or the jitter is not full.
            let max = *samples.iter().max().expect("samples");
            let min = *samples.iter().min().expect("samples");
            assert!(
                max > ceiling_ms / 2,
                "attempt {retry} never sampled the upper half of [0, {ceiling_ms}]"
            );
            assert!(
                min < ceiling_ms / 2,
                "attempt {retry} never sampled the lower half of [0, {ceiling_ms}]"
            );
        }
    }

    #[test]
    fn an_adaptive_limiter_halves_on_push_back_and_climbs_back_on_success() {
        let limiter = AdaptiveLimiter::new(16);
        assert_eq!(limiter.current_rate(), 16);

        limiter.on_throttled();
        assert_eq!(limiter.current_rate(), 8, "a 429 halves the rate");

        // A burst from the same congestion event must not halve repeatedly.
        limiter.on_throttled();
        limiter.on_throttled();
        assert_eq!(
            limiter.current_rate(),
            8,
            "reductions collapse to one per cooldown window"
        );

        for _ in 0..SUCCESSES_PER_RATE_INCREASE {
            limiter.on_success();
        }
        assert_eq!(limiter.current_rate(), 9, "sustained success adds one back");
    }

    #[test]
    fn an_adaptive_limiter_never_drops_below_one_or_exceeds_its_ceiling() {
        let limiter = AdaptiveLimiter::new(4);

        for _ in 0..20 {
            // Bypass the cooldown so the descent is actually exercised.
            *limiter.last_decrease.write().unwrap() = Instant::now() - RATE_DECREASE_COOLDOWN * 2;
            limiter.on_throttled();
        }
        assert_eq!(limiter.current_rate(), 1, "the floor is one request/second");

        for _ in 0..(SUCCESSES_PER_RATE_INCREASE * 50) {
            limiter.on_success();
        }
        assert_eq!(
            limiter.current_rate(),
            4,
            "recovery stops at the configured ceiling"
        );
    }
}
