use edgarkit::{Edgar, EdgarError, FilingOperations};

#[tokio::test]
#[ignore]
async fn rate_limiting_and_backoff() {
    let edgar = Edgar::new("test_agent example@example.com").unwrap();

    // Exercise the HTTP client's rate limiter by issuing many requests via a public API method.
    for i in 0..15 {
        let result = edgar.submissions("320193").await;
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
