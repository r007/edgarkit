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
///
/// # Fields
/// * `follow_links` - Whether to follow links in the feed
/// * `max_entries` - Optional limit on the number of entries to parse
/// * `filter_categories` - List of categories to filter entries by
#[derive(Default)]
#[cfg(feature = "atom")]
pub struct AtomConfig {
    pub follow_links: bool,
    pub max_entries: Option<usize>,
    pub filter_categories: Vec<String>,
}

/// Represents an Atom feed document structure
///
/// This struct maps the main elements of an Atom feed including author information,
/// company details, title, links, and entries.
///
/// # Fields
///
/// * `author` - Optional author information of the feed
/// * `company_info` - Optional company information, serialized from "company-info"
/// * `title` - The title of the feed
/// * `links` - Collection of related links, serialized from "link"
/// * `description` - Optional description of the feed
/// * `updated` - Timestamp indicating when the feed was last updated
/// * `entries` - Collection of feed entries, serialized from "entry"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg(feature = "atom")]
pub struct AtomDocument {
    #[serde(default)]
    pub author: Option<Author>,
    #[serde(rename = "company-info", default)]
    pub company_info: Option<CompanyInfo>,
    pub title: String,
    #[serde(rename = "link", default)]
    pub links: Vec<Link>,
    pub description: Option<String>,
    pub updated: String,
    #[serde(rename = "entry", default)]
    pub entries: Vec<AtomEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg(feature = "atom")]
pub struct CompanyInfo {
    pub addresses: Addresses,
    pub assigned_sic: String,
    pub assigned_sic_desc: String,
    pub cik: String,
    pub conformed_name: String,
    pub fiscal_year_end: String,
    pub state_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Addresses {
    pub address: Vec<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Address {
    #[serde(rename = "@type")]
    pub address_type: String,
    pub city: String,
    pub state: String,
    pub street1: String,
    pub street2: Option<String>,
    pub zip: String,
}

/// Represents a single entry in an Atom feed.
///
/// # Fields
/// * `title` - Title of the entry
/// * `link` - Link to the full entry
/// * `published` - Optional publication date
/// * `pub_date` - Optional alternative publication date field
/// * `updated` - Optional last update timestamp
/// * `id` - Unique identifier for the entry
/// * `content` - Optional full content of the entry
/// * `description` - Optional summary or description
/// * `author` - Optional author information
/// * `category` - Optional category information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct AtomEntry {
    pub title: String,
    #[serde(rename = "link", default)]
    pub links: Vec<Link>,
    #[serde(skip)]
    pub link: String,
    pub published: Option<String>,
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,
    pub updated: Option<String>,
    pub id: String,
    #[serde(rename = "content")]
    pub content: Option<Content>,
    pub description: Option<String>,
    pub author: Option<Author>,
    #[serde(default)]
    pub category: Option<Category>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Content {
    #[serde(rename = "@type")]
    pub content_type: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "accession-number")]
    pub accession_number: Option<String>,
    pub act: Option<String>,
    pub amend: Option<String>,
    #[serde(rename = "file-number")]
    pub file_number: Option<String>,
    #[serde(rename = "file-number-href")]
    pub file_number_href: Option<String>,
    #[serde(rename = "filing-date")]
    pub filing_date: Option<String>,
    #[serde(rename = "filing-href")]
    pub filing_href: Option<String>,
    #[serde(rename = "filing-type")]
    pub filing_type: Option<String>,
    #[serde(rename = "film-number")]
    pub film_number: Option<String>,
    #[serde(rename = "form-name")]
    pub form_name: Option<String>,
    pub size: Option<String>,
    #[serde(rename = "xbrl_href")]
    pub xbrl_href: Option<String>,
    #[serde(rename = "items-desc")]
    pub items_desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Link {
    #[serde(rename = "@href")]
    pub href: String,
    #[serde(rename = "@rel")]
    pub rel: Option<String>,
    #[serde(rename = "@type")]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Author {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "atom")]
pub struct Category {
    #[serde(rename = "@term")]
    pub term: String,
    #[serde(rename = "@scheme")]
    pub scheme: Option<String>,
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
