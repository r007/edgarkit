//! Error types for the EdgarKit library.
//!
//! All fallible operations in EdgarKit return `Result<T, EdgarError>` where `EdgarError`
//! is an enum covering various failure modes: network errors, HTTP status codes, parsing
//! failures, validation errors, and SEC-specific issues.
//!
//! Errors are designed to be informative, including context like URL previews and HTTP
//! status codes to aid in debugging. The error types use `thiserror` for clean `Display`
//! implementations and proper `Error` trait support.

use std::string::FromUtf8Error;
use thiserror::Error;

/// Comprehensive error type for all EdgarKit operations.
///
/// This enum covers the various ways that operations can fail when interacting with
/// the SEC EDGAR system. Errors are categorized by their source: network issues,
/// HTTP status codes, parsing problems, configuration mistakes, or validation failures.
///
/// Each variant includes relevant context to help diagnose issues. For example,
/// `InvalidResponse` includes a preview of the response content, and `UnexpectedContentType`
/// shows both the expected and actual content types along with a content preview.
///
/// # Examples
///
/// Handling specific error types:
/// ```rust
/// # use edgarkit::{Edgar, EdgarError, FilingOperations};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let edgar = Edgar::new("app contact@example.com")?;
/// match edgar.get_recent_filings("0001234567").await {
///     Ok(filings) => println!("Found {} filings", filings.len()),
///     Err(EdgarError::NotFound) => println!("Company not found"),
///     Err(EdgarError::RateLimitExceeded) => println!("Rate limited, try again later"),
///     Err(e) => println!("Error: {}", e),
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Error, Debug)]
pub enum EdgarError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Resource not found")]
    NotFound,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid year: must be 1994 or greater")]
    InvalidYear,

    #[error("Invalid quarter: must be between 1 and 4")]
    InvalidQuarter,

    #[error("Invalid month: must be between 1 and 12")]
    InvalidMonth,

    #[error("Invalid day: must be between 1 and 31")]
    InvalidDay,

    #[error("Invalid year: must be 2005 or greater for XBRL")]
    InvalidXBRLYear,

    #[error("Ticker not found")]
    TickerNotFound,

    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("XML parsing error: {0}")]
    XmlError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[cfg(any(feature = "atom", feature = "rss"))]
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[cfg(any(feature = "atom", feature = "rss"))]
    #[error("XML deserialization error: {0}")]
    XmlDe(#[from] quick_xml::DeError),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Value conversion error: {0}")]
    ValueConversion(String),

    #[error("String parsing error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] FromUtf8Error),

    #[error(
        "Unexpected content type from URL {url}. Expected pattern {expected_pattern}, but got Content-Type: {got_content_type}. Content preview: {content_preview}..."
    )]
    UnexpectedContentType {
        url: String,
        expected_pattern: String, // e.g., "application/json"
        got_content_type: String,
        content_preview: String, // Add a preview of the content
    },
}

pub type Result<T> = std::result::Result<T, EdgarError>;
