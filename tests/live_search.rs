use edgarkit::{Edgar, SearchOperations, SearchOptions};

#[tokio::test]
#[ignore]
async fn search_basic() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("MAQC")
        .with_forms(vec!["8-K".to_string()])
        .with_count(10);

    let response = edgar.search(options).await.unwrap();
    assert!(!response.hits.hits.is_empty());

    for hit in response.hits.hits {
        assert!(hit._source.form.starts_with("8-K"));
    }
}

#[tokio::test]
#[ignore]
async fn search_with_date_range() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("MAQC")
        .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

    let response = edgar.search(options).await.unwrap();

    for hit in response.hits.hits {
        let file_date =
            chrono::NaiveDate::parse_from_str(&hit._source.file_date, "%Y-%m-%d").unwrap();
        assert!(file_date >= chrono::NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
        assert!(file_date <= chrono::NaiveDate::from_ymd_opt(2023, 12, 31).unwrap());
    }
}

#[tokio::test]
#[ignore]
async fn search_null_fields_handling() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("MAQC")
        .with_forms(vec!["DEFA14A".to_string()])
        .with_count(10);

    let response = edgar.search(options).await.unwrap();

    for hit in response.hits.hits {
        let _ = hit._source.period_ending;
    }
}

#[tokio::test]
#[ignore]
async fn search_all() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("SPAC")
        .with_forms(vec!["S-1".to_string()])
        .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

    let results = edgar.search_all(options).await.unwrap();

    assert!(results.len() > 100);

    for hit in results {
        assert!(hit._source.form.starts_with("S-1"));
    }
}

#[tokio::test]
#[ignore]
async fn search_all_with_small_result_set() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("SPAC Rule 419")
        .with_forms(vec!["S-1".to_string()])
        .with_date_range("2024-01-01".to_string(), "2024-03-01".to_string());

    let results = edgar.search_all(options).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
#[ignore]
async fn search_with_sic() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("semiconductor")
        .with_forms(vec!["10-K".to_string()])
        .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

    let everything = edgar.search(options.clone()).await.unwrap();
    // Asked right after the unfiltered search, so EDGAR's cache holds the
    // unfiltered response under the key this one would otherwise share.
    let filtered = edgar.search(options.with_sic("3674")).await.unwrap();

    assert!(!filtered.hits.hits.is_empty());
    assert!(filtered.hits.total.value < everything.hits.total.value);
    for hit in filtered.hits.hits {
        assert!(hit._source.sics.contains(&"3674".to_string()));
    }
}

#[tokio::test]
#[ignore]
async fn search_with_incorporated_location() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("osmium")
        .with_forms(vec!["10-K".to_string()])
        .with_date_range("2021-01-01".to_string(), "2021-12-31".to_string())
        .with_location_code("DE")
        .with_incorporated_location(true);

    let response = edgar.search(options).await.unwrap();
    assert!(!response.hits.hits.is_empty());
    for hit in response.hits.hits {
        assert!(hit._source.inc_states.contains(&"DE".to_string()));
    }
}

#[tokio::test]
#[ignore]
async fn search_second_page() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    let options = SearchOptions::new()
        .with_query("semiconductor")
        .with_forms(vec!["10-K".to_string()])
        .with_date_range("2023-01-01".to_string(), "2023-12-31".to_string());

    let first = edgar.search(options.clone()).await.unwrap();
    let second = edgar.search(options.with_page(2)).await.unwrap();

    assert!(!second.hits.hits.is_empty());
    assert_ne!(first.hits.hits[0]._id, second.hits.hits[0]._id);
}
