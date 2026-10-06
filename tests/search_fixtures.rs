mod common;

use common::{FakeSec, read_fixture};
use edgarkit::{EdgarError, SearchOperations, SearchOptions, SearchResponse};
use serde_json::json;

#[test]
fn parse_search_response() {
    let content = read_fixture("search/search-index.json");
    let response: SearchResponse = serde_json::from_str(&content).unwrap();

    assert_eq!(response.took, 49);
    assert!(!response.timed_out);
    assert_eq!(response.hits.total.value, 146);

    let first_hit = &response.hits.hits[0];
    assert_eq!(first_hit._source.form, "8-K");
    assert!(!first_hit._source.display_names.is_empty());
}

#[test]
fn parse_search_response_with_null_fields() {
    let content = read_fixture("search/search-index.json");
    let response: SearchResponse = serde_json::from_str(&content).unwrap();

    for hit in response.hits.hits {
        let _ = hit._source.xsl;
        let _ = hit._source.period_ending;
        let _ = hit._source.file_description;
    }
}

/// A search response whose echoed query filters by `sics`, the way EDGAR reports
/// which SIC codes a response was computed for. An empty slice is an unfiltered one.
fn response_filtered_by(sics: &[&str]) -> String {
    let mut response: serde_json::Value =
        serde_json::from_str(&read_fixture("search/search-index.json")).unwrap();

    let mut filters = vec![json!({"terms": {"root_forms": ["10-K"]}})];
    if !sics.is_empty() {
        filters.push(json!({"terms": {"sics": sics}}));
    }
    response["query"]["query"]["bool"]["filter"] = json!(filters);
    response.to_string()
}

#[tokio::test]
async fn search_sends_the_sic_filter_the_endpoint_reads() {
    let sec = FakeSec::spawn(|_| response_filtered_by(&["7372"])).await;

    let options = SearchOptions::new().with_query("revenue").with_sic("7372");
    let response = sec.edgar().search(options).await.unwrap();
    assert!(!response.hits.hits.is_empty());

    let requests = sec.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("&sics=7372"));
    assert!(!requests[0].contains("sic=7372"));
}

#[tokio::test]
async fn search_accepts_several_sic_codes() {
    let sec = FakeSec::spawn(|_| response_filtered_by(&["7372", "3674"])).await;

    let options = SearchOptions::new().with_sic("3674, 7372");
    sec.edgar().search(options).await.unwrap();

    let requests = sec.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("sics=3674%2C7372"));
}

#[tokio::test]
async fn search_gets_past_a_cached_response_for_another_sic_filter() {
    // EDGAR's cache key leaves `sics` out: here the plain request is answered with a
    // cached unfiltered response, and only a different cache key reaches the filter.
    let sec = FakeSec::spawn(|target| {
        if target.contains("entityName=") {
            response_filtered_by(&["7372"])
        } else {
            response_filtered_by(&[])
        }
    })
    .await;

    let options = SearchOptions::new().with_query("revenue").with_sic("7372");
    let response = sec.edgar().search(options).await.unwrap();

    let echoed = response.query.unwrap();
    assert_eq!(
        echoed["query"]["bool"]["filter"][1],
        json!({"terms": {"sics": ["7372"]}})
    );

    let requests = sec.requests();
    assert_eq!(requests.len(), 2);
    assert!(!requests[0].contains("entityName"));
    assert!(requests[1].ends_with("&sics=7372&entityName="));
}

#[tokio::test]
async fn unfiltered_search_gets_past_a_cached_sic_filtered_response() {
    let sec = FakeSec::spawn(|target| {
        if target.contains("entityName=") {
            response_filtered_by(&[])
        } else {
            response_filtered_by(&["7372"])
        }
    })
    .await;

    let options = SearchOptions::new().with_query("revenue");
    sec.edgar().search(options).await.unwrap();

    assert_eq!(sec.requests().len(), 2);
}

#[tokio::test]
async fn search_fails_rather_than_return_another_sic_filters_results() {
    let sec = FakeSec::spawn(|_| response_filtered_by(&[])).await;

    let options = SearchOptions::new().with_query("revenue").with_sic("7372");
    let result = sec.edgar().search(options).await;

    assert!(matches!(result, Err(EdgarError::InvalidResponse(_))));
    // The plain request, then one per alternative cache key.
    let requests = sec.requests();
    assert_eq!(requests.len(), 3);
    assert!(requests[1].ends_with("&entityName="));
    assert!(requests[2].ends_with("&locationType=located"));
}

#[tokio::test]
async fn search_does_not_override_filters_to_change_the_cache_key() {
    // Both alternative cache keys are filters the caller already set, so there is
    // nothing left to vary and the search fails after the one request.
    let sec = FakeSec::spawn(|_| response_filtered_by(&[])).await;

    let options = SearchOptions::new()
        .with_entity_name("apple")
        .with_incorporated_location(true)
        .with_sic("7372");
    let result = sec.edgar().search(options).await;

    assert!(matches!(result, Err(EdgarError::InvalidResponse(_))));
    let requests = sec.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].contains("entityName=apple"));
    assert!(requests[0].contains("locationType=incorporated"));
}
