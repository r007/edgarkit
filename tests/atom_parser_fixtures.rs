mod common;

use common::read_fixture;
use edgarkit::parsing::atom::{AtomConfig, AtomParser};

const ATOM_FIXTURE: &str = "atom/atom.xml";
const ATOM1_FIXTURE: &str = "atom/atom1.xml";

fn setup_atom_parser() -> AtomParser {
    AtomParser::new(AtomConfig::default())
}

#[test]
fn test_parse_spac_feed() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(
        doc.company_info.as_ref().unwrap().conformed_name,
        "Maquia Capital Acquisition Corp"
    );
    assert_eq!(doc.company_info.as_ref().unwrap().cik, "0001844419");
    assert_eq!(doc.company_info.as_ref().unwrap().assigned_sic, "7372");

    let entries = &doc.entries;
    assert!(
        entries
            .iter()
            .all(|e| e.category.as_ref().unwrap().term == "S-1"
                || e.category.as_ref().unwrap().term == "S-1/A")
    );
}

#[test]
fn test_parse_keen_vision_feed() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM1_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(
        doc.company_info.as_ref().unwrap().conformed_name,
        "Keen Vision Acquisition Corp."
    );
    assert_eq!(doc.company_info.as_ref().unwrap().cik, "0001889983");
    assert_eq!(doc.company_info.as_ref().unwrap().assigned_sic, "6770");

    // Test address
    let address = &doc.company_info.as_ref().unwrap().addresses.address[0];
    assert_eq!(address.city, "SUMMIT");
    assert_eq!(address.state, "NJ");
}

#[test]
fn test_different_form_types() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM1_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    let form_types: Vec<_> = doc
        .entries
        .iter()
        .map(|e| e.category.as_ref().unwrap().term.as_str())
        .collect();

    assert!(form_types.contains(&"8-K"));
    assert!(form_types.contains(&"425"));
    assert!(form_types.contains(&"SC 13G"));
}

#[test]
fn test_filing_content() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM1_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    let filing = doc.entries.first().unwrap();
    assert!(filing.content.is_some());

    let content = filing.content.as_ref().unwrap();
    assert!(content.file_number_href.is_some());
    assert!(content.filing_href.is_some());
    assert!(content.items_desc.is_some());
    assert_eq!(content.filing_type.as_deref(), Some("8-K"));
}

#[test]
fn test_xml_namespaces() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM1_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    // Test xmlns attribute handling
    assert!(
        doc.entries.iter().all(
            |e| e.category.as_ref().unwrap().scheme == Some("https://www.sec.gov/".to_string())
        )
    );
}

#[test]
fn test_atom_feed_links() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert!(!doc.links.is_empty());
    for entry in &doc.entries {
        assert!(!entry.links.is_empty());
        assert!(entry.get_primary_link().contains("sec.gov"));
    }
}

// Atom Parser Tests
#[test]
fn test_atom_feed_metadata() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert!(doc.title.contains("Maquia Capital"));
    assert!(doc.entries.len() > 0);
}

#[test]
fn test_atom_category_parsing() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    for entry in doc.entries {
        if let Some(cat) = entry.category {
            assert!(!cat.term.is_empty());
            assert!(cat.scheme.is_some());
            assert!(cat.label.is_some());
        }
    }
}

#[test]
fn test_atom_company_info() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert!(
        doc.entries
            .iter()
            .any(|e| e.category.as_ref().map_or(false, |c| c.term == "S-1"))
    );
}

#[test]
fn test_atom_entry_content() {
    let parser = setup_atom_parser();
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    for entry in doc.entries {
        assert!(!entry.title.is_empty());
        assert!(!entry.link.is_empty());
        assert!(entry.id.contains("accession-number="));

        if let Some(content) = entry.content {
            // Check content fields properly mapped
            assert!(content.file_number_href.is_some());
            assert!(content.filing_href.is_some());
            if content.filing_type.as_deref() == Some("8-K") {
                assert!(content.items_desc.is_some());
            }
        }
    }
}

#[test]
fn test_atom_with_category_filter() {
    let config = AtomConfig {
        filter_categories: vec!["S-1".to_string()],
        ..Default::default()
    };
    let parser = AtomParser::new(config);
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert!(
        doc.entries
            .iter()
            .all(|e| e.category.as_ref().map_or(false, |c| c.term == "S-1"))
    );
}

#[test]
fn test_atom_with_max_entries() {
    let config = AtomConfig {
        max_entries: Some(3),
        ..Default::default()
    };
    let parser = AtomParser::new(config);
    let content = read_fixture(ATOM_FIXTURE);
    let doc = parser.parse(&content).unwrap();

    assert_eq!(doc.entries.len(), 3);
}
