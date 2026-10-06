use edgarkit::{CompanyOperations, Edgar, EdgarError};

#[tokio::test]
#[ignore]
async fn company_cik() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let cik = edgar.company_cik("AAPL").await.unwrap();
    assert_eq!(cik, 320193);
}

#[tokio::test]
#[ignore]
async fn company_cik_not_found() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let result = edgar.company_cik("INVALID").await;
    assert!(matches!(result, Err(EdgarError::TickerNotFound)));
}

#[tokio::test]
#[ignore]
async fn mutual_fund_cik() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let cik = edgar.mutual_fund_cik("LACAX").await.unwrap();
    assert_eq!(cik, 2110);
}

#[tokio::test]
#[ignore]
async fn mutual_fund_cik_not_found() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let result = edgar.mutual_fund_cik("INVALID").await;
    assert!(matches!(result, Err(EdgarError::TickerNotFound)));
}

#[tokio::test]
#[ignore]
async fn company_facts_not_found() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let result = edgar.company_facts(0).await;
    assert!(matches!(result, Err(EdgarError::NotFound)));
}

#[tokio::test]
#[ignore]
async fn company_concept() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let concept = edgar
        .company_concept(320193, "dei", "EntityCommonStockSharesOutstanding")
        .await
        .unwrap();
    assert_eq!(concept.taxonomy, "dei");
    assert_eq!(concept.tag, "EntityCommonStockSharesOutstanding");
}

#[tokio::test]
#[ignore]
async fn company_facts_for_recently_registered_filer() {
    // EDGAR sends this filer's CIK as the string "0002012383".
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let facts = edgar.company_facts(2012383).await.unwrap();
    assert_eq!(facts.cik, 2012383);
    assert!(!facts.taxonomies.us_gaap.is_empty());
}

#[tokio::test]
#[ignore]
async fn company_facts_for_ifrs_filer() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let facts = edgar.company_facts(1995306).await.unwrap();
    assert_eq!(facts.cik, 1995306);
    assert!(facts.taxonomies.us_gaap.is_empty());
    assert!(!facts.taxonomies.other["ifrs-full"].is_empty());
}

#[tokio::test]
#[ignore]
async fn frames_with_losses() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let frame = edgar
        .frames("us-gaap", "NetIncomeLoss", "USD", "CY2023")
        .await
        .unwrap();
    assert_eq!(frame.data_points.len() as u64, frame.pts);
    assert!(frame.data_points.iter().any(|p| p.val < 0.0));
    assert!(frame.data_points.iter().all(|p| p.start.is_some()));
}

#[tokio::test]
#[ignore]
async fn frames_with_per_share_values() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let frame = edgar
        .frames(
            "us-gaap",
            "EarningsPerShareBasic",
            "USD-per-shares",
            "CY2023",
        )
        .await
        .unwrap();
    assert!(frame.data_points.iter().any(|p| p.val.fract() != 0.0));
}
