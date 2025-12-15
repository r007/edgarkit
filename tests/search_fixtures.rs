mod common;

use common::read_fixture;
use edgarkit::SearchResponse;

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
