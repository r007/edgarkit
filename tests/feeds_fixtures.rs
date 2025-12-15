mod common;

use common::{edgar, read_fixture};
use edgarkit::FeedOperations;

#[test]
fn parse_testimony_feed() {
    let edgar = edgar();
    let content = read_fixture("rss/testimony.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    assert_eq!(feed.channel.title, "Testimony");
    assert_eq!(feed.channel.language.as_deref().unwrap(), "en");
    assert!(feed.channel.description.contains("Congressional testimony"));
}

#[test]
fn parse_feed_with_multiple_categories() {
    let edgar = edgar();
    let content = read_fixture("atom/atom1.xml");
    let feed = edgar.current_feed_from_string(&content).unwrap();

    let categories: Vec<_> = feed
        .entries
        .iter()
        .filter_map(|e| e.category.as_ref())
        .map(|c| c.term.as_str())
        .collect();

    assert!(categories.contains(&"8-K"));
    assert!(categories.contains(&"425"));
    assert!(categories.contains(&"SC 13G"));
}

#[test]
fn xml_namespaces() {
    let edgar = edgar();
    let content = read_fixture("rss/usgaap.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    assert!(feed.channel.atom_link.is_some());
    let atom_link = feed.channel.atom_link.unwrap();
    assert_eq!(atom_link.rel.as_deref(), Some("self"));
    assert!(atom_link.href.contains("usgaap.rss.xml"));
}

#[test]
fn feed_dates() {
    let edgar = edgar();
    let content = read_fixture("rss/pressreleases.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    let first_item = &feed.channel.items[0];
    assert!(first_item.pub_date.is_some());
    assert_eq!(
        first_item.pub_date.as_deref().unwrap(),
        "Fri, 24 Jan 2025 11:00:00 -0500"
    );

    assert!(
        feed.channel
            .items
            .iter()
            .all(|item| item.pub_date.is_some())
    );
}

#[test]
fn enclosure_handling() {
    let edgar = edgar();
    let content = read_fixture("rss/usgaap.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    let first_item = &feed.channel.items[0];
    let enclosure = first_item.enclosure.as_ref().unwrap();

    assert!(enclosure.url.contains("xbrl.zip"));
    assert!(enclosure.length.is_some());
    assert_eq!(enclosure.enclosure_type.as_deref(), Some("application/zip"));
}

#[test]
fn special_characters() {
    let edgar = edgar();
    let content = read_fixture("rss/pressreleases.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    for item in &feed.channel.items {
        assert!(!item.description.as_ref().unwrap().contains("&amp;"));
        assert!(!item.description.as_ref().unwrap().contains("&lt;"));
        assert!(!item.description.as_ref().unwrap().contains("&gt;"));
    }
}

#[test]
fn parse_rss_feed() {
    let edgar = edgar();
    let content = read_fixture("rss/usgaap.rss");
    let feed = edgar.rss_feed_from_string(&content).unwrap();

    assert_eq!(
        feed.channel.title,
        "Filings containing financial statements tagged using the US GAAP or IFRS taxonomies."
    );
    assert!(!feed.channel.items.is_empty());
}

#[test]
fn parse_atom_feed() {
    let edgar = edgar();
    let content = read_fixture("atom/atom.xml");
    let feed = edgar.current_feed_from_string(&content).unwrap();

    assert!(!feed.entries.is_empty());
    assert!(feed.company_info.is_some());
}
