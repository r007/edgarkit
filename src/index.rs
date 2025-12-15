//! Daily and quarterly filing indices.
//!
//! EDGAR publishes *index files* that act as a manifest of filings, either for a single day
//! (daily index) or for an entire quarter (full index). These indices are a great fit for
//! bulk ingestion pipelines because they are stable, append-only over time, and avoid the
//! need to crawl company-by-company when you want “everything filed on a date”.
//!
//! This module implements `IndexOperations` for [`Edgar`]. Under the hood it:
//! - Lists available index files via SEC-provided `index.json` directory listings.
//! - Downloads the selected index file (`.idx` or `.gz`) from the EDGAR archives.
//! - Parses it using the `parsers::index` parser into [`IndexEntry`] records.
//! - Optionally applies [`FilingOptions`] filters (`form_types`, `ciks`, `offset`, `limit`).
//!
//! The SEC directory listing uses human-readable sizes and a custom timestamp format
//! (`MM/DD/YYYY HH:MM:SS AM/PM`), which is handled by `edgar_date_format`.
//!
//! # Examples
//!
//! ```ignore
//! use edgarkit::{Edgar, EdgarDay, EdgarPeriod, FilingOptions, IndexOperations, Quarter};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let edgar = Edgar::new("MyApp contact@example.com")?;
//!
//!     // Fetch all filings for a specific day.
//!     let day = EdgarDay::new(2023, 8, 15)?;
//!     let daily = edgar.get_daily_filings(day, None).await?;
//!
//!     // Apply filters (form types + CIKs) on the parsed index entries.
//!     let opts = FilingOptions::new()
//!         .with_form_types(vec!["10-K".to_string(), "8-K".to_string()])
//!         .with_ciks(vec![320193]);
//!     let filtered = edgar.get_daily_filings(day, Some(opts)).await?;
//!
//!     // Fetch all filings for a specific quarter.
//!     let period = EdgarPeriod::new(2023, Quarter::Q3)?;
//!     let quarterly = edgar.get_period_filings(period, None).await?;
//!
//!     // Inspect what index files exist for a quarter.
//!     let listing = edgar.daily_index(Some(period)).await?;
//!     println!("{} items", listing.directory.item.len());
//!     Ok(())
//! }
//! ```

use super::Edgar;
use super::error::{EdgarError, Result};
use super::options::FilingOptions;
use super::traits::IndexOperations;
use async_trait::async_trait;
use chrono::{Datelike, NaiveDateTime};
use flate2::read::GzDecoder;
use parsers::index::{IndexConfig, IndexEntry, IndexParser, IndexType};
use serde::{Deserialize, Serialize};
use serde_json;
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IndexResponse {
    /// Directory listing payload returned by `index.json`.
    pub directory: Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    /// Directory items (files and subdirectories).
    pub item: Vec<DirectoryItem>,

    /// Directory name (typically ends with a trailing `/`).
    pub name: String,

    /// Parent directory path as reported by the SEC listing.
    #[serde(rename = "parent-dir")]
    pub parent_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    /// A subdirectory.
    Dir,

    /// A file.
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryItem {
    /// Last modified timestamp.
    #[serde(rename = "last-modified")]
    #[serde(with = "edgar_date_format")]
    pub last_modified: NaiveDateTime,

    /// Item name (filename or directory name).
    pub name: String,

    /// Item type (file or directory).
    #[serde(rename = "type")]
    pub type_: ItemType,

    /// Relative URL path (joined with the corresponding archives prefix).
    pub href: String,

    /// File size (human-readable, as provided by the SEC listing).
    pub size: String,
}

/// Serde helpers for EDGAR date format (`MM/DD/YYYY HH:MM:SS AM/PM`).
mod edgar_date_format {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%m/%d/%Y %I:%M:%S %p";

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.format(FORMAT).to_string())
    }
}

/// Fiscal quarter (Q1-Q4).
///
/// EDGAR index directories are grouped by quarter (e.g., `QTR1` .. `QTR4`).
///
/// Each quarter maps to specific months:
/// - Q1: January through March
/// - Q2: April through June
/// - Q3: July through September
/// - Q4: October through December
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quarter {
    Q1 = 1,
    Q2 = 2,
    Q3 = 3,
    Q4 = 4,
}

impl Quarter {
    /// Creates a Quarter from a month number (1-12)
    ///
    /// # Arguments
    /// * `month` - Month number (1-12)
    ///
    /// # Returns
    /// * `Ok(Quarter)` if month is valid
    /// * `Err(EdgarError::InvalidMonth)` if month is invalid
    pub fn from_month(month: u32) -> Result<Self> {
        match month {
            1..=3 => Ok(Quarter::Q1),
            4..=6 => Ok(Quarter::Q2),
            7..=9 => Ok(Quarter::Q3),
            10..=12 => Ok(Quarter::Q4),
            _ => Err(EdgarError::InvalidMonth),
        }
    }

    /// Converts the quarter to its integer representation (1-4).
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// A specific day in EDGAR's system (must be 1994 or later).
#[derive(Debug, Clone, Copy)]
pub struct EdgarDay {
    /// Calendar year (>= 1994).
    year: i32,

    /// Calendar month (1-12).
    month: u32,

    /// Calendar day (1-31).
    day: u32,
}

/// A validated calendar date used to locate a daily EDGAR index file.
///
/// Daily indices live under `.../daily-index/<YEAR>/QTR<1-4>/` and include the date in the
/// filename (e.g., `company.20230815.idx`). `EdgarDay` exists to keep that path construction
/// correct and to provide a single place for basic validation.
///
/// # Example
/// ```rust
/// use edgarkit::{EdgarDay, Quarter, Result};
///
/// fn main() -> Result<()> {
///     let day = EdgarDay::new(2023, 12, 25)?;
///     assert_eq!(day.format_date(), "20231225");
///     assert_eq!(day.quarter(), Quarter::Q4);
///     Ok(())
/// }
/// ```
impl EdgarDay {
    /// Creates a new EdgarDay with validation.
    ///
    /// # Errors
    ///
    /// - `InvalidYear` if year < 1994
    /// - `InvalidMonth` if month not 1-12
    /// - `InvalidDay` if day not 1-31
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self> {
        if year < 1994 {
            return Err(EdgarError::InvalidYear);
        }
        if month < 1 || month > 12 {
            return Err(EdgarError::InvalidMonth);
        }
        if day < 1 || day > 31 {
            return Err(EdgarError::InvalidDay);
        }
        Ok(Self { year, month, day })
    }

    /// Formats as `YYYYMMDD` (e.g., "20230815").
    pub fn format_date(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    /// Returns the quarter directory (`QTR1`..`QTR4`) that EDGAR uses for this date.
    pub fn quarter(&self) -> Quarter {
        Quarter::from_month(self.month).unwrap()
    }

    /// Gets the year
    pub fn year(&self) -> i32 {
        self.year
    }
}

/// A fiscal period (year + quarter) used to locate quarterly index directories.
///
/// Quarterly indices live under paths like `.../full-index/<YEAR>/QTR<1-4>/` (and similarly
/// for `daily-index`). This type is intentionally small: it validates the year and carries the
/// quarter, which is all you need for directory listings and quarterly index retrieval.
#[derive(Debug, Clone, Copy)]
pub struct EdgarPeriod {
    year: i32,
    quarter: Quarter,
}

/// Internal discriminator used when searching a directory listing.
///
/// Daily indices encode the date in the filename (e.g., `company.YYYYMMDD.idx`). Quarterly
/// indices use a fixed base name (e.g., `company.idx` or `company.gz`) within a quarter folder.
#[derive(Debug, Clone, Copy)]
pub enum EdgarDate {
    Day(EdgarDay),
    Period(),
}

impl From<EdgarDay> for EdgarDate {
    fn from(day: EdgarDay) -> Self {
        EdgarDate::Day(day)
    }
}

impl From<EdgarPeriod> for EdgarDate {
    fn from(_period: EdgarPeriod) -> Self {
        EdgarDate::Period()
    }
}

impl EdgarPeriod {
    /// Creates a new EdgarPeriod (year must be >= 1994).
    pub fn new(year: i32, quarter: Quarter) -> Result<Self> {
        if year < 1994 {
            return Err(EdgarError::InvalidYear);
        }
        Ok(Self { year, quarter })
    }

    /// Returns the year of this period.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// Returns the quarter of this period.
    pub fn quarter(&self) -> Quarter {
        self.quarter
    }
}

impl Edgar {
    /// Returns `true` if the index file is gzipped (`.gz`).
    fn is_archive(filename: &str) -> bool {
        filename.ends_with(".gz")
    }

    /// Converts raw bytes into UTF-8 text, transparently decompressing `.gz` inputs.
    ///
    /// EDGAR hosts index files both as plain text (`.idx`) and gzipped (`.gz`). The parsing
    /// layer expects text, so this helper normalizes both variants into a `String`.
    async fn extract_archive(&self, content: Vec<u8>, filename: &str) -> Result<String> {
        if filename.ends_with(".gz") {
            let mut decoder = GzDecoder::new(&content[..]);
            let mut result = String::new();
            decoder.read_to_string(&mut result)?;
            Ok(result)
        } else {
            Ok(String::from_utf8(content)?)
        }
    }

    /// Downloads an index file as text.
    ///
    /// Callers tell this function whether the target is an archive so we can choose between
    /// `get_bytes()` (for `.gz`) and `get()` (for plain text). This keeps HTTP handling centralized
    /// and makes the rest of the index pipeline operate on strings.
    async fn download_file(&self, url: &str, is_archive: bool) -> Result<String> {
        if is_archive {
            let bytes = self.get_bytes(url).await?;
            self.extract_archive(bytes, url).await
        } else {
            self.get(url).await
        }
    }

    async fn download_and_parse_index(
        &self,
        url: &str,
        file_name: &str,
        index_type: Option<IndexType>,
    ) -> Result<Vec<IndexEntry>> {
        let is_archive = Self::is_archive(file_name);
        let content = self.download_file(url, is_archive).await?;

        // If no index_type provided, the parser will try to guess it
        let config = IndexConfig {
            index_type,
            ..Default::default()
        };

        let parser = IndexParser::new(config);
        Ok(parser.parse(content.as_bytes())?)
    }

    /// Picks the most appropriate index file from a directory listing.
    ///
    /// For daily indices, EDGAR includes the date in the filename (e.g., `company.20230815.idx`).
    /// For quarterly indices, the filename is stable within a quarter folder (e.g., `company.idx`).
    ///
    /// When both `.gz` and `.idx` are present, we prefer `.gz` first.
    fn find_index_file<'a>(
        items: &'a [DirectoryItem],
        date: impl Into<EdgarDate>,
        index_type: IndexType,
    ) -> Option<&'a DirectoryItem> {
        let prefix = index_type.as_str();
        let extensions = ["gz", "idx"]; // Priority order

        match date.into() {
            EdgarDate::Day(day) => {
                // Search for daily index files (e.g. "company.20230815.idx")
                let date_str = day.format_date();
                for ext in extensions {
                    let pattern = format!("{}.{}.{}", prefix, date_str, ext);
                    if let Some(item) = items
                        .iter()
                        .find(|i| i.name == pattern && i.type_ == ItemType::File)
                    {
                        return Some(item);
                    }
                }
            }
            EdgarDate::Period() => {
                // Search for quarterly index files (e.g. "company.gz")
                for ext in extensions {
                    let pattern = format!("{}.{}", prefix, ext);
                    if let Some(item) = items
                        .iter()
                        .find(|i| i.name == pattern && i.type_ == ItemType::File)
                    {
                        return Some(item);
                    }
                }
            }
        }
        None
    }

    /// Builds the `index.json` URL used to list available index files.
    ///
    /// The SEC exposes directory listings as JSON at predictable locations:
    /// - `.../<daily|full>-index/index.json` (top level)
    /// - `.../<daily|full>-index/<YEAR>/index.json`
    /// - `.../<daily|full>-index/<YEAR>/QTR<1-4>/index.json`
    ///
    /// This helper centralizes that formatting and keeps the validation logic in `fetch_index()`.
    fn build_index_url(
        &self,
        index_type: &str,
        year: Option<i32>,
        quarter: Option<i32>,
    ) -> Result<String> {
        let url = match (year, quarter) {
            (None, None) => {
                format!(
                    "{}/{}-index/index.json",
                    self.edgar_archives_url, index_type
                )
            }
            (Some(y), None) => {
                format!(
                    "{}/{}-index/{}/index.json",
                    self.edgar_archives_url, index_type, y
                )
            }
            (None, Some(q)) => {
                let current_year = chrono::Local::now().year();
                format!(
                    "{}/{}-index/{}/QTR{}/index.json",
                    self.edgar_archives_url, index_type, current_year, q
                )
            }
            (Some(y), Some(q)) => {
                format!(
                    "{}/{}-index/{}/QTR{}/index.json",
                    self.edgar_archives_url, index_type, y, q
                )
            }
        };
        Ok(url)
    }

    /// Fetches and parses a SEC `index.json` directory listing.
    ///
    /// This performs basic input validation (year >= 1994, quarter in 1..=4), then downloads and
    /// deserializes the listing into [`IndexResponse`].
    async fn fetch_index(
        &self,
        index_type: &str,
        year: Option<i32>,
        quarter: Option<i32>,
    ) -> Result<IndexResponse> {
        match (year, quarter) {
            (Some(y), _) if y < 1994 => Err(EdgarError::InvalidYear),
            (_, Some(q)) if q < 1 || q > 4 => Err(EdgarError::InvalidQuarter),
            _ => {
                let url = self.build_index_url(index_type, year, quarter)?;
                let response = self.get(&url).await?;
                Ok(serde_json::from_str(&response)?)
            }
        }
    }

    /// Applies `FilingOptions` filters to parsed index entries.
    ///
    /// This filter stage is intentionally simple: it operates on already-parsed `IndexEntry` values,
    /// matching form types (exact string match after trimming), CIKs, and then applying offset/limit.
    fn apply_filters(&self, mut entries: Vec<IndexEntry>, opts: &FilingOptions) -> Vec<IndexEntry> {
        // Filter by form types if specified
        if let Some(ref form_types) = opts.form_types {
            entries.retain(|entry| form_types.iter().any(|ft| ft == &entry.form_type.trim()));
        }

        // Filter by CIK if specified
        if let Some(ref ciks) = opts.ciks {
            entries.retain(|entry| ciks.contains(&entry.cik));
        }

        // Apply offset
        if let Some(offset) = opts.offset {
            entries = entries.into_iter().skip(offset).collect();
        }

        // Apply limit
        if let Some(limit) = opts.limit {
            entries.truncate(limit);
        }

        entries
    }
}

/// Operations for interacting with EDGAR index files.
///
/// The index endpoints are designed for “what was filed on a date/quarter?” workflows.
/// Both `get_daily_filings` and `get_period_filings` return parsed [`IndexEntry`] values
/// which can then be fed into your own ingestion/download pipeline.
///
/// Internally, edgarkit downloads either a plain-text `.idx` file or a gzipped `.gz` file,
/// then parses it using `parsers::index`.
///
/// # Examples
///
/// ```ignore
/// use edgarkit::{Edgar, EdgarDay, EdgarPeriod, FilingOptions, IndexOperations, Quarter};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("MyApp contact@example.com")?;
///
///     let day = EdgarDay::new(2023, 8, 15)?;
///     let opts = FilingOptions::new().with_form_type("10-K".to_string());
///     let daily = edgar.get_daily_filings(day, Some(opts)).await?;
///
///     let period = EdgarPeriod::new(2023, Quarter::Q3)?;
///     let quarterly = edgar.get_period_filings(period, None).await?;
///
///     println!("daily={}, quarterly={}", daily.len(), quarterly.len());
///     Ok(())
/// }
/// ```
#[async_trait]
impl IndexOperations for Edgar {
    /// Retrieves filings for a specific day
    ///
    /// This downloads the appropriate daily index file for the given day and returns the
    /// parsed entries. If `options` are provided, they are applied in-memory after parsing.
    ///
    /// # Arguments
    /// * `day` - The specific day to retrieve filings for
    /// * `options` - Optional filing options
    ///
    /// # Errors
    /// * `EdgarError::InvalidYear` if year < 1994
    /// * `EdgarError::NotFound` if no index file exists
    /// * `EdgarError::RequestError` for network issues
    async fn get_daily_filings(
        &self,
        day: EdgarDay,
        options: Option<FilingOptions>,
    ) -> Result<Vec<IndexEntry>> {
        let index = IndexType::default();
        let response = self
            .fetch_index("daily", Some(day.year()), Some(day.quarter().as_i32()))
            .await?;

        let index_file = Self::find_index_file(&response.directory.item, day, index)
            .ok_or_else(|| EdgarError::NotFound)?;

        let url = format!(
            "{}/daily-index/{}/QTR{}/{}",
            self.edgar_archives_url,
            day.year(),
            day.quarter().as_i32(),
            index_file.href
        );

        let mut entries = self
            .download_and_parse_index(&url, &index_file.name, Some(index))
            .await?;

        // Apply filters if provided
        if let Some(opts) = options {
            entries = self.apply_filters(entries, &opts);
        }

        Ok(entries)
    }

    /// Retrieves filings for a specific quarter
    ///
    /// This downloads the quarterly “full index” file for the given period and returns the
    /// parsed entries. If `options` are provided, they are applied in-memory after parsing.
    ///
    /// # Arguments
    /// * `period` - The year and quarter to retrieve filings for
    /// * `options` - Optional filing options
    ///
    /// # Errors
    /// * `EdgarError::InvalidYear` if year < 1994
    /// * `EdgarError::InvalidQuarter` if quarter is invalid
    /// * `EdgarError::NotFound` if no index file exists
    async fn get_period_filings(
        &self,
        period: EdgarPeriod,
        options: Option<FilingOptions>,
    ) -> Result<Vec<IndexEntry>> {
        let index = IndexType::default();
        let response = self
            .fetch_index("full", Some(period.year()), Some(period.quarter().as_i32()))
            .await?;

        let index_file = Self::find_index_file(&response.directory.item, period, index)
            .ok_or_else(|| EdgarError::NotFound)?;

        let url = format!(
            "{}/full-index/{}/QTR{}/{}",
            self.edgar_archives_url,
            period.year(),
            period.quarter().as_i32(),
            index_file.href
        );

        let mut entries = self
            .download_and_parse_index(&url, &index_file.name, Some(index))
            .await?;

        // Apply filters if provided
        if let Some(opts) = options {
            entries = self.apply_filters(entries, &opts);
        }

        Ok(entries)
    }

    /// Retrieves directory listing for daily indices.
    async fn daily_index(&self, period: Option<EdgarPeriod>) -> Result<IndexResponse> {
        match period {
            Some(p) => {
                self.fetch_index("daily", Some(p.year()), Some(p.quarter().as_i32()))
                    .await
            }
            None => self.fetch_index("daily", None, None).await,
        }
    }

    /// Retrieves directory listing for full (quarterly) indices.
    async fn full_index(&self, period: Option<EdgarPeriod>) -> Result<IndexResponse> {
        match period {
            Some(p) => {
                self.fetch_index("full", Some(p.year()), Some(p.quarter().as_i32()))
                    .await
            }
            None => self.fetch_index("full", None, None).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_index_file() {
        let items = vec![
            DirectoryItem {
                last_modified: NaiveDateTime::parse_from_str(
                    "08/15/2023 12:00:00 AM",
                    "%m/%d/%Y %I:%M:%S %p",
                )
                .unwrap(),
                name: "company.20230815.idx".to_string(),
                type_: ItemType::File,
                href: "company.20230815.idx".to_string(),
                size: "1000".to_string(),
            },
            DirectoryItem {
                last_modified: NaiveDateTime::parse_from_str(
                    "08/15/2023 12:00:00 AM",
                    "%m/%d/%Y %I:%M:%S %p",
                )
                .unwrap(),
                name: "company.idx".to_string(),
                type_: ItemType::File,
                href: "company.idx".to_string(),
                size: "2000".to_string(),
            },
        ];

        let day = EdgarDay::new(2023, 8, 15).unwrap();
        let file = Edgar::find_index_file(&items, day, IndexType::Company).unwrap();
        assert_eq!(file.name, "company.20230815.idx");
        assert_eq!(file.href, "company.20230815.idx");
        assert_eq!(file.size, "1000");
        assert_eq!(file.type_, ItemType::File);

        let period = EdgarPeriod::new(2023, Quarter::Q3).unwrap();
        let file = Edgar::find_index_file(&items, period, IndexType::Company).unwrap();
        assert_eq!(file.name, "company.idx");
        assert_eq!(file.href, "company.idx");
        assert_eq!(file.size, "2000");
        assert_eq!(file.type_, ItemType::File);
    }

    #[test]
    fn test_daily_index_invalid_year() {
        let period = EdgarPeriod::new(1993, Quarter::Q1);
        assert!(matches!(period, Err(EdgarError::InvalidYear)));
    }

    #[test]
    fn test_period_invalid_year() {
        let period = EdgarPeriod::new(1993, Quarter::Q1);
        assert!(matches!(period, Err(EdgarError::InvalidYear)));
    }
}
