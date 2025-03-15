//! # Edgar - A Rust client for the SEC EDGAR system
//!
//! The Edgar crate provides a comprehensive and ergonomic API for interacting with
//! the SEC's EDGAR (Electronic Data Gathering, Analysis, and Retrieval) system.
//!
//! ## Features
//!
//! - **Rate-limited HTTP client** - Complies with SEC.gov fair access rules
//! - **Filing operations** - Access company filings, submissions, and text documents
//! - **Company information** - Retrieve company facts, tickers, and metadata
//! - **Search capabilities** - Find filings with customizable search criteria
//! - **Feed operations** - Access Atom and RSS feeds for filings and news
//! - **Index operations** - Retrieve and parse daily and quarterly filing indices
//!
//! ## Basic Usage
//!
//! ```rust
//! use edgar::{Edgar, FilingOperations, FilingOptions};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize with a proper user agent (required by SEC.gov)
//!     let edgar = Edgar::new("YourAppName contact@example.com")?;
//!     
//!     // Get recent 10-K filings for a company
//!     let options = FilingOptions::new()
//!         .with_form_type("10-K")
//!         .with_limit(5);
//!     
//!     let filings = edgar.filings("320193", Some(options)).await?;
//!     
//!     for filing in filings {
//!         println!("Filing: {} on {}", filing.form, filing.filing_date);
//!     }
//!     
//!     Ok(())
//! }
//! ```

// Public modules
mod company;
mod config;
mod core;
mod error;
mod feeds;
mod filings;
mod index;
mod options;
mod search;
mod traits;

// Re-export core types and traits for a clean API
pub use company::{
    CompanyConcept, CompanyFacts, CompanyTicker, CompanyTickerExchange, Frame, MutualFundTicker,
};
pub use config::{EdgarConfig, EdgarUrls};
pub use core::Edgar;
pub use error::{EdgarError, Result};
pub use filings::{DetailedFiling, Directory, DirectoryItem, DirectoryResponse, Submission};
pub use index::{EdgarDay, EdgarPeriod, IndexResponse, Quarter};
pub use options::{FeedOptions, FilingOptions};
pub use search::{Hit, Hits, SearchOptions, SearchResponse, TotalHits};
pub use traits::{
    CompanyOperations, FeedOperations, FilingOperations, IndexOperations, SearchOperations,
};

// Version information
/// Current crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
