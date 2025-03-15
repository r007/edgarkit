use super::error::{EdgarError, Result};
use super::options::FilingOptions;
use super::traits::IndexOperations;
use super::Edgar;
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
    pub directory: Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Directory {
    pub item: Vec<DirectoryItem>,
    pub name: String,
    #[serde(rename = "parent-dir")]
    pub parent_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ItemType {
    Dir,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryItem {
    #[serde(rename = "last-modified")]
    #[serde(with = "edgar_date_format")]
    pub last_modified: NaiveDateTime,
    pub name: String,
    #[serde(rename = "type")]
    pub type_: ItemType,
    pub href: String,
    pub size: String,
}

/// A module providing custom serialization and deserialization for dates in Edgar format.
///
/// The format used is "%m/%d/%Y %I:%M:%S %p" (example: "12/31/2023 11:59:59 PM")
///
/// # Functions
///
/// * `deserialize` - Deserializes a string in Edgar date format into a `NaiveDateTime`
/// * `serialize` - Serializes a `NaiveDateTime` into a string using Edgar date format
///
/// # Example
///
/// ```
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Document {
///     #[serde(with = "edgar_date_format")]
///     filing_date: NaiveDateTime
/// }
/// ```
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

/// Represents a fiscal quarter in the EDGAR filing system
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

    /// Converts Quarter to its integer representation (1-4)
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Represents a specific day in EDGAR's filing system
///
/// All dates must be from 1994 or later, as this marks the
/// beginning of electronic filings in the EDGAR system.
#[derive(Debug, Clone, Copy)]
pub struct EdgarDay {
    year: i32,
    month: u32,
    day: u32,
}

/// Represents a day in the EDGAR filing system's timeline
///
/// # Fields
/// * `year` - The year (must be 1994 or later)
/// * `month` - The month (1-12)
/// * `day` - The day of the month (1-31)
///
/// # Methods
/// * `new()` - Creates a new EdgarDay instance with validation
/// * `format_date()` - Formats the date as YYYYMMDD string
/// * `quarter()` - Returns the fiscal quarter for this date
/// * `year()` - Returns the year
///
/// # Errors
/// Returns an error if:
/// * Year is before 1994
/// * Month is not 1-12
/// * Day is not 1-31
///
/// # Examples
/// ```
/// let edgar_day = EdgarDay::new(2023, 12, 25)?;
/// assert_eq!(edgar_day.format_date(), "20231225");
/// assert_eq!(edgar_day.quarter(), Quarter::Q4);
/// ```
impl EdgarDay {
    /// Creates a new EdgarDay with validation
    ///
    /// # Arguments
    /// * `year` - Year (must be 1994 or later)
    /// * `month` - Month (1-12)
    /// * `day` - Day (1-31)
    ///
    /// # Errors
    /// * `EdgarError::InvalidYear` if year < 1994
    /// * `EdgarError::InvalidMonth` if month is invalid
    /// * `EdgarError::InvalidDay` if day is invalid
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

    /// Formats the date as YYYYMMDD string
    ///
    /// # Returns
    /// String in format "YYYYMMDD" (e.g., "20230815")
    pub fn format_date(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    /// Gets the fiscal quarter for this date
    ///
    /// # Returns
    /// Quarter enum representing the fiscal quarter
    pub fn quarter(&self) -> Quarter {
        Quarter::from_month(self.month).unwrap()
    }

    /// Gets the year
    pub fn year(&self) -> i32 {
        self.year
    }
}

/// Represents a fiscal period (year + quarter) in EDGAR
#[derive(Debug, Clone, Copy)]
pub struct EdgarPeriod {
    year: i32,
    quarter: Quarter,
}

#[derive(Debug, Clone, Copy)]
pub enum EdgarDate {
    Day(EdgarDay),
    Period(), // Unused for now
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

/// Represents a specific period in EDGAR filings, consisting of a year and quarter
///
/// # Examples
/// ```
/// use your_crate::EdgarPeriod;
/// use your_crate::Quarter;
///
/// let period = EdgarPeriod::new(2023, Quarter::Q1).unwrap();
/// assert_eq!(period.year(), 2023);
/// ```
///
impl EdgarPeriod {
    /// Creates a new EdgarPeriod instance
    ///
    /// # Arguments
    /// * `year` - The year of the period (must be 1994 or later)
    /// * `quarter` - The quarter of the period
    ///
    /// # Returns
    /// * `Result<EdgarPeriod>` - Ok with the new instance if valid, Err if year is before 1994
    ///
    /// # Errors
    /// Returns `EdgarError::InvalidYear` if the year is before 1994
    pub fn new(year: i32, quarter: Quarter) -> Result<Self> {
        if year < 1994 {
            return Err(EdgarError::InvalidYear);
        }
        Ok(Self { year, quarter })
    }

    /// Returns the year of this EdgarPeriod
    pub fn year(&self) -> i32 {
        self.year
    }

    /// Returns the quarter of this EdgarPeriod
    pub fn quarter(&self) -> Quarter {
        self.quarter
    }
}

/// Helper module for interacting with the EDGAR (Electronic Data Gathering, Analysis, and Retrieval) system.
///
/// This implementation provides functionality to:
/// - Download and extract archived (.gz) and non-archived index files
/// - Parse index files into structured data
/// - Find specific index files based on date and type
/// - Build and validate URLs for accessing EDGAR indices
///
/// # Methods
///
/// - `is_archive`: Determines if a file is an archive based on its extension
/// - `extract_archive`: Extracts content from a .gz archive
/// - `download_file`: Downloads and optionally extracts a file from EDGAR
/// - `find_index_file`: Locates an index file in a directory listing
/// - `build_index_url`: Constructs URLs for EDGAR index files
/// - `fetch_index`: Retrieves index data from EDGAR
/// - `download_and_parse_index`: Downloads and parses an index file
/// - `apply_filters`: Filters index entries based on options
///
/// # Examples
///
/// ```
/// # use your_crate::Edgar;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let edgar = Edgar::new();
///
/// // Download and parse a daily index
/// let entries = edgar.download_and_parse_index(
///     "https://www.sec.gov/Archives/edgar/daily-index/2023/QTR3/company.20230815.idx",
///     "company.20230815.idx"
/// ).await?;
///
/// // Fetch quarterly index information
/// let index_response = edgar.fetch_index("company", Some(2023), Some(3)).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Note
///
/// - EDGAR archives are available from 1994 onwards
/// - Quarterly data is divided into QTR1 through QTR4
/// - Index files can be either compressed (.gz) or uncompressed (.idx)
/// - The system supports both daily and quarterly index retrievals
///
/// # Errors
///
/// Returns error in cases of:
/// - Invalid year (< 1994)
/// - Invalid quarter (not 1-4)
/// - Network issues
/// - File parsing problems
/// - Invalid UTF-8 encoding in response content
impl Edgar {
    /// Only consider .gz files as archives
    fn is_archive(filename: &str) -> bool {
        filename.ends_with(".gz")
    }

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

/// Operations for interacting with EDGAR index files
///
/// This trait defines the core operations for retrieving filing indices from the SEC EDGAR system.
/// It supports both daily and quarterly (periodic) filing retrievals, as well as directory listings.
///
/// # Index Types
///
/// - Daily Index: Contains filings for a specific day
/// - Full Index: Contains quarterly compilations of filings
/// - Company Index: Most commonly used, contains company filing information
/// - Crawler Index: Alternative format with additional metadata
/// - Master Index: Comprehensive quarterly index
///
/// # Examples
///
/// ```rust
/// use edgar::{Edgar, IndexOperations, EdgarDay, EdgarPeriod, Quarter, IndexType};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let edgar = Edgar::new("example@email.com")?;
///
///     // Get daily filings
///     let day = EdgarDay::new(2023, 8, 15)?;
///     let daily = edgar.get_daily_filings(day, Some(IndexType::Company)).await?;
///
///     // Get quarterly filings
///     let period = EdgarPeriod::new(2023, Quarter::Q3)?;
///     let quarterly = edgar.get_period_filings(period, Some(IndexType::Company)).await?;
///
///     // Get directory listing
///     let listing = edgar.daily_index(Some(period)).await?;
///     Ok(())
/// }
/// ```
#[async_trait]
impl IndexOperations for Edgar {
    /// Retrieves filings for a specific day
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

    /// Retrieves directory listing for daily indices
    ///
    /// # Arguments
    /// * `period` - Optional period to get listing for (if None, returns root listing)
    ///
    /// # Returns
    /// Directory structure containing available index files and subdirectories
    async fn daily_index(&self, period: Option<EdgarPeriod>) -> Result<IndexResponse> {
        match period {
            Some(p) => {
                self.fetch_index("daily", Some(p.year()), Some(p.quarter().as_i32()))
                    .await
            }
            None => self.fetch_index("daily", None, None).await,
        }
    }

    /// Retrieves directory listing for full (quarterly) indices
    ///
    /// # Arguments
    /// * `period` - Optional period to get listing for (if None, returns root listing)
    ///
    /// # Returns
    /// Directory structure containing available index files and subdirectories
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
    use std::fs;

    const FULL_INDEX_FIXTURE: &str = "fixtures/index/full-index.json";
    const FULL_INDEX_QTR_FIXTURE: &str = "fixtures/index/full-index-qtr.json";
    const DAILY_INDEX_FIXTURE: &str = "fixtures/index/daily-index.json";
    const DAILY_INDEX_2023_FIXTURE: &str = "fixtures/index/daily-index-2023.json";

    #[test]
    fn test_parse_full_index() {
        let content = fs::read_to_string(FULL_INDEX_FIXTURE).unwrap();
        let response: IndexResponse = serde_json::from_str(&content).unwrap();

        assert_eq!(response.directory.name, "full-index/");
        assert_eq!(response.directory.parent_dir, "../");

        let first_item = &response.directory.item[0];
        assert_eq!(first_item.name, "1993");
        assert_eq!(first_item.type_, ItemType::Dir);

        // Test date parsing
        assert_eq!(
            first_item
                .last_modified
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            "2025-01-25 01:00:21"
        );
    }

    #[test]
    fn test_parse_quarter_index() {
        let content = fs::read_to_string(FULL_INDEX_QTR_FIXTURE).unwrap();
        let response: IndexResponse = serde_json::from_str(&content).unwrap();

        let item = response
            .directory
            .item
            .iter()
            .find(|i| i.name == "company.idx")
            .unwrap();

        assert_eq!(item.type_, ItemType::File);
        assert_eq!(item.size, "52453 KB");
    }

    #[test]
    fn test_parse_daily_index() {
        let content = fs::read_to_string(DAILY_INDEX_FIXTURE).unwrap();
        let response: IndexResponse = serde_json::from_str(&content).unwrap();

        assert!(response.directory.item.len() > 0);

        // Test year directory
        let year_2023 = response
            .directory
            .item
            .iter()
            .find(|i| i.name == "2023")
            .unwrap();

        assert_eq!(year_2023.type_, ItemType::Dir);
        assert_eq!(year_2023.href, "2023/");
        assert_eq!(year_2023.size, "743909 KB");
    }

    #[test]
    fn test_parse_daily_index_year() {
        let content = fs::read_to_string(DAILY_INDEX_2023_FIXTURE).unwrap();
        let response: IndexResponse = serde_json::from_str(&content).unwrap();

        // Test quarters
        let quarters: Vec<_> = response
            .directory
            .item
            .iter()
            .map(|i| i.name.as_str())
            .collect();

        assert_eq!(quarters, vec!["QTR1", "QTR2", "QTR3", "QTR4"]);

        // Test specific quarter
        let qtr1 = response
            .directory
            .item
            .iter()
            .find(|i| i.name == "QTR1")
            .unwrap();

        assert_eq!(qtr1.type_, ItemType::Dir);
        assert_eq!(qtr1.href, "QTR1/");
        assert_eq!(qtr1.size, "16 KB");
    }

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

    #[tokio::test]
    async fn test_get_daily_filings() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let entries = edgar
            .get_daily_filings(EdgarDay::new(2023, 8, 1).unwrap(), None)
            .await
            .unwrap();
        assert!(!entries.is_empty());

        // Test entry fields
        let entry = &entries[0];
        assert!(entry.cik > 0);
        assert!(!entry.company_name.is_empty());
        assert!(!entry.form_type.is_empty());
        assert!(!entry.url.is_empty());
    }

    #[tokio::test]
    async fn test_get_periodic_filings() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let entries = edgar
            .get_period_filings(EdgarPeriod::new(2023, Quarter::Q1).unwrap(), None)
            .await
            .unwrap();
        assert!(!entries.is_empty());

        // Test entry fields
        let entry = &entries[0];
        assert!(entry.cik > 0);
        assert!(!entry.company_name.is_empty());
        assert!(!entry.form_type.is_empty());
        assert!(!entry.url.is_empty());
    }

    #[tokio::test]
    async fn test_filing_options() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();

        let options = FilingOptions::new()
            .with_form_types(vec!["10-K".to_string(), "10-Q".to_string()])
            .with_ciks(vec![1234567])
            .with_offset(5)
            .with_limit(10);

        let day = EdgarDay::new(2023, 8, 15).unwrap();
        let entries = edgar
            .get_daily_filings(day, Some(options.clone()))
            .await
            .unwrap();

        assert!(entries.iter().all(|e| e.cik == 1234567));
        assert!(entries
            .iter()
            .all(|e| ["10-K", "10-Q"].contains(&e.form_type.trim())));
        assert!(entries.len() <= 10);
    }

    #[tokio::test]
    async fn test_daily_index_invalid_year() {
        let period = EdgarPeriod::new(1993, Quarter::Q1);
        assert!(matches!(period, Err(EdgarError::InvalidYear)));
    }

    #[tokio::test]
    async fn test_daily_index_current_year() {
        let edgar = Edgar::new("test_agent example@example.com").unwrap();
        let result = edgar.daily_index(None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_period_invalid_year() {
        let period = EdgarPeriod::new(1993, Quarter::Q1);
        assert!(matches!(period, Err(EdgarError::InvalidYear)));
    }
}
