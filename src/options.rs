//! Configuration options for filing and feed operations.
//!
//! This module provides builder-style option structs for customizing filing queries
//! and feed requests. Options use the builder pattern for clean, composable configuration.

use serde::Serialize;
use std::collections::HashMap;

/// Options for filtering and configuring filing queries.
///
/// This struct allows you to filter filings by form type, skip or limit results,
/// filter by CIK, and control whether amendments are automatically included. It's
/// used across both direct filing operations and index-based queries to provide
/// consistent filtering behavior.
///
/// The default configuration includes amendments and applies no filters, returning
/// all available filings. Use the builder methods to narrow results based on your
/// specific needs.
///
/// # Examples
///
/// Basic usage with form type filter:
/// ```rust
/// # use edgarkit::FilingOptions;
/// let options = FilingOptions::new()
///     .with_form_type("10-K")
///     .with_limit(10);
/// ```
///
/// Multiple form types with pagination:
/// ```rust
/// # use edgarkit::FilingOptions;
/// let options = FilingOptions::new()
///     .with_form_types(vec!["10-K".to_string(), "10-Q".to_string()])
///     .with_offset(20)
///     .with_limit(10);
/// ```
///
/// Exclude amendments:
/// ```rust
/// # use edgarkit::FilingOptions;
/// let options = FilingOptions::new()
///     .with_form_type("S-1")
///     .with_include_amendments(false);
/// ```
#[cfg(any(feature = "filings", feature = "index"))]
#[derive(Debug, Clone)]
pub struct FilingOptions {
    // Which form types to include (e.g., ["10-K"])
    pub form_types: Option<Vec<String>>,

    // Skip this many filings from the start
    pub offset: Option<usize>,

    // Return at most this many filings
    pub limit: Option<usize>,

    // Optional filter for multiple CIKs
    pub ciks: Option<Vec<u64>>,

    /// Whether to automatically include amendment forms (e.g., S-1/A when S-1 is requested).
    /// Defaults to true.
    pub include_amendments: bool,
}

#[cfg(any(feature = "filings", feature = "index"))]
impl Default for FilingOptions {
    fn default() -> Self {
        Self {
            form_types: None,
            offset: None,
            limit: None,
            ciks: None,
            include_amendments: true,
        }
    }
}

#[cfg(any(feature = "filings", feature = "index"))]
impl FilingOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_form_type(mut self, form_type: impl Into<String>) -> Self {
        let form_type = form_type.into();
        self.form_types = Some(vec![form_type]);
        self
    }

    pub fn with_form_types(mut self, form_types: Vec<String>) -> Self {
        self.form_types = Some(form_types);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_cik(mut self, cik: u64) -> Self {
        self.ciks = Some(vec![cik]);
        self
    }

    pub fn with_ciks(mut self, ciks: Vec<u64>) -> Self {
        self.ciks = Some(ciks);
        self
    }

    /// Set whether to include amendment forms automatically.
    ///
    /// When true (default), requesting "S-1" will also include "S-1/A" filings.
    /// When false, only the exact form type specified will be returned.
    pub fn with_include_amendments(mut self, include_amendments: bool) -> Self {
        self.include_amendments = include_amendments;
        self
    }
}

/// Options for customizing SEC feed requests.
///
/// Feed options use a simple key-value parameter system that maps directly to the
/// SEC's feed query string parameters. Common parameters include `count` (number of
/// results), `type` (form type filter), and `start` (pagination offset).
///
/// The options default to Atom output format, which is the standard for SEC feeds.
/// You can add custom parameters using `with_param()` to access advanced feed features.
///
/// # Examples
///
/// ```rust
/// # use edgarkit::FeedOptions;
/// let options = FeedOptions::new(None)
///     .with_param("count", "50")
///     .with_param("type", "10-K");
/// ```
#[cfg(feature = "feeds")]
#[derive(Debug, Clone, Default, Serialize)]
pub struct FeedOptions {
    #[serde(flatten)]
    params: HashMap<String, String>, // Arbitrary feed parameters (e.g., count=10)
}

#[cfg(feature = "feeds")]
impl FeedOptions {
    fn default() -> Self {
        let mut options = FeedOptions {
            params: HashMap::new(),
        };
        options
            .params
            .insert("output".to_string(), "atom".to_string());
        options
    }

    pub fn new(params: Option<FeedOptions>) -> Self {
        match params {
            Some(options) => Self::default().merge(options),
            None => Self::default(),
        }
    }

    // Add a merge method to combine two FeedOptions
    pub fn merge(mut self, other: FeedOptions) -> Self {
        // Extend the current params with the other params
        self.params.extend(other.params);
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }
}
