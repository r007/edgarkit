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
    assert_eq!(
        doc.company_info.as_ref().unwrap().assigned_sic.as_deref(),
        Some("7372")
    );

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
    assert_eq!(
        doc.company_info.as_ref().unwrap().assigned_sic.as_deref(),
        Some("6770")
    );

    // Test address
    let address = &doc.company_info.as_ref().unwrap().addresses.address[0];
    assert_eq!(address.city.as_deref(), Some("SUMMIT"));
    assert_eq!(address.state.as_deref(), Some("NJ"));
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
    assert!(!doc.entries.is_empty());
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
            .any(|e| e.category.as_ref().is_some_and(|c| c.term == "S-1"))
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
            .all(|e| e.category.as_ref().is_some_and(|c| c.term == "S-1"))
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

/// Real feeds EDGAR served for SPACs whose company record is incomplete. Each
/// used to fail the whole parse — and with it the filing entries, which are
/// what a caller wants — over one missing company field.
#[test]
fn test_parse_feeds_with_incomplete_company_records() {
    let parser = setup_atom_parser();

    // Singapore: no state, zip or state-location anywhere.
    let doc = parser
        .parse(&read_fixture("atom/offshore_address.xml"))
        .unwrap();
    let info = doc.company_info.as_ref().unwrap();
    assert_eq!(info.conformed_name, "RF Acquisition Corp II");
    assert_eq!(info.state_location, None);
    assert_eq!(info.addresses.address[0].state, None);
    assert_eq!(info.addresses.address[0].zip, None);
    assert!(!doc.entries.is_empty());

    // Newly registered: no SIC assigned yet.
    let doc = parser.parse(&read_fixture("atom/no_sic.xml")).unwrap();
    assert_eq!(doc.company_info.as_ref().unwrap().assigned_sic, None);

    // No fiscal year end on record.
    let doc = parser
        .parse(&read_fixture("atom/no_fiscal_year_end.xml"))
        .unwrap();
    assert_eq!(doc.company_info.as_ref().unwrap().fiscal_year_end, None);
    assert!(!doc.entries.is_empty());
}
