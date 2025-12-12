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
    pub(crate) client: reqwest::Client,
    pub(crate) rate_limiter: Arc<Governor>,
    pub(crate) edgar_archives_url: String,
    pub(crate) edgar_data_url: String,
    pub(crate) edgar_files_url: String,
    pub(crate) edgar_search_url: String,
}

/// A client for interacting with the SEC EDGAR API that handles rate limiting and HTTP requests.
///
/// The `Edgar` client provides methods to interact with different parts of the SEC EDGAR system,
/// including archives, data, and files. It implements rate limiting and automatic retries with exponential
/// backoff to comply with SEC's access requirements and handles HTTP requests with appropriate error handling.
///
/// # Rate Limiting
///
/// The client implements a token bucket algorithm for rate limiting requests and automatic retries with
/// exponential backoff to comply with SEC's guidelines. By default, it's set to 10 requests per second
/// but can be customized through the configuration.
///
/// # Configuration
///
/// The client can be configured with:
/// - Custom user agent string (required by SEC)
/// - Rate limiting parameters
/// - Request timeout settings
/// - Custom base URLs for different EDGAR services
///
/// # Error Handling
///
/// The client handles various HTTP status codes and network errors, including:
/// - 404 Not Found
/// - 429 Too Many Requests
/// - Network timeouts and connection errors
///
/// # Examples
///
/// Basic usage with default configuration:
/// ```rust
/// let edgar = Edgar::new("my_app/1.0 (my@email.com)").expect("Failed to create EDGAR client");
/// ```
///
/// Custom configuration:
/// ```rust
/// let config = EdgarConfig {
///     user_agent: "custom_app/2.0".to_string(),
///     rate_limit: 5,
///     timeout: Duration::from_secs(60),
///     base_urls: EdgarUrls::default(),
/// };
/// let edgar = Edgar::with_config(config).expect("Failed to create EDGAR client");
/// ```
///
/// # Errors
///
/// Operations can fail with `EdgarError` for various reasons:
/// - Configuration errors (invalid user agent, rate limit)
/// - Network request failures
/// - Rate limit exceeded
/// - Invalid responses from the EDGAR service
impl Edgar {
    /// Creates a new instance of the Edgar client with default configuration.
    ///
    /// # Arguments
    ///
    /// * `user_agent` - A string representing the user agent to be used in the HTTP requests.
    ///                  Should follow SEC's format requirements: "name+email"
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - A `Result` containing the new `Edgar` instance if successful,
    ///                    or an `EdgarError` if an error occurs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use etl::src::edgar::Edgar;
    ///
    /// let edgar = Edgar::new("my_app/1.0 (<EMAIL>)");
    /// assert!(edgar.is_ok());
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

    /// Creates a new instance of the Edgar client with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Custom configuration for the Edgar client including user agent,
    ///             rate limits, timeout, and base URLs.
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - A `Result` containing the configured Edgar instance or an error.
    ///
    /// # Errors
    ///
    /// Returns `EdgarError::ConfigError` if:
    /// - The user agent is invalid
    /// - The HTTP client fails to build
    /// - The rate limit is zero
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

    /// Calculates the exponential backoff duration for retrying requests.
    ///
    /// # Arguments
    ///
    /// * `retry` - The current retry attempt number
    ///
    /// # Returns
    ///
    /// A `Duration` representing the time to wait before the next retry,
    /// including a random jitter of ±20% to prevent thundering herd problems.
    fn calculate_backoff(retry: u32) -> Duration {
        let backoff_ms = INITIAL_BACKOFF_MS * (2_u64.pow(retry));
        // Add some jitter (±20% of the calculated backoff)
        let jitter = (backoff_ms as f64 * 0.2 * (fastrand::f64() - 0.5)) as i64;
        Duration::from_millis((backoff_ms as i64 + jitter) as u64)
    }

    /// Sends a GET request to the specified URL with rate limiting and retry logic for retrieving bytes.
    ///
    /// # Parameters
    ///
    /// * `url` - A string slice representing the URL to send the GET request to.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>>` - On success, returns a `Result` containing a vector of bytes representing the response body.
    ///   On failure, returns an `EdgarError` indicating the type of error that occurred.
    ///
    /// # Errors
    ///
    /// Returns various `EdgarError` variants depending on the failure:
    /// - `RequestError` for network/HTTP errors
    /// - `NotFound` for 404 responses
    /// - `RateLimitExceeded` after maximum retries
    /// - `InvalidResponse` for unexpected status codes
    ///
    /// # Rate Limiting
    ///
    /// Implements a token bucket algorithm for rate limiting and exponential backoff with jitter for rate limit responses (HTTP 429).
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

    /// Sends a GET request to the specified URL with rate limiting and retry logic.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to send the GET request to
    ///
    /// # Returns
    ///
    /// * `Result<String>` - The response body as a string if successful
    ///
    /// # Errors
    ///
    /// Returns various `EdgarError` variants depending on the failure:
    /// - `RequestError` for network/HTTP errors
    /// - `NotFound` for 404 responses
    /// - `RateLimitExceeded` after maximum retries for rate limiting
    /// - `InvalidResponse` for unexpected status codes with response preview
    /// - `UnexpectedContentType` when a JSON endpoint returns HTML content
    ///
    /// # Content Type Validation
    ///
    /// For URLs ending with ".json", this method validates that the response
    /// Content-Type is not "text/html". If HTML is detected (regardless of HTTP
    /// status code), it returns `UnexpectedContentType` with a preview of the
    /// HTML content. This prevents downstream JSON parsing errors when servers
    /// return HTML error pages instead of expected JSON responses.
    ///
    /// # Rate Limiting
    ///
    /// Implements a token bucket algorithm for rate limiting and exponential
    /// backoff with jitter for rate limit responses (HTTP 429). The method
    /// respects "retry-after" headers when present, falling back to calculated
    /// exponential backoff otherwise.
    ///
    /// # Retry Logic
    ///
    /// - Rate limit errors (429): Retries up to `MAX_RETRIES` times with backoff
    /// - Network errors: Retries up to `MAX_RETRIES` times with backoff
    /// - Other HTTP errors: No retry, immediate error return
    /// - Content type mismatches: No retry, immediate error return
    ///
    /// # Logging
    ///
    /// Logs warnings for:
    /// - Rate limit retries with attempt count and wait duration
    /// - Network error retries with error details and attempt count
    /// - Content type mismatches with response preview
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

    #[tokio::test]
    async fn test_rate_limiting_and_backoff() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let url = "https://www.sec.gov/files/company_tickers.json";

        // Make multiple requests in quick succession
        for i in 0..15 {
            let result = edgar.get(url).await;
            match result {
                Ok(_) => println!("Request {} succeeded", i),
                Err(EdgarError::RateLimitExceeded) => {
                    println!("Rate limit exceeded on request {}", i);
                    // Should only happen after MAX_RETRIES attempts
                    assert!(i > 5);
                    break;
                }
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    }
}
