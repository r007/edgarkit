use super::company::{
    CompanyConcept, CompanyFacts, CompanyTicker, CompanyTickerExchange, Frame, MutualFundTicker,
};
use super::error::Result;
use super::filings::{DetailedFiling, DirectoryResponse, Submission};
use super::index::{EdgarDay, EdgarPeriod, IndexResponse};
use super::options::{FeedOptions, FilingOptions};
use super::search::{Hit, SearchOptions, SearchResponse};
use async_trait::async_trait;
use parsers::atom::AtomDocument;
use parsers::index::IndexEntry;
use parsers::rss::RssDocument;

/// A collection of trait definitions for interacting with the SEC EDGAR system.
/// These traits provide a comprehensive interface for retrieving and parsing
/// various types of financial data and documents from the SEC EDGAR database.

/// Operations related to company information retrieval from EDGAR.
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

/// Operations related to SEC filing retrieval and management.
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
    /// Fetches the latest filing of a specific type for a company
    async fn get_latest_filing_content(&self, cik: &str, form_type: &str) -> Result<String>;
    /// Generates URLs for text filings based on specified options without downloading content
    async fn get_text_filing_links(
        &self,
        cik: &str,
        opts: Option<FilingOptions>,
    ) -> Result<Vec<(DetailedFiling, String)>>;
}

/// Operations related to EDGAR feed data retrieval.
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
    /// Fetches the litigation feed
    async fn litigation_feed(&self) -> Result<RssDocument>;
    /// Fetches the administrative proceedings feed
    async fn administrative_proceedings_feed(&self) -> Result<RssDocument>;
    /// Fetches the trading suspensions feed
    async fn trading_suspensions_feed(&self) -> Result<RssDocument>;
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

/// Operations for retrieving EDGAR index files.
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

#[async_trait]
pub trait SearchOperations {
    /// Performs a search query on EDGAR
    async fn search(&self, options: SearchOptions) -> Result<SearchResponse>;
    /// Performs a search query and fetches all available pages
    async fn search_all(&self, options: SearchOptions) -> Result<Vec<Hit>>;
}
