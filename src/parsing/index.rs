use super::utils::deserialize_str_to_u64;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::str::FromStr;

#[derive(Default)]
pub struct IndexConfig {
    pub field_widths: Option<Vec<usize>>,
    pub delimiter: Option<char>,
    pub max_entries: Option<usize>,
    pub index_type: Option<IndexType>,
}

pub struct IndexParser {
    config: IndexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub company_name: String,
    pub form_type: String,
    #[serde(deserialize_with = "deserialize_str_to_u64")]
    pub cik: u64,
    pub date_filed: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexType {
    Company,
    Crawler,
    Master,
}

impl IndexType {
    pub const VARIANTS: &'static [(&'static str, IndexType)] = &[
        ("company", IndexType::Company),
        ("crawler", IndexType::Crawler),
        ("master", IndexType::Master),
    ];

    pub fn as_str(&self) -> &'static str {
        Self::VARIANTS
            .iter()
            .find(|(_, variant)| variant == self)
            .map(|(s, _)| *s)
            .unwrap_or("company")
    }
}

/// Converts a string slice to an `IndexType` enumeration.
impl FromStr for IndexType {
    type Err = crate::EdgarError;

    fn from_str(s: &str) -> Result<Self> {
        Self::VARIANTS
            .iter()
            .find(|(pattern, _)| s.to_lowercase().contains(pattern))
            .map(|(_, variant)| *variant)
            .ok_or_else(|| crate::EdgarError::InvalidFormat("Unknown index type".to_string()))
    }
}

impl Default for IndexType {
    /// Master is the most stable format so far. It uses '|' as delimiter.
    /// Not sure why other formats fail. For now, default to master.
    fn default() -> Self {
        Self::Master
    }
}

/// A parser for various types of EDGAR index files.
///
/// The `IndexParser` is capable of parsing different types of SEC EDGAR index files, including:
/// - Company Index files
/// - Crawler Index files
/// - Master Index files
///
/// It supports both fixed-width and delimiter-based parsing strategies, which can be configured
/// through the `IndexConfig`.
///
/// # Examples
///
/// ```
/// use edgarkit::parsing::index::{IndexParser, IndexConfig};
///
/// let config = IndexConfig::default();
/// let parser = IndexParser::new(config);
/// ```
///
/// # Features
///
/// - Automatic index type detection
/// - Configurable field widths and delimiters
/// - Header line skipping
/// - Maximum entry limit support
///
/// # Configuration
///
/// The parser can be configured using `IndexConfig` to specify:
/// - Custom field widths
/// - Custom delimiters
/// - Maximum number of entries to parse
///
/// # Parsing Strategy
///
/// The parser automatically detects the index type from the input and applies the appropriate
/// parsing strategy:
/// - Fixed-width parsing for Company and Crawler indices (default)
/// - Delimiter-based parsing for Master indices (using '|' as delimiter)
/// - Custom parsing based on configuration
impl IndexParser {
    const ARCHIVES_PREFIX: &'static str = "https://www.sec.gov/Archives/";

    /// Creates a new `IndexParser` with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration settings for the parser
    ///
    pub fn new(config: IndexConfig) -> Self {
        Self { config }
    }

    /// Detects the type of index from the first 10 lines of a reader.
    ///
    /// # Parameters
    ///
    /// * `reader`: A mutable reference to a reader that implements the `BufRead` trait.
    ///
    /// # Returns
    ///
    /// * `Result<IndexType>`: A result containing the detected index type.
    ///   - `Ok(IndexType::Company)`: If the first 10 lines contain the company index header.
    ///   - `Ok(IndexType::Crawler)`: If the first 10 lines contain the crawler index header.
    ///   - `Ok(IndexType::Master)`: If the first 10 lines contain the master index header.
    ///   - `Ok(IndexType::Crawler)`: If none of the above headers are found, the default index type is crawler.
    ///   - `Err(err)`: If an error occurs while reading from the reader.
    fn detect_type<R: BufRead>(&self, reader: &mut R) -> Result<IndexType> {
        for line in reader.lines().take(10) {
            let line = line?;
            if line.contains("Daily Index of EDGAR Dissemination Feed by Company Name") {
                return Ok(IndexType::Company);
            }
            if line.contains("Daily Crawler Index") {
                return Ok(IndexType::Crawler);
            }
            if line.contains("Master Index") || line.contains("XBRL Index") {
                return Ok(IndexType::Master);
            }
        }

        // Fall back to crawler index type by default
        Ok(IndexType::Crawler)
    }

    /// Skips header lines from a reader until a separator line is found.
    ///
    /// The separator line is detected by looking for a line containing only dashes ("---").
    /// If no separator line is found after `MAX_TRIES` iterations, the function returns.
    ///
    /// # Arguments
    ///
    /// * `reader` - A mutable reference to a reader that implements the `BufRead` trait.
    ///
    fn skip_header_lines<R: BufRead>(&self, reader: &mut R) {
        const MAX_TRIES: usize = 50;
        let mut lines = reader.lines();

        // Try to find separator line
        for _i in 0..MAX_TRIES {
            if let Some(Ok(line)) = lines.next() {
                if line.contains("---") {
                    return;
                }
            } else {
                break;
            }
        }
    }

    /// This function reads the input, detects the index type, skips header lines,
    /// and processes each line to create `IndexEntry` objects.
    ///
    /// # Arguments
    ///
    /// * `reader` - A mutable reference implementing `BufRead` trait, providing the index file content.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<IndexEntry>>` - A Result containing a vector of `IndexEntry` objects if successful,
    ///   or an error if parsing fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The index type cannot be detected
    /// - There's an I/O error while reading the input
    /// - Line parsing fails
    pub fn parse<R: BufRead>(&self, mut reader: R) -> Result<Vec<IndexEntry>> {
        // Use provided type or detect
        let index_type = match self.config.index_type {
            Some(t) => t,
            None => self.detect_type(&mut reader)?,
        };

        // Skip header until a line containing only dashes ("---") found, 50 lines max
        self.skip_header_lines(&mut reader);

        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() && !line.starts_with("---") {
                if let Some(entry) = self.parse_line(&line, &index_type)? {
                    entries.push(entry);
                }
            }
        }

        if let Some(max) = self.config.max_entries {
            entries.truncate(max);
        }

        Ok(entries)
    }

    /// Parses a single line from the index file into an `IndexEntry`.
    ///
    /// # Arguments
    ///
    /// * `line` - A single line from the index file
    /// * `index_type` - The type of index being parsed
    ///
    /// # Returns
    ///
    /// * `Result<Option<IndexEntry>>` - The parsed entry, or None if the line is empty
    ///
    fn parse_line(&self, line: &str, index_type: &IndexType) -> Result<Option<IndexEntry>> {
        if line.trim().is_empty() {
            return Ok(None);
        }

        // Get fields based on configuration or default widths
        let fields = if let Some(widths) = &self.config.field_widths {
            self.parse_fixed_width(line, widths)
        } else if let Some(delimiter) = self.config.delimiter {
            line.split(delimiter)
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            // Use different parsing strategies based on index type
            match index_type {
                IndexType::Company | IndexType::Crawler => {
                    self.parse_fixed_width(line, &[62, 12, 12, 12, 74])
                }
                IndexType::Master => {
                    // Split by | for master/XBRL index
                    line.split('|').map(|s| s.trim().to_string()).collect()
                }
            }
        };

        if fields.len() < 4 {
            return Ok(None);
        }

        // Create entry based on index type and field order
        let (company_name, form_type, cik_str, date_filed, path_or_url) = match index_type {
            IndexType::Company => (
                fields[0].clone(),
                fields[1].clone(),
                fields[2].trim().trim_start_matches('0'),
                fields[3].clone(),
                fields.get(4).map(|s| Self::ARCHIVES_PREFIX.to_string() + s),
            ),
            IndexType::Crawler => (
                fields[0].clone(),
                fields[1].clone(),
                fields[2].trim().trim_start_matches('0'),
                fields[3].clone(),
                fields.get(4).cloned(),
            ),
            IndexType::Master => (
                fields[1].clone(),
                fields[2].clone(),
                fields[0].trim().trim_start_matches('0'),
                fields[3].clone(),
                fields.get(4).map(|s| Self::ARCHIVES_PREFIX.to_string() + s),
            ),
        };

        let cik = cik_str
            .parse::<u64>()
            .map_err(|_| crate::EdgarError::InvalidFormat(format!("Invalid CIK: {}", cik_str)))?;

        Ok(Some(IndexEntry {
            company_name,
            form_type,
            cik,
            date_filed,
            url: path_or_url.unwrap_or_default(),
        }))
    }

    /// Parses a fixed-width formatted line into fields.
    ///
    /// # Arguments
    ///
    /// * `line` - The line to parse
    /// * `widths` - Array of field widths
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - The parsed fields
    ///
    fn parse_fixed_width(&self, line: &str, widths: &[usize]) -> Vec<String> {
        let mut result = Vec::new();
        let mut start = 0;

        for &width in widths {
            if start >= line.len() {
                break;
            }
            let end = (start + width).min(line.len());
            result.push(line[start..end].trim().to_string());
            start += width;
        }

        // Add remaining content as the last field if any
        if start < line.len() {
            result.push(line[start..].trim().to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_index_type_conversion() {
        // Test from_str
        assert_eq!(
            IndexType::from_str("company_index").unwrap(),
            IndexType::Company
        );
        assert_eq!(
            IndexType::from_str("crawler_data").unwrap(),
            IndexType::Crawler
        );
        assert_eq!(
            IndexType::from_str("master_list").unwrap(),
            IndexType::Master
        );
        assert!(IndexType::from_str("invalid").is_err());

        // Test as_str
        assert_eq!(IndexType::Company.as_str(), "company");
        assert_eq!(IndexType::Crawler.as_str(), "crawler");
        assert_eq!(IndexType::Master.as_str(), "master");
    }

    #[test]
    fn test_invalid_input() {
        let parser = IndexParser::new(IndexConfig::default());
        let result = parser.parse(BufReader::new("invalid content".as_bytes()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_master_index_with_pipe_delimiter() {
        let parser = IndexParser::new(IndexConfig::default());
        let content = r#"
Description:           Master Index of EDGAR Dissemination Feed
Last Data Received:    March 31, 2023
Comments:              webmaster@sec.gov
Anonymous FTP:         ftp://ftp.sec.gov/edgar/
Cloud HTTP:            https://www.sec.gov/Archives/

CIK|Company Name|Form Type|Date Filed|Filename
--------------------------------------------------------------------------------
1000045|NICHOLAS FINANCIAL INC|10-Q|2023-02-14|edgar/data/1000045/0000950170-23-002704.txt
"#;
        let reader = BufReader::new(content.as_bytes());
        let entries = parser.parse(reader).unwrap();

        assert!(!entries.is_empty());
        let entry = &entries[0];
        assert_eq!(entry.cik, 1000045);
        assert_eq!(entry.company_name.trim(), "NICHOLAS FINANCIAL INC");
        assert_eq!(
            entry.url,
            "https://www.sec.gov/Archives/edgar/data/1000045/0000950170-23-002704.txt"
        )
    }

    #[test]
    fn test_parse_master_index_line() {
        let parser = IndexParser::new(IndexConfig::default());
        let line = "1000045|NICHOLAS FINANCIAL INC|10-Q|2023-02-14|edgar/data/1000045/0000950170-23-002704.txt";
        let index_type = IndexType::Master;

        let entry = parser.parse_line(line, &index_type).unwrap().unwrap();

        assert_eq!(entry.cik, 1000045);
        assert_eq!(entry.company_name.trim(), "NICHOLAS FINANCIAL INC");
        assert_eq!(entry.form_type.trim(), "10-Q");
        assert_eq!(entry.date_filed, "2023-02-14");
        assert_eq!(
            entry.url,
            "https://www.sec.gov/Archives/edgar/data/1000045/0000950170-23-002704.txt"
        );
    }

    #[test]
    fn test_parse_company_index_line() {
        let parser = IndexParser::new(IndexConfig::default());
        let line = "3J LLC                                                        D           1975393     20230703    edgar/data/1975393/0001975393-23-000001.txt";
        let index_type = IndexType::Company;

        let entry = parser.parse_line(line, &index_type).unwrap().unwrap();

        assert_eq!(entry.company_name.trim(), "3J LLC");
        assert_eq!(entry.form_type.trim(), "D");
        assert_eq!(entry.cik, 1975393);
        assert_eq!(entry.date_filed, "20230703");
        assert_eq!(
            entry.url,
            "https://www.sec.gov/Archives/edgar/data/1975393/0001975393-23-000001.txt"
        );
    }

    #[test]
    fn test_parse_crawler_index_line() {
        let parser = IndexParser::new(IndexConfig::default());
        let line = "EXAMPLE COMPANY                                               10-K        1234567     2023-07-03  https://www.sec.gov/Archives/edgar/data/1234567/000123456723000001.txt";
        let index_type = IndexType::Crawler;

        let entry = parser.parse_line(line, &index_type).unwrap().unwrap();

        assert_eq!(entry.company_name.trim(), "EXAMPLE COMPANY");
        assert_eq!(entry.form_type.trim(), "10-K");
        assert_eq!(entry.cik, 1234567);
        assert_eq!(entry.date_filed, "2023-07-03");
        assert_eq!(
            entry.url,
            "https://www.sec.gov/Archives/edgar/data/1234567/000123456723000001.txt"
        );
    }
}
