use edgarkit::{Edgar, EdgarError, FilingOperations, FilingOptions};

#[tokio::test]
#[ignore]
async fn latest_filing_content() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let filing_content = edgar
        .get_latest_filing_content("320193", &["10-K"])
        .await
        .unwrap();

    assert!(!filing_content.is_empty());
    assert!(filing_content.len() > 1000);

    let invalid_result = edgar.get_latest_filing_content("000000", &["10-K"]).await;
    assert!(matches!(invalid_result, Err(EdgarError::NotFound)));

    let invalid_form = edgar
        .get_latest_filing_content("320193", &["INVALID"])
        .await;
    assert!(matches!(invalid_form, Err(EdgarError::NotFound)));
}

#[tokio::test]
#[ignore]
async fn get_text_filing_links() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let opts = FilingOptions::new().with_limit(3);
    let filing_links = edgar
        .get_text_filing_links("320193", Some(opts))
        .await
        .unwrap();

    assert_eq!(filing_links.len(), 3);

    for (filing, url, sec_url) in &filing_links {
        let expected_url_pattern = format!(
            "{}/data/320193/{}/{}.txt",
            edgar.archives_url(),
            filing.accession_number.replace("-", ""),
            filing.accession_number
        );

        let expected_sec_url_pattern = format!(
            "{}/data/320193/{}/{}-index.html",
            edgar.archives_url(),
            filing.accession_number.replace("-", ""),
            filing.accession_number
        );

        assert_eq!(&expected_url_pattern, url);
        assert_eq!(&expected_sec_url_pattern, sec_url);
    }

    let form_opts = FilingOptions::new().with_form_type("10-K").with_limit(2);
    let form_filing_links = edgar
        .get_text_filing_links("320193", Some(form_opts))
        .await
        .unwrap();

    for (filing, _, _) in &form_filing_links {
        assert_eq!(filing.form, "10-K");
    }

    let invalid_form_opts = FilingOptions::new().with_form_type("INVALID_FORM_TYPE");
    let invalid_form_result = edgar
        .get_text_filing_links("320193", Some(invalid_form_opts))
        .await
        .unwrap();
    assert!(invalid_form_result.is_empty());
}

#[tokio::test]
#[ignore]
async fn get_sgml_header_links() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let opts = FilingOptions::new().with_limit(3);
    let filing_links = edgar
        .get_sgml_header_links("320193", Some(opts))
        .await
        .unwrap();

    assert_eq!(filing_links.len(), 3);

    for (filing, url, _) in &filing_links {
        assert!(url.ends_with(".hdr.sgml"));

        let formatted_acc = filing.accession_number.replace("-", "");
        assert!(url.contains(&formatted_acc));
        assert!(url.contains(&filing.accession_number));
    }
}

#[tokio::test]
#[ignore]
async fn filings_with_form_type() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let opts = FilingOptions::new().with_form_type("10-K");
    let filings = edgar.filings("320193", Some(opts)).await.unwrap();
    assert!(filings.iter().all(|f| f.form == "10-K"));
}

#[tokio::test]
#[ignore]
async fn filings_with_limit() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let opts = FilingOptions::new().with_limit(1);
    let filings = edgar.filings("320193", Some(opts)).await.unwrap();
    assert_eq!(filings.len(), 1);
}

#[tokio::test]
#[ignore]
async fn filings_with_offset() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let all_filings = edgar.filings("320193", None).await.unwrap();
    let opts = FilingOptions::new().with_offset(1);
    let offset_filings = edgar.filings("320193", Some(opts)).await.unwrap();
    assert_eq!(offset_filings.len(), all_filings.len() - 1);
}

#[tokio::test]
#[ignore]
async fn submissions_live() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let submissions = edgar.submissions("320193").await.unwrap();
    assert_eq!(submissions.name, "Apple Inc.");
}

#[tokio::test]
#[ignore]
async fn submissions_not_found() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let result = edgar.submissions("0").await;
    assert!(matches!(result, Err(EdgarError::NotFound)));
}
