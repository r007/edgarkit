use std::time::Duration;

/// Configuration for the Edgar client
#[derive(Debug, Clone)]
pub struct EdgarConfig {
    /// User agent string for HTTP requests
    pub user_agent: String,
    /// Rate limit in requests per second
    pub rate_limit: u32,
    /// HTTP request timeout
    pub timeout: Duration,
    /// Base URLs for different EDGAR services
    pub base_urls: EdgarUrls,
}

/// Base URLs for different EDGAR services
#[derive(Debug, Clone)]
pub struct EdgarUrls {
    /// Base URL for EDGAR archives
    pub archives: String,
    /// Base URL for EDGAR data
    pub data: String,
    /// Base URL for EDGAR files
    pub files: String,
    /// Base URL for EDGAR search
    pub search: String,
}

impl Default for EdgarConfig {
    fn default() -> Self {
        Self {
            user_agent: "edgar_client/0.1.0".to_string(),
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
    /// Creates a new EdgarConfig with custom settings
    ///
    /// # Basic usage
    ///
    /// ```rust
    /// use edgar_client::{Edgar, EdgarConfig, EdgarUrls};
    /// use std::time::Duration;
    /// let config = EdgarConfig {
    ///    user_agent: "YourAppName contact@example.com".to_string(),
    ///    rate_limit: 10, // requests per second
    ///    timeout: Duration::from_secs(30),
    ///    base_urls: EdgarUrls::default(),
    /// };
    /// let edgar = Edgar::with_config(config)?;
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
