//! Trait definitions organizing EDGAR operations by feature area.
//!
//! EdgarKit uses traits to logically group related functionality into domains:
//! company information, filings, feeds, search, and indices. Each feature has
//! a corresponding trait that the `Edgar` client implements when that feature
//! is enabled.
//!
//! This design allows for:
//! - Clear separation of concerns
//! - Feature-gated compilation (only include what you need)
//! - Easy mocking and testing
//! - Discoverable API through trait methods
//!
//! Users typically interact with the `Edgar` struct directly rather than through
//! trait objects, but the traits are useful for understanding the API surface and
//! for testing scenarios where you want to provide alternative implementations.

#[cfg(feature = "company")]
use super::company::{
    CompanyConcept, CompanyFacts, CompanyTicker, CompanyTickerExchange, Frame, MutualFundTicker,
};
use super::error::Result;
#[cfg(feature = "filings")]
use super::filings::{DetailedFiling, DirectoryResponse, Submission};
#[cfg(feature = "index")]
use super::index::{EdgarDay, EdgarPeriod, IndexResponse};
#[cfg(any(feature = "filings", feature = "index", feature = "feeds"))]
use super::options::{FeedOptions, FilingOptions};
#[cfg(feature = "search")]
use super::search::{Hit, SearchOptions, SearchResponse};
use async_trait::async_trait;
#[cfg(feature = "feeds")]
use parsers::atom::AtomDocument;
#[cfg(feature = "index")]
use parsers::index::IndexEntry;
#[cfg(feature = "feeds")]
use parsers::rss::RssDocument;

/// Operations for retrieving company information and financial data.
///
/// This trait provides access to SEC company identifiers (CIKs), ticker mappings,
/// and structured financial data through the XBRL API. It covers company metadata,
/// fact aggregations, specific concept queries, and aggregated frames across companies.
///
/// Company data is retrieved from SEC's data API which provides JSON-formatted
/// company facts based on XBRL filings. This is particularly useful for financial
/// analysis and building company databases.
#[cfg(feature = "company")]
#[async_trait]
pub trait CompanyOperations {
    /// Retrieves a list of all company tickers from EDGAR.
    async fn company_tickers(&self) -> Result<Vec<CompanyTicker>>;
    /// Retrieves the Central Index Key (CIK) for a given company ticker symbol.
    async fn company_cik(&self, ticker: &str) -> Result<u64>;
    /// Retrieves the CIK for a given mutual fund ticker symbol.
    async fn mutual_fund_cik(&self, ticker: &str) -> Result<u64>;
    /// Retrieves a list of company tickers along with their exchange information.
    async fn company_tickers_with_exchange(&self) -> Result<Vec<CompanyTickerExchange>>;
    /// Retrieves a list of mutual fund tickers from the SEC EDGAR database.
    async fn mutual_fund_tickers(&self) -> Result<Vec<MutualFundTicker>>;
    /// Retrieves company facts and financial data for a given CIK.
    async fn company_facts(&self, cik: u64) -> Result<CompanyFacts>;
    /// Retrieves specific concept data for a company using taxonomy and tag.
    async fn company_concept(&self, cik: u64, taxonomy: &str, tag: &str) -> Result<CompanyConcept>;
    /// Retrieves frames for a given taxonomy, concept, unit, and period.
    async fn frames(&self, taxonomy: &str, tag: &str, unit: &str, period: &str) -> Result<Frame>;
}

/// Operations for accessing SEC filings and related documents.
///
/// This trait provides comprehensive access to company filings including submissions
/// data, filing directories, and document content. It supports retrieving recent
/// filings, latest filings of specific types, and generating URLs for text filings
/// and SGML headers.
///
/// Filing operations are the core of most EDGAR use cases, enabling you to discover
/// what a company has filed and retrieve the actual filing documents for analysis.
#[cfg(feature = "filings")]
#[async_trait]
pub trait FilingOperations {
    /// Retrieves all submissions for a specific company identified by CIK.
    async fn submissions(&self, cik: &str) -> Result<Submission>;
    /// Helper function to get recent filings in a form of a Vec.
    async fn get_recent_filings(&self, cik: &str) -> Result<Vec<DetailedFiling>>;
    /// Retrieves a list of filings for a specific company identified by CIK.
    async fn filings(&self, cik: &str, opts: Option<FilingOptions>) -> Result<Vec<DetailedFiling>>;
    /// Retrieves the directory structure for a specific filing.
    async fn filing_directory(
        &self,
        cik: &str,
        accession_number: &str,
    ) -> Result<DirectoryResponse>;
    /// Retrieves the directory structure for a specific entity.
    async fn entity_directory(&self, cik: &str) -> Result<DirectoryResponse>;
    /// Constructs a filing URL from a combined filing ID (format: "accession_number:filename")
    fn get_filing_url_from_id(&self, cik: &str, filing_id: &str) -> Result<String>;
    /// Fetches a filing's content directly using its URL
    async fn get_filing_content_by_id(&self, cik: &str, filing_id: &str) -> Result<String>;
    /// Fetches the latest filing for a company matching one of the requested form types.
    ///
    /// Use this when you want “latest 10-Q **or** 10-K”, etc. The forms are applied as a filter,
    /// and the newest matching filing (as returned by the SEC) is downloaded.
    async fn get_latest_filing_content(&self, cik: &str, form_types: &[&str]) -> Result<String>;
    /// Generates URLs for text filings with original SEC.gov links based on specified options without downloading content
    async fn get_text_filing_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String, String)>>;
    /// Generates URLs for SGML header files with original SEC.gov links based on specified options without downloading content
    async fn get_sgml_header_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String, String)>>;
}

/// Operations for accessing EDGAR Atom and RSS feeds.
///
/// This trait provides methods to retrieve various SEC feeds including current filings,
/// company-specific feeds, press releases, speeches, and specialized XBRL feeds. Feeds
/// are useful for monitoring recent activity, tracking news, and staying updated on
/// regulatory announcements.
///
/// The SEC provides both Atom feeds (for filings) and RSS feeds (for news and alerts).
/// This trait abstracts the differences and provides a consistent interface to both.
#[cfg(feature = "feeds")]
#[async_trait]
pub trait FeedOperations {
    /// Retrieves the current EDGAR feed with optional parameters.
    async fn current_feed(&self, opts: Option<FeedOptions>) -> Result<AtomDocument>;
    /// Parses the current feed from a string
    fn current_feed_from_string(&self, content: &str) -> Result<AtomDocument>;
    /// Retrieves the feed for a specific company identified by CIK.
    async fn company_feed(&self, cik: &str, opts: Option<FeedOptions>) -> Result<AtomDocument>;
    /// Parses the company feed from a string
    fn company_feed_from_string(&self, content: &str) -> Result<AtomDocument>;
    /// Retrieves an RSS feed from a specified URL.
    async fn get_rss_feed(&self, url: &str) -> Result<RssDocument>;
    /// Parses an RSS feed from a string
    fn rss_feed_from_string(&self, content: &str) -> Result<RssDocument>;
    /// Fetches the press release feed
    async fn press_release_feed(&self) -> Result<RssDocument>;
    /// Fetches the speeches and statements feed
    async fn speeches_and_statements_feed(&self) -> Result<RssDocument>;
    /// Fetches the speeches feed
    async fn speeches_feed(&self) -> Result<RssDocument>;
    /// Fetches the statements feed
    async fn statements_feed(&self) -> Result<RssDocument>;
    /// Fetches the testimony feed
    async fn testimony_feed(&self) -> Result<RssDocument>;
    /// Fetches the administrative proceedings feed
    async fn administrative_proceedings_feed(&self) -> Result<RssDocument>;
    /// Fetches the division of corporation finance feed
    async fn division_of_corporation_finance_feed(&self) -> Result<RssDocument>;
    /// Fetches the division of investment management feed
    async fn division_of_investment_management_feed(&self) -> Result<RssDocument>;
    /// Fetches the investor alerts feed
    async fn investor_alerts_feed(&self) -> Result<RssDocument>;
    /// Fetches the filings feed
    async fn filings_feed(&self) -> Result<RssDocument>;
    /// Fetches the mutual funds feed
    async fn mutual_funds_feed(&self) -> Result<RssDocument>;
    /// Fetches the XBRL feed
    async fn xbrl_feed(&self) -> Result<RssDocument>;
    /// Fetches the inline XBRL feed
    async fn inline_xbrl_feed(&self) -> Result<RssDocument>;
    /// Fetches the historical XBRL feed
    async fn historical_xbrl_feed(&self, year: i32, month: i32) -> Result<RssDocument>;
}

/// Operations for retrieving daily and quarterly filing indices.
///
/// The SEC publishes index files that list all filings for a given day or quarter.
/// These indices are useful for bulk processing, historical analysis, or discovering
/// filings without relying on the search API. Index files are available from 1994
/// onwards when the EDGAR system began.
///
/// Indices provide a lightweight way to get filing metadata without downloading full
/// documents, making them ideal for building filing databases or monitoring systems.
#[cfg(feature = "index")]
#[async_trait]
pub trait IndexOperations {
    /// Retrieves the full index file for a specific year and quarter.
    async fn full_index(&self, period: Option<EdgarPeriod>) -> Result<IndexResponse>;
    /// Retrieves the daily index file for a specific period.
    async fn daily_index(&self, period: Option<EdgarPeriod>) -> Result<IndexResponse>;
    /// Gets and parses daily index file for specific date
    async fn get_daily_filings(
        &self,
        day: EdgarDay,
        options: Option<FilingOptions>,
    ) -> Result<Vec<IndexEntry>>;
    /// Gets and parses daily index file for specific date
    async fn get_period_filings(
        &self,
        period: EdgarPeriod,
        options: Option<FilingOptions>,
    ) -> Result<Vec<IndexEntry>>;
}

/// Operations for searching EDGAR filings with flexible criteria.
///
/// The search trait provides access to SEC's full-text search capabilities, allowing
/// you to find filings by keywords, form types, dates, companies, and other attributes.
/// It supports both single-page queries and comprehensive multi-page retrieval.
///
/// Search is particularly useful when you need to find filings based on content or
/// when you don't know exact identifiers. The search system indexes filing text,
/// metadata, and company information for comprehensive discoverability.
#[cfg(feature = "search")]
#[async_trait]
pub trait SearchOperations {
    /// Performs a search query on EDGAR
    async fn search(&self, options: SearchOptions) -> Result<SearchResponse>;
    /// Performs a search query and fetches all available pages
    async fn search_all(&self, options: SearchOptions) -> Result<Vec<Hit>>;
}
