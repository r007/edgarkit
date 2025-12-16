mod common;

use common::read_fixture;
use edgarkit::parsing::rss::{RssConfig, RssParser};

const USGAAP_FIXTURE: &str = "rss/usgaap.rss";
const PRESSREL_FIXTURE: &str = "rss/pressreleases.rss";

fn setup_rss_parser() -> RssParser {
    RssParser::new(RssConfig::default())
}

#[test]
fn test_link_extraction() {
    let parser = setup_rss_parser();
    let content = read_fixture(USGAAP_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    // Test standard link
    assert!(doc.channel.link.contains("sec.gov"));

    // Test atom:link
    let atom_link = doc.channel.atom_link.unwrap();
    assert!(atom_link.href.contains("usgaap.rss.xml"));
    assert_eq!(atom_link.rel.as_deref(), Some("self"));
}

// RSS Parser Tests
#[test]
fn test_rss_usgaap_feed() {
    let parser = setup_rss_parser();
    let content = read_fixture(USGAAP_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(
        doc.channel.title,
        "Filings containing financial statements tagged using the US GAAP or IFRS taxonomies."
    );
    assert!(doc.channel.link.contains("sec.gov"));
    assert_eq!(doc.channel.language.as_deref().unwrap(), "en-us");
    assert!(doc.channel.items.len() > 0);

    // Check first item
    let first_item = &doc.channel.items[0];
    assert!(first_item.title.contains("HF Sinclair Corp"));
    assert!(first_item.enclosure.is_some());
    assert_eq!(first_item.description.as_deref().unwrap(), "8-K");
}

#[test]
fn test_rss_press_releases() {
    let parser = setup_rss_parser();
    let content = read_fixture(PRESSREL_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(doc.channel.title, "Press Releases");
    assert_eq!(doc.channel.language.as_deref().unwrap(), "en");
    assert!(doc.channel.description.contains("Official announcements"));

    // Check press release content
    let first_release = &doc.channel.items[0];
    assert!(first_release.title.contains("Acting Chairman"));
    assert!(first_release.link.contains("newsroom/press-releases"));
    assert!(first_release.pub_date.is_some());
}

#[test]
fn test_rss_with_max_entries() {
    let config = RssConfig {
        max_entries: Some(5),
        ..Default::default()
    };
    let parser = RssParser::new(config);
    let content = read_fixture(USGAAP_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(doc.channel.items.len(), 5);
}

#[test]
fn test_xbrl_filing_parsing() {
    let parser = setup_rss_parser();
    let content = read_fixture(USGAAP_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    let item = &doc.channel.items[0];
    if let Some(filing) = &item.xbrl_filing {
        assert!(filing.xmlns.is_some() || filing.form_type.is_some());

        if let Some(xbrl_files) = &filing.xbrl_files {
            for file in &xbrl_files.files {
                // Check if at least one field is present
                assert!(file.sequence.is_some() || file.file_type.is_some() || file.url.is_some());
            }
        }
    }
}

#[test]
fn test_multiple_filings() {
    let parser = setup_rss_parser();
    let content = read_fixture(USGAAP_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    // Check items have XBRL filings
    let items_with_xbrl = doc
        .channel
        .items
        .iter()
        .filter(|item| {
            item.xbrl_filing.as_ref().map_or(false, |filing| {
                filing.xmlns.is_some() || filing.form_type.is_some()
            })
        })
        .count();
    assert!(items_with_xbrl > 0);
}
