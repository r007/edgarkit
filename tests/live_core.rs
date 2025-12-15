use edgarkit::{Edgar, EdgarError};

#[tokio::test]
#[ignore]
async fn rate_limiting_and_backoff() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();
    let url = "https://www.sec.gov/files/company_tickers.json";

    for i in 0..15 {
        let result = edgar.get(url).await;
        match result {
            Ok(_) => {}
            Err(EdgarError::RateLimitExceeded) => {
                assert!(i > 5);
                break;
            }
            Err(e) => panic!("Unexpected error: {e}"),
        }
    }
}
