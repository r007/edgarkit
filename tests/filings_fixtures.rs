mod common;

use common::read_fixture;
use edgarkit::{DetailedFiling, DirectoryResponse, Submission};

#[test]
fn parse_submission() {
    let content = read_fixture("submissions/submission.json");
    let submission: Submission = serde_json::from_str(&content).unwrap();

    assert_eq!(submission.name, "Apple Inc.");
    assert_eq!(submission.cik, "0000320193");
    assert_eq!(submission.tickers, vec!["AAPL"]);
    assert!(!submission.filings.recent.accession_number.is_empty());
}

#[test]
fn detailed_filing_conversion() {
    let content = read_fixture("submissions/submission.json");
    let submission: Submission = serde_json::from_str(&content).unwrap();

    let filing = DetailedFiling::try_from((&submission.filings.recent, 0)).unwrap();

    assert!(filing.acceptance_date_time.timestamp() > 0);
    assert!(!filing.accession_number.is_empty());
    assert!(!filing.filing_date.is_empty());
}

#[test]
fn parse_directory_response() {
    let content = read_fixture("submissions/directory.json");
    let dir: DirectoryResponse = serde_json::from_str(&content).unwrap();

    assert!(!dir.directory.item.is_empty());
    let first_item = &dir.directory.item[0];
    assert_eq!(first_item.name, "0001140361-25-000228-index-headers.html");
    assert_eq!(first_item.type_, "text.gif");
}

#[test]
fn parse_submission2_with_xbrl_numeric_all_nulls() {
    let content = read_fixture("submissions/submission2.json");
    let submission: Submission = serde_json::from_str(&content).unwrap();

    assert_eq!(submission.name, "EQV Ventures Acquisition Corp. II");
    assert!(submission.filings.recent.is_xbrl_numeric.is_some());

    let filings = submission.into_detailed_filings();
    assert!(!filings.is_empty());
    assert!(filings.iter().all(|f| !f.is_xbrl_numeric));
}

#[test]
fn parse_submission3_with_xbrl_numeric_mixed() {
    let content = read_fixture("submissions/submission3.json");
    let submission: Submission = serde_json::from_str(&content).unwrap();

    assert_eq!(submission.name, "Twelve Seas Investment Co III/Cayman");
    assert!(submission.filings.recent.is_xbrl_numeric.is_some());

    let filings = submission.into_detailed_filings();
    assert!(!filings.is_empty());
    assert!(filings[0].is_xbrl_numeric);
    assert!(filings[1..].iter().all(|f| !f.is_xbrl_numeric));
}

#[test]
fn detailed_filing_has_xbrl_data() {
    let content = read_fixture("submissions/submission3.json");
    let submission: Submission = serde_json::from_str(&content).unwrap();

    let filings = submission.into_detailed_filings();
    // First filing has isXBRL=1 and isXBRLNumeric=1
    assert!(filings[0].has_xbrl_data());
}
