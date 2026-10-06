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
    assert_eq!(point.val, 218600000.0);
    assert!(point.start.is_none());
    assert_eq!(point.accn, "0001104659-19-016320");
    assert_eq!(point.end, "2019-02-28");
}

#[test]
fn parse_company_facts_with_string_cik() {
    // A recently registered filer: EDGAR sends its CIK as a zero-padded string.
    let content = read_fixture("tickers/companyfacts-ifrs.json");
    assert!(content.starts_with(r#"{"cik":"0001995306""#));

    let facts: CompanyFacts = serde_json::from_str(&content).unwrap();

    assert_eq!(facts.cik, 1995306);
    assert!(facts.entity_name.starts_with("SUPER HI INTERNATIONAL"));
}

#[test]
fn parse_company_facts_without_us_gaap() {
    // A foreign private issuer reporting under IFRS has no `us-gaap` facts.
    let content = read_fixture("tickers/companyfacts-ifrs.json");
    let facts: CompanyFacts = serde_json::from_str(&content).unwrap();

    assert!(facts.taxonomies.us_gaap.is_empty());
    assert!(!facts.taxonomies.dei.is_empty());

    let ifrs = facts.taxonomies.other.get("ifrs-full").unwrap();
    let revenue = ifrs.get("RevenueFromContractsWithCustomers").unwrap();
    assert!(!revenue.units.get("USD").unwrap().is_empty());
    assert!(!facts.taxonomies.other.contains_key("dei"));
}

#[test]
fn company_facts_keep_every_taxonomy_through_a_round_trip() {
    let content = read_fixture("tickers/companyfacts-ifrs.json");
    let facts: CompanyFacts = serde_json::from_str(&content).unwrap();

    let again: CompanyFacts =
        serde_json::from_str(&serde_json::to_string(&facts).unwrap()).unwrap();

    assert_eq!(again.cik, facts.cik);
    assert_eq!(again.taxonomies.dei.len(), facts.taxonomies.dei.len());
    assert_eq!(
        again.taxonomies.other["ifrs-full"].len(),
        facts.taxonomies.other["ifrs-full"].len()
    );
}

#[test]
fn parse_company_concept_with_string_cik() {
    let content = read_fixture("tickers/companyconcept.json").replacen(
        r#""cik":320193"#,
        r#""cik":"0000320193""#,
        1,
    );
    assert!(content.contains(r#""cik":"0000320193""#));

    let concept: CompanyConcept = serde_json::from_str(&content).unwrap();
    assert_eq!(concept.cik, 320193);
}

#[test]
fn parse_frame_with_losses() {
    let content = read_fixture("tickers/frames-net-income.json");
    let frame: Frame = serde_json::from_str(&content).unwrap();

    assert_eq!(frame.tag, "NetIncomeLoss");
    assert_eq!(frame.ccp, "CY2023");
    assert_eq!(frame.data_points.len() as u64, frame.pts);

    let loss = frame
        .data_points
        .iter()
        .find(|p| p.entity_name == "BK Technologies Corporation")
        .unwrap();
    assert_eq!(loss.val, -2230000.0);
    assert_eq!(loss.start.as_deref(), Some("2023-01-01"));
    assert_eq!(loss.end, "2023-12-31");
}

#[test]
fn parse_frame_with_per_share_values() {
    let content = read_fixture("tickers/frames-eps.json");
    let frame: Frame = serde_json::from_str(&content).unwrap();

    assert_eq!(frame.tag, "EarningsPerShareBasic");
    assert_eq!(frame.uom, "USD/shares");

    let point = frame
        .data_points
        .iter()
        .find(|p| p.entity_name == "BK Technologies Corp")
        .unwrap();
    assert_eq!(point.val, -0.65);
    assert!(frame.data_points.iter().any(|p| p.val.fract() != 0.0));
}

#[test]
fn nested_company_types_are_exported() {
    // These appear in the fields of `CompanyFacts` and `Frame`, so callers need to
    // be able to name them in their own signatures.
    fn latest(points: &[edgarkit::DataPoint]) -> Option<&edgarkit::DataPoint> {
        points.iter().max_by_key(|p| p.end.as_str())
    }
    fn largest(points: &[edgarkit::FrameDataPoint]) -> Option<&edgarkit::FrameDataPoint> {
        points.iter().max_by(|a, b| a.val.total_cmp(&b.val))
    }

    let facts: CompanyFacts =
        serde_json::from_str(&read_fixture("tickers/companyfacts-ifrs.json")).unwrap();
    let taxonomies: &edgarkit::TaxonomyGroups = &facts.taxonomies;
    let fact: &edgarkit::Fact = &taxonomies.other["ifrs-full"]["ProfitLoss"];
    assert!(latest(&fact.units["USD"]).is_some());

    let frame: Frame = serde_json::from_str(&read_fixture("tickers/frames-eps.json")).unwrap();
    assert!(largest(&frame.data_points).is_some());
}
