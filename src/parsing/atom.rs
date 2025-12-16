//! Atom feed parser module.
//!
//! This module provides functionality to parse Atom feeds into structured data.
//! It supports configurable parsing options such as following links, limiting entries,
//! and filtering by categories.
#[cfg(feature = "atom")]
use crate::Result;
#[cfg(feature = "atom")]
use quick_xml::{Reader, de::from_reader};
#[cfg(feature = "atom")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "atom")]
pub struct AtomParser {
    config: AtomConfig,
}

/// Configuration options for Atom feed parsing.
#[derive(Default)]
#[cfg(feature = "atom")]
pub struct AtomConfig {
    /// Whether to follow links in the feed
    pub follow_links: bool,

    /// Optional limit on the number of entries to parse
    pub max_entries: Option<usize>,

    /// List of categories to filter entries by
    pub filter_categories: Vec<String>,
}

/// Represents an Atom feed document from SEC EDGAR.
///
/// Atom feeds are used by EDGAR to distribute filing information in a structured,
/// machine-readable format. They include metadata about the feed itself plus a
/// collection of entries representing individual filings or updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg(feature = "atom")]
pub struct AtomDocument {
    /// Author information for the feed (typically SEC or the filing company).
    #[serde(default)]
    pub author: Option<Author>,

    /// Detailed company information including address, SIC code, and corporate details.
    #[serde(rename = "company-info", default)]
    pub company_info: Option<CompanyInfo>,

    /// Human-readable title describing the feed's content.
    pub title: String,

    /// Collection of links related to this feed (self, alternate, etc.).
    #[serde(rename = "link", default)]
    pub links: Vec<Link>,

    /// Optional textual description providing context about the feed.
    pub description: Option<String>,

    /// Timestamp of the feed's last update (ISO 8601 format).
    pub updated: String,

    /// Individual filing entries contained in this feed.
    #[serde(rename = "entry", default)]
    pub entries: Vec<AtomEntry>,
}

/// Comprehensive company information included in EDGAR Atom feeds.
///
/// Provides corporate identification, location, and classification details
/// for the company that owns or is the subject of the feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg(feature = "atom")]
pub struct CompanyInfo {
    /// Business and mailing addresses for the company.
    pub addresses: Addresses,

    /// Standard Industrial Classification (SIC) code.
    pub assigned_sic: String,

    /// Human-readable description of the SIC category.
    pub assigned_sic_desc: String,

    /// Central Index Key - SEC's unique identifier for this filer.
    pub cik: String,

    /// Official company name as registered with the SEC.
    pub conformed_name: String,

    /// Company's fiscal year end date (MMDD format).
    pub fiscal_year_end: String,

    /// Two-letter state code where the company is located.
    pub state_location: String,
}

/// Container for one or more company addresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Addresses {
    /// List of addresses (business, mailing, etc.).
    pub address: Vec<Address>,
}

/// A single address record for a company.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Address {
    /// Type of address ("business", "mailing", etc.).
    #[serde(rename = "@type")]
    pub address_type: String,

    /// City name.
    pub city: String,

    /// Two-letter state code.
    pub state: String,

    /// Primary street address line.
    pub street1: String,

    /// Secondary street address line (suite, floor, etc.).
    pub street2: Option<String>,

    /// ZIP or postal code.
    pub zip: String,
}

/// A single entry in an Atom feed representing a filing or update.
///
/// Each entry corresponds to an individual SEC filing with metadata about
/// the filing type, content, publication date, and access links.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct AtomEntry {
    /// Brief title or description of the filing.
    pub title: String,

    /// Collection of links to different representations of this filing.
    #[serde(rename = "link", default)]
    pub links: Vec<Link>,

    /// Primary link extracted from the links collection.
    #[serde(skip)]
    pub link: String,

    /// When the filing was first published (ISO 8601).
    pub published: Option<String>,

    /// Alternative publication date field (RSS-style format).
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,

    /// When this entry was last modified (ISO 8601).
    pub updated: Option<String>,

    /// Unique identifier for this entry (typically includes accession number).
    pub id: String,

    /// Detailed filing content and metadata.
    #[serde(rename = "content")]
    pub content: Option<Content>,

    /// Brief textual summary of the filing.
    pub description: Option<String>,

    /// Author or submitter information.
    pub author: Option<Author>,

    /// Categorization information (form type, tags, etc.).
    #[serde(default)]
    pub category: Option<Category>,
}

/// Detailed content and metadata for a filing entry.
///
/// Contains SEC-specific fields like accession numbers, file numbers,
/// and links to various document representations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Content {
    /// MIME type of the content (e.g., "text/html").
    #[serde(rename = "@type")]
    pub content_type: Option<String>,

    /// Textual content or description.
    #[serde(rename = "$text")]
    pub text: Option<String>,

    /// SEC accession number (unique filing identifier).
    #[serde(rename = "accession-number")]
    pub accession_number: Option<String>,

    /// Securities Act under which the filing was made.
    pub act: Option<String>,

    /// Whether this is an amendment to a previous filing.
    pub amend: Option<String>,

    /// SEC-assigned file number.
    #[serde(rename = "file-number")]
    pub file_number: Option<String>,

    /// Link to file number search results.
    #[serde(rename = "file-number-href")]
    pub file_number_href: Option<String>,

    /// Date the filing was submitted.
    #[serde(rename = "filing-date")]
    pub filing_date: Option<String>,

    /// Direct link to the filing document.
    #[serde(rename = "filing-href")]
    pub filing_href: Option<String>,

    /// Type of form ("10-K", "8-K", etc.).
    #[serde(rename = "filing-type")]
    pub filing_type: Option<String>,

    /// Film number assigned by the SEC.
    #[serde(rename = "film-number")]
    pub film_number: Option<String>,

    /// Human-readable form name.
    #[serde(rename = "form-name")]
    pub form_name: Option<String>,

    /// Size of the filing document.
    pub size: Option<String>,

    /// Link to XBRL data if available.
    #[serde(rename = "xbrl_href")]
    pub xbrl_href: Option<String>,

    /// Description of items covered (for 8-K filings).
    #[serde(rename = "items-desc")]
    pub items_desc: Option<String>,
}

/// A hyperlink with optional relationship and type information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Link {
    /// Target URL of the link.
    #[serde(rename = "@href")]
    pub href: String,

    /// Relationship type ("self", "alternate", "related", etc.).
    #[serde(rename = "@rel")]
    pub rel: Option<String>,

    /// MIME type of the linked resource.
    #[serde(rename = "@type")]
    pub link_type: Option<String>,
}

/// Author or publisher information for a feed or entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Author {
    /// Name of the author or organization.
    pub name: String,

    /// Contact email address.
    pub email: Option<String>,
}

/// Category or classification information for an entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Category {
    /// Category term or identifier (often the form type).
    #[serde(rename = "@term")]
    pub term: String,

    /// URI defining the categorization scheme.
    #[serde(rename = "@scheme")]
    pub scheme: Option<String>,

    /// Human-readable label for the category.
    #[serde(rename = "@label")]
    pub label: Option<String>,
}

#[cfg(feature = "atom")]
impl AtomEntry {
    pub fn get_primary_link(&self) -> String {
        self.links
            .iter()
            .find(|l| l.rel.as_deref() == Some("alternate"))
            .or_else(|| self.links.first())
            .map(|l| l.href.clone())
            .unwrap_or_default()
    }
}

/// Represents an Atom feed parser with configurable options.
///
/// # Example
/// ```
/// use edgarkit::parsing::atom::{AtomParser, AtomConfig};
///
/// let config = AtomConfig {
///     follow_links: false,
///     max_entries: Some(10),
///     filter_categories: vec!["tech".to_string()],
/// };
/// let parser = AtomParser::new(config);
/// ```
#[cfg(feature = "atom")]
impl AtomParser {
    pub fn new(config: AtomConfig) -> Self {
        Self { config }
    }

    /// Parses the provided Atom feed content into a structured `AtomDocument`.
    ///
    /// This function uses the `quick_xml` crate to parse the XML content and deserialize it into an
    /// `AtomDocument` struct. It also processes the entries to set the primary link, applies any
    /// configured entry limits, and filters entries based on the specified categories.
    ///
    /// # Parameters
    ///
    /// * `content` - A string containing the Atom feed XML content to be parsed.
    ///
    /// # Returns
    ///
    /// * `Result<AtomDocument>` - On success, returns an `AtomDocument` containing the parsed feed data.
    ///   On failure, returns an error indicating the cause of the parsing failure.
    pub fn parse(&self, content: &str) -> Result<AtomDocument> {
        let mut reader = Reader::from_str(content);
        let config = reader.config_mut();
        config.trim_text(true);

        let mut doc: AtomDocument = from_reader(reader.into_inner())?;

        // Process entries to set primary link
        for entry in &mut doc.entries {
            entry.link = entry.get_primary_link();
        }

        if let Some(max) = self.config.max_entries {
            doc.entries.truncate(max);
        }

        if !self.config.filter_categories.is_empty() {
            doc.entries.retain(|entry| {
                entry.category.as_ref().map_or(false, |cat| {
                    self.config.filter_categories.contains(&cat.term)
                })
            });
        }

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_xml() {
        let config = AtomConfig::default();
        let parser = AtomParser::new(config);
        assert!(parser.parse("invalid xml").is_err());
    }

    #[test]
    fn test_empty_feed() {
        let config = AtomConfig::default();
        let parser = AtomParser::new(config);
        let empty_feed = r#"<?xml version="1.0"?><feed></feed>"#;
        assert!(parser.parse(empty_feed).is_err());
    }
}
