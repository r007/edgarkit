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
