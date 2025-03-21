//! Search functionality for SEC EDGAR system.
//!
//! This module provides structures and traits for searching SEC EDGAR filings
//! using various criteria like form types, dates, and keywords.
//!
//! # Examples
//!
//! ```rust
//! use edgar_client::{Edgar, SearchOperations, SearchOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let edgar = Edgar::new("your_app_name contact@example.com")?;
//!     
//!     // Search for recent 10-K filings
//!     let options = SearchOptions::new()
//!         .with_forms(vec!["10-K".to_string()])
//!         .with_count(10)
//!         .with_date_range("2024-01-01".to_string(), "2024-03-01".to_string());
//!     
//!     let results = edgar.search(options).await?;
//!     
//!     for hit in results.hits.hits {
//!         println!("Found: {} filed on {}", hit._source.form, hit._source.file_date);
//!     }
//!     Ok(())
//! }
//! ```

use super::Edgar;
use super::error::{EdgarError, Result};
use super::traits::SearchOperations;
use async_trait::async_trait;
use serde::Deserialize;

/// Response from the EDGAR search API
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    /// Time taken to execute the search in milliseconds
    pub took: u32,
    /// Whether the search timed out
    pub timed_out: bool,
    /// Information about the shards that processed the search
    pub _shards: Shards,
    /// Search results containing matched documents
    pub hits: Hits,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shards {
    pub total: u32,
    pub successful: u32,
    pub skipped: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hits {
    pub total: TotalHits,
    #[serde(default)]
    pub max_score: Option<f64>,
    pub hits: Vec<Hit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TotalHits {
    pub value: u32,
    pub relation: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Hit {
    pub _index: String,
    pub _id: String,
    #[serde(default)]
    pub _score: Option<f64>,
    pub _source: Source,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    pub ciks: Vec<String>,
    #[serde(default)]
    pub period_ending: Option<String>,
    pub file_num: Option<Vec<String>>,
    pub display_names: Vec<String>,
    #[serde(default)]
    pub xsl: Option<String>,
    #[serde(deserialize_with = "deserialize_sequence")]
    pub sequence: u32,
    pub root_forms: Vec<String>,
    pub file_date: String,
    pub biz_states: Vec<String>,
    pub sics: Vec<String>,
    pub form: String,
    pub adsh: String,
    pub film_num: Vec<String>,
    pub biz_locations: Vec<String>,
    pub file_type: String,
    #[serde(default)]
    pub file_description: Option<String>,
    pub inc_states: Vec<String>,
    pub items: Option<Vec<String>>,
}

/// Options for configuring EDGAR searches
///
/// # Examples
///
/// Basic search for a company:
/// ```rust
/// let options = SearchOptions::new()
///     .with_query("Apple Inc")
///     .with_forms(vec!["10-K".to_string(), "10-Q".to_string()]);
/// ```
///
/// Search with date range and pagination:
/// ```rust
/// let options = SearchOptions::new()
///     .with_query("merger announcement")
///     .with_forms(vec!["8-K".to_string()])
///     .with_date_range("2024-01-01".to_string(), "2024-12-31".to_string())
///     .with_page(1)
///     .with_count(50);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub keys_typed: Option<String>,
    /// Search query. For details on special formatting, see the {@link https://www.sec.gov/edgar/search/efts-faq.html|FAQ}
    pub query: Option<String>,
    pub category: Option<String>,
    /// Filter based on company's location
    pub location_code: Option<String>,
    /// Company name OR individual's name. Cannot be combined with `cik` or `sik`..
    pub entity_name: Option<String>,
    /// Type of forms to search - e.g. '10-K'. Can also be an array of types - e.g. ["S-1", "10-K", "10-Q"]
    pub forms: Option<Vec<String>>,
    /// Filter based on company's location
    pub location_codes: Option<Vec<String>>,
    /// Which page of results to return
    pub page: Option<u32>,
    /// Skip a number of results
    pub from: Option<u32>,
    /// Number of results to return - will always try to return 100
    pub count: Option<u32>,
    /// If true, order by oldest first instead of newest first.
    pub reverse_order: Option<bool>,
    /// Start date. Must be in the form of `yyyy-mm-dd`. Must also specify `enddt`
    pub start_date: Option<String>,
    /// End date. Must be in the form of `yyyy-mm-dd`. Must also specify `startdt`
    pub end_date: Option<String>,
    /// Search by base words(default) or exactly as entered
    pub stemming: Option<String>,
    /// Company code(s) to search. Can be a single CIK or multiple CIKs as an array.
    /// Cannot be combined with `name` or `sic`
    pub ciks: Option<Vec<String>>,
    /// Standard Industrial Classification of filer
    /// Special options - 1: all, 0: Unspecified
    pub sic: Option<String>,
    /// Boolean to use location of incorporation rather than location of HQ
    pub incorporated_location: Option<bool>,
}

/// Custom deserializer for sequence field that can be either u32 or string
fn deserialize_sequence<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct SequenceVisitor;

    impl<'de> serde::de::Visitor<'de> for SequenceVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or a string containing an integer")
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as u32)
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            value.parse().map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_any(SequenceVisitor)
}

impl SearchOptions {
    /// Creates a new instance of SearchOptions with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the search query text
    ///
    /// # Example
    /// ```rust
    /// let options = SearchOptions::new()
    ///     .with_query("quarterly report");
    /// ```
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Sets the keys typed for typeahead search functionality
    pub fn with_keys_typed(mut self, keys: impl Into<String>) -> Self {
        self.keys_typed = Some(keys.into());
        self
    }

    /// Sets the category filter for the search
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Sets the location code filter
    pub fn with_location_code(mut self, code: impl Into<String>) -> Self {
        self.location_code = Some(code.into());
        self
    }

    /// Sets the entity name filter
    pub fn with_entity_name(mut self, name: impl Into<String>) -> Self {
        self.entity_name = Some(name.into());
        self
    }

    /// Sets the form types to filter by (e.g., ["10-K", "10-Q"])
    pub fn with_forms(mut self, forms: Vec<String>) -> Self {
        self.forms = Some(forms);
        self
    }

    /// Sets the location codes to filter by
    pub fn with_location_codes(mut self, codes: Vec<String>) -> Self {
        self.location_codes = Some(codes);
        self
    }

    /// Sets the page number for pagination (starting from 1)
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the starting index for results
    pub fn with_from(mut self, from: u32) -> Self {
        self.from = Some(from);
        self
    }

    /// Sets the maximum number of results to return
    pub fn with_count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    /// Sets whether to return results in reverse order
    pub fn with_reverse_order(mut self, reverse: bool) -> Self {
        self.reverse_order = Some(reverse);
        self
    }

    /// Sets the date range for the search
    ///
    /// # Arguments
    /// * `start_date` - Start date in YYYY-MM-DD format
    /// * `end_date` - End date in YYYY-MM-DD format
    pub fn with_date_range(mut self, start_date: String, end_date: String) -> Self {
        self.start_date = Some(start_date);
        self.end_date = Some(end_date);
        self
    }

    /// Sets stemming option for search
    pub fn with_stemming(mut self, stemming: impl Into<String>) -> Self {
        self.stemming = Some(stemming.into());
        self
    }

    /// Sets company CIK filter(s)
    ///
    /// # Arguments
    /// * `ciks` - A single CIK or multiple CIKs
    ///
    /// # Examples
    /// ```
    /// // Single CIK
    /// let options = SearchOptions::new().with_ciks("0001234567");
    ///
    /// // Multiple CIKs
    /// let options = SearchOptions::new().with_ciks(vec!["0001234567", "0007654321"]);
    /// ```
    pub fn with_ciks<T>(mut self, ciks: T) -> Self
    where
        T: Into<Vec<String>>,
    {
        self.ciks = Some(ciks.into());
        self
    }

    /// Sets a single company CIK filter
    ///
    /// This is a convenience method for backwards compatibility
    ///
    /// # Arguments
    /// * `cik` - A single CIK
    pub fn with_cik(self, cik: impl Into<String>) -> Self {
        self.with_ciks(vec![cik.into()])
    }

    /// Sets SIC code filter
    pub fn with_sic(mut self, sic: impl Into<String>) -> Self {
        self.sic = Some(sic.into());
        self
    }

    /// Sets whether to use incorporation location instead of HQ
    pub fn with_incorporated_location(mut self, incorporated: bool) -> Self {
        self.incorporated_location = Some(incorporated);
        self
    }

    pub fn to_query_params(&self) -> Vec<(String, String)> {
        let mut params = Vec::new();

        if let Some(ref query) = self.query {
            params.push(("q".to_string(), query.clone()));
        }

        if let Some(ref keys) = self.keys_typed {
            params.push(("keysTyped".to_string(), keys.clone()));
        }

        if let Some(ref category) = self.category {
            params.push(("category".to_string(), category.clone()));
        }

        if let Some(ref code) = self.location_code {
            params.push(("locationCode".to_string(), code.clone()));
        }

        if let Some(ref name) = self.entity_name {
            params.push(("entityName".to_string(), name.clone()));
        }

        if let Some(ref forms) = self.forms {
            params.push(("forms".to_string(), forms.join(",")));
        }

        if let Some(ref codes) = self.location_codes {
            params.push(("locationCodes".to_string(), codes.join(",")));
        }

        if let Some(page) = self.page {
            params.push(("page".to_string(), page.to_string()));
        }

        if let Some(from) = self.from {
            params.push(("from".to_string(), from.to_string()));
        }

        if let Some(count) = self.count {
            params.push(("count".to_string(), count.to_string()));
        }

        if let Some(reverse) = self.reverse_order {
            params.push((
                "reverse_order".to_string(),
                if reverse { "TRUE" } else { "FALSE" }.to_string(),
            ));
        }

        if let Some(ref start) = self.start_date {
            params.push(("startdt".to_string(), start.clone()));
        }

        if let Some(ref end) = self.end_date {
            params.push(("enddt".to_string(), end.clone()));
        }

        if let Some(ref stemming) = self.stemming {
            params.push(("stemming".to_string(), stemming.clone()));
        }

        if let Some(ref ciks) = self.ciks {
            params.push(("ciks".to_string(), ciks.join(",")));
        }

        if let Some(ref sic) = self.sic {
            params.push(("sic".to_string(), sic.clone()));
        }

        if let Some(incorporated) = self.incorporated_location {
            params.push((
                "incorporated_location".to_string(),
                incorporated.to_string(),
            ));
        }

        params
    }
}

/// Operations for searching SEC EDGAR filings
///
/// This trait provides methods for searching and retrieving filing documents from the SEC EDGAR system.
/// Supports both single-page searches and comprehensive multi-page searches with automatic pagination.
///
/// # Examples
///
/// Basic search for the most recent filings:
/// ```rust
/// use edgar_client::{Edgar, SearchOperations, SearchOptions};
///
/// async fn search_recent_filings() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("your_app_name contact@example.com")?;
///     
///     let options = SearchOptions::new()
///         .with_forms(vec!["10-K".to_string()])
///         .with_count(10);
///     
///     // Get just the first page
///     let first_page = edgar.search(options.clone()).await?;
///     
///     // Get all matching results across all pages
///     let all_results = edgar.search_all(options).await?;
///     
///     println!("Found {} total results", all_results.len());
///     Ok(())
/// }
/// ```
///
/// Advanced search with multiple criteria:
/// ```rust
/// async fn search_spac_filings() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("your_app_name contact@example.com")?;
///     
///     let options = SearchOptions::new()
///         .with_query("SPAC \"Rule 419\"")
///         .with_forms(vec!["S-1".to_string()])
///         .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());
///     
///     let all_results = edgar.search_all(options).await?;
///     
///     // Process all results
///     for hit in all_results {
///         println!("Found: {} filed on {}", hit._source.form, hit._source.file_date);
///     }
///     Ok(())
/// }
/// ```
#[async_trait]
impl SearchOperations for Edgar {
    /// Performs a single search query on EDGAR
    ///
    /// Returns results for a single page based on the provided options.
    /// Use `search_all` if you need all results across multiple pages.
    async fn search(&self, options: SearchOptions) -> Result<SearchResponse> {
        let params = options.to_query_params();
        let query_string = serde_urlencoded::to_string(&params)
            .map_err(|e| EdgarError::InvalidResponse(e.to_string()))?;

        let url = format!("{}?{}", self.search_url(), query_string);
        let response = self.get(&url).await?;

        Ok(serde_json::from_str(&response)?)
    }

    /// Performs a comprehensive search query and fetches all available results
    ///
    /// This method automatically handles pagination and fetches all available results
    /// matching the search criteria. Results are fetched in parallel batches for efficiency
    /// while respecting SEC's rate limits.
    ///
    /// # Arguments
    ///
    /// * `options` - Search criteria and filters
    ///
    /// # Returns
    ///
    /// Returns a vector containing all matching hits across all pages
    ///
    /// # Example
    ///
    /// ```rust
    /// let options = SearchOptions::new()
    ///     .with_query("quarterly report")
    ///     .with_forms(vec!["10-Q".to_string()])
    ///     .with_date_range("2024-01-01".to_string(), "2024-03-31".to_string());
    ///
    /// let all_results = edgar.search_all(options).await?;
    /// println!("Found {} quarterly reports", all_results.len());
    /// ```
    async fn search_all(&self, mut options: SearchOptions) -> Result<Vec<Hit>> {
        const BATCH_SIZE: u32 = 7; // Maximum number of concurrent requests
        const PAGE_SIZE: u32 = 100; // Results per page

        // Set defaults
        options.count = Some(PAGE_SIZE);
        options.page = Some(1);
        options.reverse_order = Some(false);

        let initial_response = self.search(options.clone()).await?;
        let total_hits = initial_response.hits.total.value;

        tracing::info!("Found {} total hits", total_hits);

        let mut all_hits = Vec::with_capacity(total_hits as usize);
        all_hits.extend(initial_response.hits.hits);

        let mut current_page = 1;
        while current_page * PAGE_SIZE < total_hits {
            let mut batch_futures = Vec::with_capacity(BATCH_SIZE as usize);

            for page in (current_page + 1)
                ..=(current_page + BATCH_SIZE).min((total_hits + PAGE_SIZE - 1) / PAGE_SIZE)
            {
                let skip = ((page - 1) * PAGE_SIZE).min(total_hits - 1);

                let mut page_options = options.clone();
                page_options.page = Some(page);
                page_options.from = Some(skip);
                page_options.count = Some(PAGE_SIZE);
                page_options.reverse_order = Some(false);

                batch_futures.push(self.search(page_options));
            }

            let results = futures::future::join_all(batch_futures).await;

            for result in results {
                match result {
                    Ok(response) => {
                        all_hits.extend(response.hits.hits);
                    }
                    Err(e) => {
                        tracing::error!("Error fetching page: {}", e);
                    }
                }
            }

            current_page += BATCH_SIZE;
        }

        Ok(all_hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SEARCH_RESPONSE_FIXTURE: &str = "fixtures/search/search-index.json";

    #[test]
    fn test_parse_search_response() {
        let content = fs::read_to_string(SEARCH_RESPONSE_FIXTURE).unwrap();
        let response: SearchResponse = serde_json::from_str(&content).unwrap();

        assert_eq!(response.took, 49);
        assert!(!response.timed_out);
        assert_eq!(response.hits.total.value, 146);

        let first_hit = &response.hits.hits[0];
        assert_eq!(first_hit._source.form, "8-K");
        assert!(!first_hit._source.display_names.is_empty());
    }

    #[tokio::test]
    async fn test_search() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = SearchOptions::new()
            .with_query("MAQC")
            .with_forms(vec!["8-K".to_string()])
            .with_count(10);

        let response = edgar.search(options).await.unwrap();
        assert!(!response.hits.hits.is_empty());

        // Verify hits contain expected form type
        for hit in response.hits.hits {
            assert!(hit._source.form.starts_with("8-K"));
        }
    }

    #[tokio::test]
    async fn test_search_with_date_range() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = SearchOptions::new()
            .with_query("MAQC")
            .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

        let response = edgar.search(options).await.unwrap();

        // Verify dates fall within range
        for hit in response.hits.hits {
            let file_date =
                chrono::NaiveDate::parse_from_str(&hit._source.file_date, "%Y-%m-%d").unwrap();
            assert!(file_date >= chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
            assert!(file_date <= chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());
        }
    }

    #[test]
    fn test_search_options_builder() {
        let options = SearchOptions::new()
            .with_query("test")
            .with_forms(vec!["10-K".to_string(), "10-Q".to_string()])
            .with_count(10)
            .with_reverse_order(true);

        let params = options.to_query_params();

        assert!(params.contains(&("q".to_string(), "test".to_string())));
        assert!(params.contains(&("forms".to_string(), "10-K,10-Q".to_string())));
        assert!(params.contains(&("count".to_string(), "10".to_string())));
        assert!(params.contains(&("reverse_order".to_string(), "TRUE".to_string())));
    }

    #[test]
    fn test_parse_search_response_with_null_fields() {
        let content = fs::read_to_string(SEARCH_RESPONSE_FIXTURE).unwrap();
        let response: SearchResponse = serde_json::from_str(&content).unwrap();

        for hit in response.hits.hits {
            // These fields can be null
            let _ = hit._source.xsl; // Option<String>
            let _ = hit._source.period_ending; // Option<String>
            let _ = hit._source.file_description; // Option<String>
        }
    }

    #[tokio::test]
    async fn test_search_null_fields_handling() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = SearchOptions::new()
            .with_query("MAQC")
            .with_forms(vec!["DEFA14A".to_string()])
            .with_count(10);

        let response = edgar.search(options).await.unwrap();

        for hit in response.hits.hits {
            // Verify we can handle null fields
            match hit._source.period_ending {
                Some(date) => println!("Period ending: {}", date),
                None => println!("No period ending date"),
            }
        }
    }

    #[tokio::test]
    async fn test_search_all() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = SearchOptions::new()
            .with_query("SPAC")
            .with_forms(vec!["S-1".to_string()])
            .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

        let results = edgar.search_all(options).await.unwrap();

        // Verify we got more than one page of results
        assert!(results.len() > 100);

        // Verify all results are S-1 forms
        for hit in results {
            assert!(hit._source.form.starts_with("S-1"));
        }
    }

    #[tokio::test]
    async fn test_search_all_with_small_result_set() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = SearchOptions::new()
            .with_query("SPAC Rule 419")
            .with_forms(vec!["S-1".to_string()])
            .with_date_range("2024-01-01".to_string(), "2024-03-01".to_string());

        let results = edgar.search_all(options).await.unwrap();

        // Should get all results even if less than one page
        assert!(!results.is_empty());
    }
}
