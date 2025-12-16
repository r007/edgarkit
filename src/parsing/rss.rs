#[cfg(feature = "rss")]
use crate::Result;
#[cfg(feature = "rss")]
use quick_xml::{Reader, events::Event};
#[cfg(feature = "rss")]
use serde::Deserialize;

/// RSS parsing configuration options.
///
/// This struct allows customization of the RSS parsing behavior, such as limiting the number of
/// entries to parse and filtering items by categories.
///
/// # Fields
/// * `max_entries` - Optional maximum number of entries to parse
/// * `filter_categories` - List of categories to filter items by
#[derive(Default)]
#[cfg(feature = "rss")]
pub struct RssConfig {
    pub max_entries: Option<usize>,
    pub filter_categories: Vec<String>,
}

#[cfg(feature = "rss")]
pub struct RssParser {
    config: RssConfig,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct RssDocument {
    pub channel: Channel,
}

/// Represents an RSS channel containing feed metadata and items.
///
/// This struct maps the standard RSS 2.0 channel elements along with an optional Atom link.
/// It contains basic feed information such as title, link, description, and a collection of items.
///
/// # Fields
/// * `title` - The name of the channel
/// * `link` - The URL to the website corresponding to the channel
/// * `atom_link` - Optional Atom syndication format link
/// * `description` - Phrase or sentence describing the channel
/// * `language` - Optional language the channel is written in
/// * `last_build_date` - Optional indication of the last time the content was updated
/// * `pub_date` - Optional indication of when the channel was published
/// * `items` - Collection of items/entries in the feed
///
/// # Note
/// This implementation follows the RSS 2.0 specification while also supporting Atom link elements
/// through the `atom_link` field.
#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct Channel {
    pub title: String,
    // Handle these links manually, because serde doesn't support xml namespaces
    #[serde(skip)]
    pub link: String,
    #[serde(skip)]
    pub atom_link: Option<AtomLink>,
    pub description: String,
    pub language: Option<String>,
    #[serde(rename = "lastBuildDate")]
    pub last_build_date: Option<String>,
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,
    #[serde(rename = "item", default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct AtomLink {
    #[serde(rename = "@href")]
    pub href: String,
    #[serde(rename = "@rel")]
    pub rel: Option<String>,
    #[serde(rename = "@type")]
    pub link_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct Item {
    pub title: String,
    pub link: String,
    pub description: Option<String>,
    #[serde(rename = "pubDate")]
    pub pub_date: Option<String>,
    pub guid: Option<String>,
    #[serde(rename = "xbrlFiling", default)]
    pub xbrl_filing: Option<XbrlFiling>,
    pub enclosure: Option<Enclosure>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFiling {
    #[serde(rename = "@xmlns:edgar")]
    pub xmlns: Option<String>,
    #[serde(rename = "accessionNumber")]
    pub accession_number: Option<String>,
    #[serde(rename = "actNumber")]
    pub act_number: Option<String>,
    #[serde(rename = "fileNumber")]
    pub file_number: Option<String>,
    #[serde(rename = "filmNumber")]
    pub film_number: Option<String>,
    #[serde(rename = "formType")]
    pub form_type: Option<String>,
    #[serde(rename = "items")]
    pub items: Option<String>,
    #[serde(rename = "period")]
    pub period: Option<String>,
    #[serde(rename = "xbrlFiles")]
    pub xbrl_files: Option<XbrlFiles>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFiles {
    #[serde(rename = "xbrlFile", default)]
    pub files: Vec<XbrlFile>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct XbrlFile {
    #[serde(rename = "@sequence", default)]
    pub sequence: Option<String>,
    #[serde(rename = "@type", default)]
    pub file_type: Option<String>,
    #[serde(rename = "@size", default)]
    pub size: Option<String>,
    #[serde(rename = "@description", default)]
    pub description: Option<String>,
    #[serde(rename = "@url", default)]
    pub url: Option<String>,
    #[serde(rename = "@date", default)]
    pub date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg(feature = "rss")]
pub struct Enclosure {
    #[serde(rename = "@url")]
    pub url: String,
    #[serde(rename = "@length")]
    pub length: Option<String>,
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
