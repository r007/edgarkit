mod common;

use common::read_fixture;
use edgarkit::parsing::index::{IndexConfig, IndexParser, IndexType};
use std::io::BufReader;

const MASTER_INDEX_FIXTURE: &str = "indexes/master.idx";
const COMPANY_INDEX_FIXTURE: &str = "indexes/company.idx";
const XBRL_INDEX_FIXTURE: &str = "indexes/xbrl.idx";
const CRAWLER_INDEX_FIXTURE: &str = "indexes/crawler.idx";

#[test]
fn parse_company_index_fixture() {
    let content = read_fixture(COMPANY_INDEX_FIXTURE);
    let parser = IndexParser::new(IndexConfig::default());

    let entries = parser.parse(BufReader::new(content.as_bytes())).unwrap();
    assert!(!entries.is_empty());

    let first = &entries[0];
    assert_eq!(first.company_name.trim(), "3J LLC");
    assert_eq!(first.form_type.trim(), "D");
    assert_eq!(first.cik, 1975393);
    assert_eq!(first.date_filed.trim(), "20230703");

    assert!(
        first
            .url
            .starts_with("https://www.sec.gov/Archives/edgar/data/")
    );
    assert!(first.url.ends_with(".txt"));
}

#[test]
fn parse_crawler_index_fixture() {
    let content = read_fixture(CRAWLER_INDEX_FIXTURE);
    let parser = IndexParser::new(IndexConfig::default());

    let entries = parser.parse(BufReader::new(content.as_bytes())).unwrap();
    assert!(!entries.is_empty());

    let first = &entries[0];
    assert_eq!(first.company_name.trim(), "3J LLC");
    assert_eq!(first.form_type.trim(), "D");
    assert_eq!(first.cik, 1975393);
    assert_eq!(first.date_filed.trim(), "20230703");
}

#[test]
fn parse_xbrl_index_fixture() {
    let content = read_fixture(XBRL_INDEX_FIXTURE);
    let parser = IndexParser::new(IndexConfig::default());

    let entries = parser.parse(BufReader::new(content.as_bytes())).unwrap();
    assert!(!entries.is_empty());

    // Check a known first row from fixture.
    let first = &entries[0];
    assert_eq!(first.cik, 1000045);
    assert_eq!(first.company_name.trim(), "NICHOLAS FINANCIAL INC");
    assert_eq!(first.form_type.trim(), "10-Q");
    assert_eq!(first.date_filed.trim(), "2023-02-14");
}

#[test]
fn parse_company_index_with_explicit_type() {
    let content = read_fixture(COMPANY_INDEX_FIXTURE);

    let config = IndexConfig {
        index_type: Some(IndexType::Company),
        ..Default::default()
    };
    let parser = IndexParser::new(config);

    let entries = parser.parse(BufReader::new(content.as_bytes())).unwrap();
    assert!(!entries.is_empty());

    // Company index should prefix Archives URL.
    assert!(
        entries[0]
            .url
            .starts_with("https://www.sec.gov/Archives/edgar/data/")
    );
}

#[test]
fn parse_master_index_fixture() {
    let content = read_fixture(MASTER_INDEX_FIXTURE);

    let config = IndexConfig {
        index_type: Some(IndexType::Master),
        ..Default::default()
    };
    let parser = IndexParser::new(config);

    let entries = parser.parse(BufReader::new(content.as_bytes())).unwrap();
    assert!(!entries.is_empty());

    // Master index should prefix Archives URL.
    assert!(
        entries[0]
            .url
            .starts_with("https://www.sec.gov/Archives/edgar/data/")
    );
}
