//! Search SEC EDGAR filings using flexible criteria and filters.
//!
//! This module provides a powerful search interface to the SEC's EDGAR full-text search system.
//! You can search by form type, date ranges, company names, CIKs, keywords, and more. The search
//! API supports both single-page queries and automatic pagination through all matching results.
//!
//! Search results include comprehensive metadata about each filing such as company names, CIKs,
//! filing dates, form types, and accession numbers. Results are returned in reverse chronological
//! order by default (newest first).
//!
//! # Search Capabilities
//!
//! - Full-text search with keyword queries
//! - Filter by form types (10-K, 8-K, S-1, etc.)
//! - Date range filtering
//! - Company name or CIK filtering
//! - SIC code and location-based filtering
//! - Pagination with configurable page sizes
//!
//! # Performance
//!
//! The `search_all()` method fetches all results across multiple pages using parallel requests
//! (up to 7 concurrent) while respecting rate limits. This provides significantly better
//! performance than sequential pagination for large result sets.
//!
//! # Example
//!
//! ```ignore
//! use edgarkit::{Edgar, SearchOperations, SearchOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let edgar = Edgar::new("your_app_name contact@example.com")?;
//!
//! let options = SearchOptions::new()
//!     .with_forms(vec!["10-K".to_string()])
//!     .with_date_range("2024-01-01".to_string(), "2024-12-31".to_string())
//!     .with_count(100);
//!
//! let results = edgar.search_all(options).await?;
//! # Ok(())
//! # }
//! ```

use super::Edgar;
use super::error::{EdgarError, Result};
use super::traits::SearchOperations;
use async_trait::async_trait;
use serde::{Deserialize, Deserializer, de};

/// Response container from the EDGAR search API containing search metadata and results.
///
/// This structure wraps the complete search response including timing information,
/// shard statistics from Elasticsearch, and the actual search hits. The search
/// system uses Elasticsearch under the hood, which is why you'll see fields like
/// `_shards` and `_score` that are specific to that search engine.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResponse {
    /// Time taken to execute search (ms)
    pub took: u32,

    /// Whether the search timed out
    pub timed_out: bool,

    /// Shard information
    pub _shards: Shards,

    /// Search results
    pub hits: Hits,
}

/// Information about Elasticsearch shards that processed the search query.
///
/// The EDGAR search system uses Elasticsearch which distributes data across multiple
/// shards for performance. This struct provides diagnostic information about how many
/// shards were involved and whether all completed successfully.
#[derive(Debug, Clone, Deserialize)]
pub struct Shards {
    pub total: u32,
    pub successful: u32,
    pub skipped: u32,
    pub failed: u32,
}

/// Container for search results including total count and individual hit documents.
///
/// This structure holds the array of matching filings along with metadata about the
/// total number of matches and relevance scoring. The `total` field indicates how
/// many documents matched your search criteria, while `hits` contains the actual
/// results for the current page.
#[derive(Debug, Clone, Deserialize)]
pub struct Hits {
    /// Total hits information
    pub total: TotalHits,

    /// Maximum relevance score
    #[serde(default)]
    pub max_score: Option<f64>,

    /// Array of hit documents
    pub hits: Vec<Hit>,
}

/// Total count of matching documents and the relationship type.
///
/// The `relation` field indicates whether the count is exact ("eq") or a lower bound
/// ("gte"). For very large result sets, Elasticsearch may provide an approximate count.
#[derive(Debug, Clone, Deserialize)]
pub struct TotalHits {
    pub value: u32,
    pub relation: String,
}

/// A single search result representing a matching SEC filing.
///
/// Each hit contains metadata about the search match (index name, document ID, relevance
/// score) and the actual filing data in the `_source` field. The underscore-prefixed
/// fields are Elasticsearch conventions for system metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct Hit {
    /// Index name
    pub _index: String,

    /// Document ID
    pub _id: String,

    /// Relevance score
    #[serde(default)]
    pub _score: Option<f64>,

    /// Filing information
    pub _source: Source,
}

/// Filing information and metadata extracted from the EDGAR search index.
///
/// This structure contains all the details about a specific SEC filing including company
/// identifiers, filing metadata, form type, and business information. This is the primary
/// data payload for each search result and includes everything you need to identify and
/// retrieve the actual filing documents.
///
/// Many fields are arrays because a single filing can be associated with multiple entities,
/// locations, or classification codes. For example, merger filings may list multiple CIKs.
#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    /// Company CIK numbers
    pub ciks: Vec<String>,

    /// Period ending date (if applicable)
    #[serde(default)]
    pub period_ending: Option<String>,

    /// File numbers
    pub file_num: Option<Vec<String>>,

    /// Company display names
    pub display_names: Vec<String>,

    /// XSL stylesheet reference
    #[serde(default)]
    pub xsl: Option<String>,

    /// Sequence number
    #[serde(deserialize_with = "deserialize_sequence")]
    pub sequence: u32,

    /// Root form types
    pub root_forms: Vec<String>,

    /// Filing date (YYYY-MM-DD)
    pub file_date: String,

    /// Business states
    pub biz_states: Vec<String>,

    /// SIC codes
    pub sics: Vec<String>,

    /// Form type (e.g., "10-K", "8-K")
    pub form: String,

    /// Accession number
    pub adsh: String,

    /// Film numbers
    pub film_num: Vec<String>,

    /// Business locations
    pub biz_locations: Vec<String>,

    /// File type
    pub file_type: String,

    /// File description
    #[serde(default)]
    pub file_description: Option<String>,

    /// Incorporation states
    pub inc_states: Vec<String>,

    /// Item numbers (for 8-K)
    pub items: Option<Vec<String>>,
}

/// Configurable options for searching SEC EDGAR filings.
///
/// This builder-style struct allows you to construct complex search queries using a fluent
/// interface. Combine multiple filters to narrow down results: form types, date ranges,
/// company identifiers, keywords, and more. All options are optional - you can construct
/// as simple or complex a query as needed.
///
/// The search system supports advanced query syntax including Boolean operators, phrase
/// searches with quotes, and wildcards. See the SEC's EDGAR full-text search FAQ for
/// details on query syntax and special operators.
///
/// # Builder Pattern
///
/// Options are set using builder methods that return `self`, allowing you to chain
/// multiple calls together. For example:
///
/// ```rust
/// # use edgarkit::SearchOptions;
/// let options = SearchOptions::new()
///     .with_query("acquisition merger")
///     .with_forms(vec!["8-K".to_string()])
///     .with_date_range("2024-01-01".to_string(), "2024-12-31".to_string())
///     .with_count(100);
/// ```
///
/// # Pagination
///
/// Control pagination using `with_page()`, `with_from()`, and `with_count()`. The maximum
/// results per page is 100. For retrieving all results across multiple pages, use the
/// `search_all()` method instead of manually paginating.
///
/// # Common Patterns
///
/// - **Recent filings**: Use `with_forms()` and `with_count()` without date filters
/// - **Company-specific**: Use `with_ciks()` to filter by one or more company CIKs
/// - **Date-bounded**: Use `with_date_range()` to limit results to a specific time period
/// - **Form type filtering**: Use `with_forms()` to search specific filing types
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Typeahead keys
    pub keys_typed: Option<String>,

    /// Search query (supports special operators, see SEC FAQ)
    pub query: Option<String>,

    /// Filing category
    pub category: Option<String>,

    /// Filter by company location
    pub location_code: Option<String>,

    /// Company or individual name (cannot combine with cik or sic)
    pub entity_name: Option<String>,

    /// Form types to search (e.g., ["10-K", "10-Q"])
    pub forms: Option<Vec<String>>,

    /// Filter by multiple location codes
    pub location_codes: Option<Vec<String>>,

    /// Page number for pagination
    pub page: Option<u32>,

    /// Number of results to skip
    pub from: Option<u32>,

    /// Number of results to return (max 100)
    pub count: Option<u32>,

    /// Order by oldest first instead of newest
    pub reverse_order: Option<bool>,

    /// Start date (YYYY-MM-DD, requires end_date)
    pub start_date: Option<String>,

    /// End date (YYYY-MM-DD, requires start_date)
    pub end_date: Option<String>,

    /// Search by base words (default) or exactly as entered
    pub stemming: Option<String>,

    /// CIK codes to search (cannot combine with name or sic)
    pub ciks: Option<Vec<String>>,

    /// Standard Industrial Classification code
    pub sic: Option<String>,

    /// Use incorporation location instead of HQ location
    pub incorporated_location: Option<bool>,
}

/// Custom deserializer for sequence field that can be either u32 or string
fn deserialize_sequence<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    struct SequenceVisitor;

    impl<'de> de::Visitor<'de> for SequenceVisitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer or a string containing an integer")
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as u32)
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_any(SequenceVisitor)
}

impl SearchOptions {
    /// Creates a new instance of SearchOptions with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the search query text.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use edgarkit::SearchOptions;
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

    /// Sets company CIK filter(s).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use edgarkit::SearchOptions;
    /// // Single CIK
    /// let options = SearchOptions::new().with_ciks(vec!["0001234567".to_string()]);
    ///
    /// // Multiple CIKs
    /// let options = SearchOptions::new().with_ciks(vec!["0001234567".to_string(), "0007654321".to_string()]);
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

/// Search operations for querying SEC EDGAR filings with flexible filters and criteria.
///
/// This trait provides two main search methods: `search()` for single-page queries and
/// `search_all()` for comprehensive multi-page retrieval. Both methods use the same
/// `SearchOptions` for filtering, but `search_all()` automatically handles pagination
/// and fetches all matching results in parallel batches.
///
/// The search system is powered by SEC's EDGAR full-text search, which indexes filing
/// content, company names, form types, and metadata. Results are ranked by relevance
/// when using keyword queries, or sorted by filing date when searching by form type
/// or date range.
///
/// # Performance Considerations
///
/// For large result sets (>100 documents), `search_all()` is significantly faster than
/// manually paginating because it fetches multiple pages concurrently. However, it will
/// retrieve ALL matching results, which could be thousands of documents. Consider using
/// date ranges or other filters to limit scope when appropriate.
///
/// # Example
///
/// ```ignore
/// use edgarkit::{Edgar, SearchOperations, SearchOptions};
///
/// async fn example() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("your_app_name contact@example.com")?;
///     
///     let options = SearchOptions::new()
///         .with_forms(vec!["10-K".to_string()])
///         .with_count(10);
///     
///     // Single page
///     let first_page = edgar.search(options.clone()).await?;
///     println!("First page: {} results", first_page.hits.hits.len());
///     
///     // All results across pages
///     let all_results = edgar.search_all(options).await?;
///     println!("Total results: {}", all_results.len());
///     Ok(())
/// }
/// ```
#[async_trait]
impl SearchOperations for Edgar {
    /// Executes a search query and returns a single page of results.
    ///
    /// This method performs one search request and returns the results for the specified
    /// page. Use this when you only need a small number of results or want to implement
    /// custom pagination logic. For retrieving all matching results, use `search_all()`
    /// which handles pagination automatically.
    ///
    /// The returned `SearchResponse` includes metadata about the search (execution time,
    /// total hits) along with the actual results for the current page. By default, results
    /// are sorted by filing date (newest first) unless a keyword query is provided, in
    /// which case they're ranked by relevance.
    ///
    /// # Arguments
    ///
    /// * `options` - Search filters and pagination settings
    ///
    /// # Returns
    ///
    /// Returns a `SearchResponse` containing search metadata and results for one page.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let options = SearchOptions::new()
    ///     .with_forms(vec!["10-Q".to_string()])
    ///     .with_page(1)
    ///     .with_count(50);
    ///
    /// let response = edgar.search(options).await?;
    /// println!("Found {} total matches", response.hits.total.value);
    /// println!("This page has {} results", response.hits.hits.len());
    /// ```
    async fn search(&self, options: SearchOptions) -> Result<SearchResponse> {
        let params = options.to_query_params();
        let query_string = serde_urlencoded::to_string(&params)
            .map_err(|e| EdgarError::InvalidResponse(e.to_string()))?;

        let url = format!("{}?{}", self.search_url(), query_string);
        let response = self.get(&url).await?;

        Ok(serde_json::from_str(&response)?)
    }

    /// Fetches all matching results across multiple pages with automatic pagination.
    ///
    /// This method is designed for comprehensive data retrieval where you need all filings
    /// matching your search criteria. It automatically handles pagination by first querying
    /// for total count, then fetching all pages in parallel batches of up to 7 concurrent
    /// requests. This provides excellent performance while respecting SEC rate limits.
    ///
    /// The method aggregates all results into a single vector of `Hit` objects, making it
    /// easy to process the complete result set. Progress and errors are logged using the
    /// `tracing` crate, so you can monitor long-running searches.
    ///
    /// # Performance Notes
    ///
    /// - Uses parallel requests (batch size: 7) to fetch multiple pages simultaneously
    /// - Respects rate limiting between batches
    /// - For 1000+ results, this is significantly faster than sequential pagination
    /// - Memory usage scales with result set size - consider filtering for very large queries
    ///
    /// # Arguments
    ///
    /// * `options` - Search filters and criteria (pagination options are overridden)
    ///
    /// # Returns
    ///
    /// Returns a vector containing all matching `Hit` objects across all pages.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let options = SearchOptions::new()
    ///     .with_query("quarterly earnings")
    ///     .with_forms(vec!["10-Q".to_string()])
    ///     .with_date_range("2024-01-01".to_string(), "2024-03-31".to_string());
    ///
    /// let all_results = edgar.search_all(options).await?;
    /// println!("Retrieved {} quarterly reports", all_results.len());
    ///
    /// for hit in all_results {
    ///     println!("{}: {} filed on {}",
    ///         hit._source.display_names[0],
    ///         hit._source.form,
    ///         hit._source.file_date);
    /// }
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

        let total_pages = (total_hits + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut current_page = 1;

        while current_page < total_pages {
            let end_page = (current_page + BATCH_SIZE).min(total_pages);
            let mut batch_futures = Vec::with_capacity((end_page - current_page) as usize);

            for page in (current_page + 1)..=end_page {
                let skip = (page - 1) * PAGE_SIZE;

                // Stop if we've gone past the total hits
                if skip >= total_hits {
                    break;
                }

                let mut page_options = options.clone();
                page_options.page = Some(page);
                page_options.from = Some(skip);
                page_options.count = Some(PAGE_SIZE.min(total_hits - skip));
                page_options.reverse_order = Some(false);

                batch_futures.push(self.search(page_options));
            }

            if batch_futures.is_empty() {
                break;
            }

            let results = futures::future::join_all(batch_futures).await;

            for result in results {
                match result {
                    Ok(response) => {
                        all_hits.extend(response.hits.hits);
                    }
                    Err(e) => {
                        tracing::error!("Error fetching page: {}", e);
                        return Err(e);
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
}
