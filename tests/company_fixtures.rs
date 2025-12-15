mod common;

use common::read_fixture;
use edgarkit::{CompanyConcept, CompanyFacts, Frame};

#[test]
fn parse_company_facts() {
    let content = read_fixture("tickers/companyfacts.json");
    let facts: CompanyFacts = serde_json::from_str(&content).unwrap();

    assert_eq!(facts.cik, 320193);
    assert_eq!(facts.entity_name, "Apple Inc.");

    let income_tax = facts
        .taxonomies
        .us_gaap
        .get("IncomeTaxExpenseBenefit")
        .unwrap();
    assert_eq!(
        income_tax.label,
        Some("Income Tax Expense (Benefit)".to_string())
    );

    let data_points = income_tax.units.get("USD").unwrap();
    let point = &data_points[0];
    assert_eq!(point.val, 1512000000);
    assert_eq!(point.form, "10-K");
    assert_eq!(point.filed, "2009-10-27");
    assert!(point.frame.is_none());
}

#[test]
fn parse_company_concept() {
    let content = read_fixture("tickers/companyconcept.json");
    let concept: CompanyConcept = serde_json::from_str(&content).unwrap();

    assert_eq!(concept.cik, 320193);
    assert_eq!(concept.taxonomy, "dei");
    assert_eq!(concept.tag, "EntityCommonStockSharesOutstanding");
    assert!(!concept.units.is_empty());

    let data_points = concept.units.get("shares").unwrap();
    let point = &data_points[0];
    assert!(point.val.is_number());
    assert_eq!(point.form, "10-Q");
}

#[test]
fn parse_frames() {
    let content = read_fixture("tickers/frames.json");
    let frame: Frame = serde_json::from_str(&content).unwrap();

    assert_eq!(frame.taxonomy, "us-gaap");
    assert_eq!(frame.tag, "AccountsPayableCurrent");
    assert_eq!(frame.uom, "USD");
    assert_eq!(frame.ccp, "CY2019Q1I");

    let point = &frame.data_points[0];
    assert_eq!(point.cik, 1750);
    assert_eq!(point.entity_name, "AAR CORP.");
    assert_eq!(point.loc, "US-IL");
    assert_eq!(point.val, 218600000);
    assert_eq!(point.accn, "0001104659-19-016320");
    assert_eq!(point.end, "2019-02-28");
}
