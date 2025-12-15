mod common;

use common::read_fixture;
use edgarkit::IndexResponse;

#[test]
fn parse_full_index() {
    let content = read_fixture("index/full-index.json");
    let response: IndexResponse = serde_json::from_str(&content).unwrap();

    assert_eq!(response.directory.name, "full-index/");
    assert_eq!(response.directory.parent_dir, "../");

    let first_item = &response.directory.item[0];
    assert_eq!(first_item.name, "1993");
    assert_eq!(format!("{:?}", first_item.type_), "Dir");

    assert_eq!(
        first_item
            .last_modified
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        "2025-01-25 01:00:21"
    );
}

#[test]
fn parse_quarter_index() {
    let content = read_fixture("index/full-index-qtr.json");
    let response: IndexResponse = serde_json::from_str(&content).unwrap();

    let item = response
        .directory
        .item
        .iter()
        .find(|i| i.name == "company.idx")
        .unwrap();

    assert_eq!(format!("{:?}", item.type_), "File");
    assert_eq!(item.size, "52453 KB");
}

#[test]
fn parse_daily_index() {
    let content = read_fixture("index/daily-index.json");
    let response: IndexResponse = serde_json::from_str(&content).unwrap();

    assert!(!response.directory.item.is_empty());

    let year_2023 = response
        .directory
        .item
        .iter()
        .find(|i| i.name == "2023")
        .unwrap();

    assert_eq!(format!("{:?}", year_2023.type_), "Dir");
    assert_eq!(year_2023.href, "2023/");
    assert_eq!(year_2023.size, "743909 KB");
}

#[test]
fn parse_daily_index_year() {
    let content = read_fixture("index/daily-index-2023.json");
    let response: IndexResponse = serde_json::from_str(&content).unwrap();

    let quarters: Vec<_> = response
        .directory
        .item
        .iter()
        .map(|i| i.name.as_str())
        .collect();

    assert_eq!(quarters, vec!["QTR1", "QTR2", "QTR3", "QTR4"]);

    let qtr1 = response
        .directory
        .item
        .iter()
        .find(|i| i.name == "QTR1")
        .unwrap();

    assert_eq!(format!("{:?}", qtr1.type_), "Dir");
    assert_eq!(qtr1.href, "QTR1/");
    assert_eq!(qtr1.size, "16 KB");
}
