#[cfg(feature = "rss")]
use crate::Result;
#[cfg(feature = "rss")]
use quick_xml::{Reader, events::Event};
#[cfg(feature = "rss")]
use serde::Deserialize;

/// Configuration options for RSS feed parsing.
///
/// Allows customization of parsing behavior including entry limits and category filtering.
#[derive(Default)]
#[cfg(feature = "rss")]
pub struct RssConfig {
    /// Maximum number of items to parse before stopping.
    pub max_entries: Option<usize>,

    /// List of categories to filter items by (if empty, all items are included).
    pub filter_categories: Vec<String>,
}

#[cfg(feature = "rss")]
pub struct RssParser {
    config: RssConfig,
}

/// Root element of an RSS 2.0 feed document.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct RssDocument {
    /// The RSS channel containing all feed metadata and items.
    pub channel: Channel,
}

/// An RSS channel containing feed metadata and items.
///
/// Represents the main container for an RSS feed with title, description,
/// publication information, and a collection of items (filings or news articles).
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct Channel {
    /// Name of the RSS channel.
    pub title: String,

    /// URL to the website corresponding to this channel (parsed manually).
    #[serde(skip)]
    pub link: String,

    /// Atom syndication link for feed discovery (parsed manually due to XML namespace).
    #[serde(skip)]
    pub atom_link: Option<AtomLink>,

    /// Textual description of the channel's content.
    pub description: String,

    /// Language the channel is written in (e.g., "en-us").
    pub language: Option<String>,

    /// Timestamp of the channel's last update.
    #[serde(rename = "lastBuildDate")]
    pub last_build_date: Option<String>,

    /// When the channel was first published.
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,

    /// Collection of items/entries in this feed.
    #[serde(rename = "item", default)]
    pub items: Vec<Item>,
}

/// Atom-style link element for feed autodiscovery.
///
/// Parsed manually due to XML namespace handling limitations in serde.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct AtomLink {
    /// Target URL of the link.
    #[serde(rename = "@href")]
    pub href: String,

    /// Link relationship (typically "self" for feed URLs).
    #[serde(rename = "@rel")]
    pub rel: Option<String>,

    /// MIME type of the linked resource.
    #[serde(rename = "@type")]
    pub link_type: Option<String>,
}

/// Individual item/entry in an RSS feed, representing a filing or news article.
///
/// Contains metadata about SEC filings including company identifiers, form types,
/// filing dates, and associated XBRL data files.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct Item {
    /// Title of the item (typically includes company name and form type).
    pub title: String,

    /// URL where the full filing can be accessed.
    pub link: String,

    /// Detailed textual description of the item.
    pub description: Option<String>,

    /// Timestamp when this item was first published.
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,

    /// Globally unique identifier for this item.
    pub guid: Option<String>,

    /// XBRL filing information including instance documents and taxonomy files.
    #[serde(rename = "xbrlFiling", default)]
    pub xbrl_filing: Option<XbrlFiling>,

    /// File attachment metadata (e.g., XBRL instance file).
    pub enclosure: Option<Enclosure>,
}

/// XBRL filing metadata including instance documents and taxonomy references.
///
/// Provides detailed information about XBRL-tagged financial data submissions,
/// including accession number, form type, filing dates, and associated files.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFiling {
    /// XML namespace for EDGAR-specific elements.
    #[serde(rename = "@xmlns:edgar")]
    pub xmlns: Option<String>,

    /// Unique accession number for this filing (format: 0000000000-00-000000).
    #[serde(rename = "accessionNumber")]
    pub accession_number: Option<String>,

    /// Securities Act number under which the filing is made.
    #[serde(rename = "actNumber")]
    pub act_number: Option<String>,

    /// SEC file number assigned to the registrant.
    #[serde(rename = "fileNumber")]
    pub file_number: Option<String>,

    /// Film number for physical SEC filing records.
    #[serde(rename = "filmNumber")]
    pub film_number: Option<String>,

    /// SEC form type (e.g., 10-K, 10-Q, 8-K).
    #[serde(rename = "formType")]
    pub form_type: Option<String>,

    /// Items being reported (for forms like 8-K).
    #[serde(rename = "items")]
    pub items: Option<String>,

    /// Reporting period end date (for financial statements).
    #[serde(rename = "period")]
    pub period: Option<String>,

    /// Collection of XBRL instance and taxonomy files.
    #[serde(rename = "xbrlFiles")]
    pub xbrl_files: Option<XbrlFiles>,
}

/// Container for XBRL file references.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFiles {
    /// List of individual XBRL files (instance documents, schemas, etc.).
    #[serde(rename = "xbrlFile", default)]
    pub files: Vec<XbrlFile>,
}

/// Individual XBRL file reference within a filing.
///
/// Can represent instance documents, taxonomy schemas, linkbases, or label files.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFile {
    /// Sequence number indicating file order.
    #[serde(rename = "@sequence", default)]
    pub sequence: Option<String>,

    /// Type of XBRL file (e.g., "EX-101.INS", "EX-101.SCH", "EX-101.CAL").
    #[serde(rename = "@type", default)]
    pub file_type: Option<String>,

    /// File size in bytes.
    #[serde(rename = "@size", default)]
    pub size: Option<String>,

    /// Human-readable description of the file's purpose.
    #[serde(rename = "@description", default)]
    pub description: Option<String>,

    /// Direct URL to download this specific XBRL file.
    #[serde(rename = "@url", default)]
    pub url: Option<String>,

    /// Date this file was created or last modified.
    #[serde(rename = "@date", default)]
    pub date: Option<String>,
}

/// File attachment metadata for an RSS item.
///
/// Describes an attached file (typically XBRL instance documents) with its location,
/// size, and content type.
#[derive(Debug, Deserialize, Clone)]
#[cfg(feature = "rss")]
pub struct Enclosure {
    /// Direct URL to download the attached file.
    #[serde(rename = "@url")]
    pub url: String,

    /// File size in bytes.
    #[serde(rename = "@length")]
    pub length: Option<u64>,

    /// MIME type of the enclosed file.
    #[serde(rename = "@type")]
    pub enclosure_type: Option<String>,
}

#[cfg(feature = "rss")]
impl RssParser {
    pub fn new(config: RssConfig) -> Self {
        Self { config }
    }

    /// Helper method to extract links from the provided RSS content. Currently serde doesn't support xml namespaces.
    /// It cause the issue with parsing Atom links (parser is throwing "duplicate field `link`" error).
    /// This method manually extracts both standard and Atom links.
    ///
    /// It returns a tuple containing the extracted standard link and an optional Atom link.
    ///
    /// # Parameters
    ///
    /// * `content` - A string containing the RSS content to be parsed.
    ///
    /// # Return Value
    ///
    /// * `Result<(String, Option<AtomLink>)>` - If the parsing is successful, the function returns a `Result` containing
    ///   a tuple with the extracted standard link and an optional Atom link.
    ///   If the parsing fails, an error is returned.
    ///
    /// # Errors
    ///
    /// This function can return the following errors:
    ///
    /// * `quick_xml::Error` - If there is an error during XML parsing.
    fn extract_links(&self, content: &str) -> Result<(String, Option<AtomLink>)> {
        let mut reader = Reader::from_str(content);
        let config = reader.config_mut();
        config.trim_text(true);

        let mut buf = Vec::new();
        let mut link = String::new();
        let mut atom_link = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        b"link" => {
                            // Regular link - get text content
                            if !e.attributes().any(|a| a.unwrap().key.as_ref() == b"href") {
                                if let Ok(Event::Text(text)) = reader.read_event_into(&mut buf) {
                                    link = text.unescape()?.into_owned();
                                }
                            }
                        }
                        b"atom:link" => {
                            // Atom link - get attributes
                            let mut href = String::new();
                            let mut rel = None;
                            let mut link_type = None;

                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"href" => href = attr.unescape_value()?.into_owned(),
                                    b"rel" => rel = Some(attr.unescape_value()?.into_owned()),
                                    b"type" => {
                                        link_type = Some(attr.unescape_value()?.into_owned())
                                    }
                                    _ => {}
                                }
                            }

                            if !href.is_empty() {
                                atom_link = Some(AtomLink {
                                    href,
                                    rel,
                                    link_type,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }
        Ok((link, atom_link))
    }

    /// Parses the provided RSS content into a structured `RssDocument`.
    ///
    /// This function takes a string containing RSS content as input and returns a `Result` containing
    /// the parsed `RssDocument` on success or an error if the parsing fails.
    ///
    /// # Parameters
    ///
    /// * `content` - A string containing the RSS content to be parsed.
    ///
    /// # Return Value
    ///
    /// * `Ok(RssDocument)` - If the parsing is successful, the function returns a `Result` containing
    ///   the parsed `RssDocument`.
    /// * `Err(Error)` - If the parsing fails, the function returns a `Result` containing an error.
    ///
    /// # Errors
    ///
    /// This function can return the following errors:
    ///
    /// * `quick_xml::Error` - If there is an error during XML parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use edgarkit::parsing::rss::{RssParser, RssConfig};
    ///
    /// let parser = RssParser::new(RssConfig::default());
    /// ```
    pub fn parse(&self, content: &str) -> Result<RssDocument> {
        let mut rss: RssDocument = quick_xml::de::from_str(content)?;

        // Extract links
        let (link, atom_link) = self.extract_links(content)?;
        rss.channel.link = link;
        rss.channel.atom_link = atom_link;

        if let Some(max) = self.config.max_entries {
            rss.channel.items.truncate(max);
        }

        Ok(rss)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_xml() {
        let config = RssConfig::default();
        let parser = RssParser::new(config);
        assert!(parser.parse("invalid xml").is_err());
    }

    #[test]
    fn test_empty_feed() {
        let config = RssConfig::default();
        let parser = RssParser::new(config);
        let empty_rss = r#"<?xml version="1.0"?><rss><channel></channel></rss>"#;
        assert!(parser.parse(empty_rss).is_err());
    }
}
