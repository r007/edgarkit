//! Configuration types for customizing Edgar client behavior.
//!
//! The configuration system allows you to control rate limiting, HTTP timeouts,
//! base URLs, and user agent strings. Most users can rely on the defaults provided
//! by `Edgar::new()`, but custom configurations are useful for testing, research
//! applications with specific performance requirements, or compliance scenarios.

use std::time::Duration;

/// Configuration settings for the Edgar HTTP client.
///
/// This struct contains all the settings needed to customize how the Edgar client
/// behaves, including network timeouts, rate limiting, and service endpoints. The
/// default configuration is optimized for general use and SEC.gov compliance, but
/// you can adjust these settings based on your application's needs.
///
/// # Examples
///
/// Using defaults:
/// ```rust
/// # use edgarkit::EdgarConfig;
/// let config = EdgarConfig::default();
/// ```
///
/// Custom configuration:
/// ```rust
/// # use edgarkit::{EdgarConfig, EdgarUrls};
/// # use std::time::Duration;
/// let config = EdgarConfig::new(
///     "research_app/1.0 contact@university.edu",
///     5,  // More conservative rate
///     Duration::from_secs(45),
///     None,  // Use default URLs
/// );
/// ```
#[derive(Debug, Clone)]
pub struct EdgarConfig {
    /// User agent string for HTTP requests (required by SEC)
    pub user_agent: String,

    /// Rate limit in requests per second (default: 10)
    pub rate_limit: u32,

    /// HTTP request timeout duration
    pub timeout: Duration,

    /// Base URLs for different EDGAR services
    pub base_urls: EdgarUrls,
}

/// Base URLs for the different SEC EDGAR service endpoints.
///
/// The SEC EDGAR system is distributed across multiple domains, each serving
/// different types of content. The archives domain hosts historical filings,
/// the data domain provides structured API access, and the files domain serves
/// various data files. You typically won't need to change these unless you're
/// running tests against a mock server.
#[derive(Debug, Clone)]
pub struct EdgarUrls {
    /// Archives base URL (historical filings)
    pub archives: String,

    /// Data API base URL (structured data)
    pub data: String,

    /// Files base URL (company tickers, etc.)
    pub files: String,

    /// Search API base URL
    pub search: String,
}

impl Default for EdgarConfig {
    fn default() -> Self {
        Self {
            user_agent: "edgarkit/0.1.0".to_string(),
            rate_limit: 10,
            timeout: Duration::from_secs(30),
            base_urls: EdgarUrls {
                archives: "https://www.sec.gov/Archives/edgar".to_string(),
                data: "https://data.sec.gov".to_string(),
                files: "https://www.sec.gov/files".to_string(),
                search: "https://efts.sec.gov/LATEST/search-index/".to_string(),
            },
        }
    }
}

impl EdgarConfig {
    /// Creates custom Edgar configuration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use edgarkit::{EdgarConfig, EdgarUrls};
    /// use std::time::Duration;
    ///
    /// let config = EdgarConfig::new(
    ///     "MyApp contact@example.com",
    ///     10,
    ///     Duration::from_secs(30),
    ///     None,
    /// );
    /// ```
    pub fn new(
        user_agent: impl Into<String>,
        rate_limit: u32,
        timeout: Duration,
        base_urls: Option<EdgarUrls>,
    ) -> Self {
        Self {
            user_agent: user_agent.into(),
            rate_limit,
            timeout,
            base_urls: base_urls.unwrap_or_default(),
        }
    }
}

impl Default for EdgarUrls {
    fn default() -> Self {
        Self {
            archives: "https://www.sec.gov/Archives/edgar".to_string(),
            data: "https://data.sec.gov".to_string(),
            files: "https://www.sec.gov/files".to_string(),
            search: "https://efts.sec.gov/LATEST/search-index/".to_string(),
        }
    }
}
