//! # EdgarKit - A Rust client for the SEC EDGAR system
//!
//! EdgarKit provides a comprehensive and ergonomic API for interacting with
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
//! ## Requirements
//!
//! EdgarKit is an async-first library and requires an async runtime. We recommend
//! [tokio](https://tokio.rs), which is the most widely used async runtime in the Rust ecosystem.
//!
//! ## Basic Usage
//!
//! ```ignore
//! use edgarkit::{Edgar, FilingOperations, FilingOptions};
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

mod config;
mod core;
mod error;
pub mod parsing;

// Conditionally include modules
#[cfg(any(feature = "filings", feature = "index", feature = "feeds"))]
mod options;

#[cfg(any(
    feature = "company",
    feature = "filings",
    feature = "feeds",
    feature = "index",
    feature = "search"
))]
mod traits;

// Public modules
#[cfg(feature = "company")]
mod company;
#[cfg(feature = "feeds")]
mod feeds;
#[cfg(feature = "filings")]
mod filings;
#[cfg(feature = "index")]
mod index;
#[cfg(feature = "search")]
mod search;

// Core Edgar functionality (always available)
pub use config::{EdgarConfig, EdgarUrls};
pub use core::Edgar;
pub use error::{EdgarError, Result};

// Conditionally export options
#[cfg(feature = "feeds")]
pub use options::FeedOptions;
#[cfg(any(feature = "filings", feature = "index"))]
pub use options::FilingOptions;

// Re-export core types and traits for a clean API
#[cfg(feature = "company")]
pub use company::{
    CompanyConcept, CompanyFacts, CompanyTicker, CompanyTickerExchange, Frame, MutualFundTicker,
};
#[cfg(feature = "filings")]
pub use filings::{DetailedFiling, Directory, DirectoryItem, DirectoryResponse, Submission};
#[cfg(feature = "index")]
pub use index::{EdgarDay, EdgarPeriod, IndexResponse, Quarter};
#[cfg(feature = "search")]
pub use search::{Hit, Hits, SearchOptions, SearchResponse, TotalHits};

// Conditionally export traits
#[cfg(feature = "company")]
pub use traits::CompanyOperations;
#[cfg(feature = "feeds")]
pub use traits::FeedOperations;
#[cfg(feature = "filings")]
pub use traits::FilingOperations;
#[cfg(feature = "index")]
pub use traits::IndexOperations;
#[cfg(feature = "search")]
pub use traits::SearchOperations;

/// Current crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
