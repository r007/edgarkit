use governor::{
    Quota, RateLimiter, clock::DefaultClock, middleware::NoOpMiddleware, state::InMemoryState,
    state::NotKeyed,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use super::config::{EdgarConfig, EdgarUrls};
use super::error::{EdgarError, Result};

const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 1000; // 1 second

type Governor = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

#[derive(Debug, Clone)]
pub struct Edgar {
    /// HTTP client for making requests
    pub(crate) client: reqwest::Client,

    /// Token bucket rate limiter for SEC compliance
    pub(crate) rate_limiter: Arc<Governor>,

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
/// # Error Handling
///
/// The client gracefully handles various error conditions including network failures, rate limit
/// responses (HTTP 429), resource not found (HTTP 404), and invalid responses. Transient errors
/// trigger automatic retries with exponential backoff and jitter to prevent thundering herd issues.
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

        let rate_limiter = Arc::new(RateLimiter::direct(Quota::per_second(
            NonZeroU32::new(config.rate_limit).ok_or_else(|| {
                EdgarError::ConfigError("Rate limit must be greater than zero".to_string())
            })?,
        )));

        Ok(Edgar {
            client,
            rate_limiter,
            edgar_archives_url: config.base_urls.archives,
            edgar_data_url: config.base_urls.data,
            edgar_files_url: config.base_urls.files,
            edgar_search_url: config.base_urls.search,
        })
    }

    /// Calculates the wait duration for retry attempts using exponential backoff with jitter.
    ///
    /// This implements a standard exponential backoff strategy where each retry waits longer
    /// than the previous attempt: 1s, 2s, 4s, 8s, 16s. Random jitter (±20%) is added to
    /// prevent the "thundering herd" problem where many clients retry simultaneously and
    /// overwhelm the server again.
    ///
    /// The formula is: `(2^retry × 1000ms) ± 20%`
    ///
    /// # Arguments
    ///
    /// * `retry` - The retry attempt number (0-indexed, so first retry is 0)
    ///
    /// # Returns
    ///
    /// A `Duration` indicating how long to wait before the next retry attempt.
    fn calculate_backoff(retry: u32) -> Duration {
        let backoff_ms = INITIAL_BACKOFF_MS * (2_u64.pow(retry));
        // Add some jitter (±20% of the calculated backoff)
        let jitter = (backoff_ms as f64 * 0.2 * (fastrand::f64() - 0.5)) as i64;
        Duration::from_millis((backoff_ms as i64 + jitter) as u64)
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
                    if retries >= MAX_RETRIES {
                        return Err(EdgarError::RateLimitExceeded);
                    }
                    let retry_after = Self::calculate_backoff(retries);
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
                            return response.text().await.map_err(EdgarError::RequestError);
                        }
                        reqwest::StatusCode::NOT_FOUND => {
                            return Err(EdgarError::NotFound);
                        }
                        reqwest::StatusCode::TOO_MANY_REQUESTS => {
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
                        other_status => {
                            // Handles other statuses like 403, 500, 503 etc.
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
        let backoff0 = Edgar::calculate_backoff(0);
        let backoff1 = Edgar::calculate_backoff(1);
        let backoff2 = Edgar::calculate_backoff(2);

        // Check that backoff increases exponentially
        assert!(backoff0 < backoff1);
        assert!(backoff1 < backoff2);

        // Check that backoff is roughly within expected range
        assert!(backoff0.as_millis() >= 800 && backoff0.as_millis() <= 1200); // ±20% of 1000ms
        assert!(backoff1.as_millis() >= 1600 && backoff1.as_millis() <= 2400); // ±20% of 2000ms
        assert!(backoff2.as_millis() >= 3200 && backoff2.as_millis() <= 4800); // ±20% of 4000ms
    }
}
