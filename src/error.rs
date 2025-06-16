use parsers::error::ParserError;
use std::string::FromUtf8Error;
use thiserror::Error;

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

    #[error("Parser error: {0}")]
    ParserError(#[from] ParserError),

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

impl From<quick_xml::DeError> for EdgarError {
    fn from(error: quick_xml::DeError) -> Self {
        EdgarError::XmlError(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, EdgarError>;
