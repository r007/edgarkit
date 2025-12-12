use serde::Serialize;
use std::collections::HashMap;

/// Options for filtering filing requests
#[cfg(any(feature = "filings", feature = "index"))]
#[derive(Debug, Clone)]
pub struct FilingOptions {
    pub form_types: Option<Vec<String>>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
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

/// Options for feed requests
#[cfg(feature = "feeds")]
#[derive(Debug, Clone, Default, Serialize)]
pub struct FeedOptions {
    #[serde(flatten)]
    params: HashMap<String, String>,
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
