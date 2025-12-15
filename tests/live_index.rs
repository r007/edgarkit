use edgarkit::{Edgar, EdgarDay, EdgarPeriod, FilingOptions, IndexOperations, Quarter};

#[tokio::test]
#[ignore]
async fn get_daily_filings() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let entries = edgar
        .get_daily_filings(EdgarDay::new(2023, 8, 1).unwrap(), None)
        .await
        .unwrap();
    assert!(!entries.is_empty());

    let entry = &entries[0];
    assert!(entry.cik > 0);
    assert!(!entry.company_name.is_empty());
    assert!(!entry.form_type.is_empty());
    assert!(!entry.url.is_empty());
}

#[tokio::test]
#[ignore]
async fn get_periodic_filings() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let entries = edgar
        .get_period_filings(EdgarPeriod::new(2023, Quarter::Q1).unwrap(), None)
        .await
        .unwrap();
    assert!(!entries.is_empty());

    let entry = &entries[0];
    assert!(entry.cik > 0);
    assert!(!entry.company_name.is_empty());
    assert!(!entry.form_type.is_empty());
    assert!(!entry.url.is_empty());
}

#[tokio::test]
#[ignore]
async fn filing_options_filters() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = FilingOptions::new()
        .with_form_types(vec!["10-K".to_string(), "10-Q".to_string()])
        .with_ciks(vec![1234567])
        .with_offset(5)
        .with_limit(10);

    let day = EdgarDay::new(2023, 8, 15).unwrap();
    let entries = edgar
        .get_daily_filings(day, Some(options.clone()))
        .await
        .unwrap();

    assert!(entries.iter().all(|e| e.cik == 1234567));
    assert!(
        entries
            .iter()
            .all(|e| ["10-K", "10-Q"].contains(&e.form_type.trim()))
    );
    assert!(entries.len() <= 10);
}

#[tokio::test]
#[ignore]
async fn daily_index_current_year() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let result = edgar.daily_index(None).await;
    assert!(result.is_ok());
}
