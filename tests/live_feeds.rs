use edgarkit::{Edgar, FeedOperations, FeedOptions};

#[tokio::test]
#[ignore]
async fn current_feed() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let feed = edgar.current_feed(None).await.unwrap();
    assert!(!feed.entries.is_empty());
}

#[tokio::test]
#[ignore]
async fn current_feed_with_options() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let params = FeedOptions::new(None)
        .with_param("count", "10")
        .with_param("type", "10-K");

    let feed = edgar.current_feed(Some(params)).await.unwrap();
    assert!(!feed.entries.is_empty());
}

#[tokio::test]
#[ignore]
async fn company_feed() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let feed = edgar.company_feed("320193", None).await.unwrap();
    assert!(!feed.entries.is_empty());
}

#[tokio::test]
#[ignore]
async fn company_feed_with_options() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let params = FeedOptions::new(None)
        .with_param("count", "10")
        .with_param("type", "10-K");

    let feed = edgar.company_feed("320193", Some(params)).await.unwrap();
    assert!(!feed.entries.is_empty());
}

#[tokio::test]
#[ignore]
async fn press_release_feed() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let feed = edgar.press_release_feed().await.unwrap();
    assert!(!feed.channel.items.is_empty());
}

#[tokio::test]
#[ignore]
async fn historical_xbrl_feed() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let feed = edgar.historical_xbrl_feed(2021, 1).await.unwrap();
    assert!(!feed.channel.items.is_empty());
}
