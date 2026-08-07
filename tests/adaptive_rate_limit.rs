//! Empirical verification that backoff plus adaptive rate limiting actually
//! recovers from server-side throttling.
//!
//! # Why this test exists
//!
//! A production ingestion run against SEC EDGAR produced this, from 91 requests
//! that received a 429:
//!
//! ```text
//! retries used: {1: 91, 2: 91, 3: 91, 4: 91, 5: 91}
//! ```
//!
//! Every request burned all five retries and then failed — a **0 % recovery
//! rate**. Exponential backoff was in place and correct in isolation, but it
//! reschedules a single request without changing the rate the client issues
//! requests at, so each retry woke into the same congestion that rejected it.
//!
//! These tests reproduce that condition against a local server that enforces a
//! fixed quota the way EDGAR does, and assert that the adaptive limiter converges
//! onto the server's capacity instead of hammering through its retry budget.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use edgarkit::{Edgar, EdgarConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A server that admits at most `capacity` requests in any rolling one-second
/// window and answers 429 to everything above it — the shape of SEC EDGAR's fair
/// access rule.
struct ThrottlingServer {
    /// Requests admitted (HTTP 200).
    served: Arc<AtomicUsize>,
    /// Requests rejected (HTTP 429).
    throttled: Arc<AtomicUsize>,
    /// Highest number of requests seen inside any one-second window.
    peak_rate: Arc<AtomicU32>,
    addr: std::net::SocketAddr,
}

impl ThrottlingServer {
    async fn spawn(capacity: usize) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");

        let served = Arc::new(AtomicUsize::new(0));
        let throttled = Arc::new(AtomicUsize::new(0));
        let peak_rate = Arc::new(AtomicU32::new(0));

        let window: Arc<std::sync::Mutex<Vec<Instant>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));

        let (s, t, p, w) = (
            Arc::clone(&served),
            Arc::clone(&throttled),
            Arc::clone(&peak_rate),
            Arc::clone(&window),
        );

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let (s, t, p, w) = (
                    Arc::clone(&s),
                    Arc::clone(&t),
                    Arc::clone(&p),
                    Arc::clone(&w),
                );
                tokio::spawn(async move {
                    // Read the request head; the body is irrelevant here.
                    let mut buf = [0u8; 2048];
                    let _ = socket.read(&mut buf).await;

                    let admitted = {
                        let now = Instant::now();
                        let mut hits = w.lock().expect("window lock");
                        hits.retain(|at| now.duration_since(*at) < Duration::from_secs(1));
                        // Count the arrival regardless of the verdict — this is
                        // load the server had to look at.
                        let in_window = hits.len() + 1;
                        p.fetch_max(in_window as u32, Ordering::Relaxed);
                        if hits.len() < capacity {
                            hits.push(now);
                            true
                        } else {
                            false
                        }
                    };

                    let response = if admitted {
                        s.fetch_add(1, Ordering::Relaxed);
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok"
                    } else {
                        t.fetch_add(1, Ordering::Relaxed);
                        "HTTP/1.1 429 Too Many Requests\r\nContent-Type: text/plain\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    };
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        Self {
            served,
            throttled,
            peak_rate,
            addr,
        }
    }

    fn url(&self) -> String {
        format!("http://{}/Archives/edgar/data/1/filing.txt", self.addr)
    }
}

/// Builds a client whose configured ceiling is well above what the server allows,
/// which is exactly the production situation: a per-process limit of 10 req/s
/// multiplied by dozens of concurrent workers.
fn client(rate_limit: u32) -> Edgar {
    Edgar::with_config(EdgarConfig::new(
        "edgarkit-test test@example.com",
        rate_limit,
        Duration::from_secs(5),
        None,
    ))
    .expect("client")
}

/// Fires `total` requests through `edgar` with `concurrency` in flight, returning
/// (successes, failures).
async fn hammer(edgar: &Edgar, url: &str, total: usize, concurrency: usize) -> (usize, usize) {
    let ok = Arc::new(AtomicUsize::new(0));
    let err = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for chunk in 0..concurrency {
        let edgar = edgar.clone();
        let url = url.to_string();
        let per_worker = total / concurrency + usize::from(chunk < total % concurrency);
        let (ok, err) = (Arc::clone(&ok), Arc::clone(&err));
        handles.push(tokio::spawn(async move {
            for _ in 0..per_worker {
                match edgar.get_bytes(&url).await {
                    Ok(_) => ok.fetch_add(1, Ordering::Relaxed),
                    Err(_) => err.fetch_add(1, Ordering::Relaxed),
                };
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }

    (ok.load(Ordering::Relaxed), err.load(Ordering::Relaxed))
}

/// The headline result: a client configured four times above the server's
/// capacity still lands every request.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_request_lands_when_the_client_is_over_the_server_quota() {
    let server = ThrottlingServer::spawn(5).await;
    let edgar = client(20);

    let (ok, failed) = hammer(&edgar, &server.url(), 60, 8).await;

    println!(
        "served={} throttled={} peak_in_window={} final_rate={} req/s",
        server.served.load(Ordering::Relaxed),
        server.throttled.load(Ordering::Relaxed),
        server.peak_rate.load(Ordering::Relaxed),
        edgar.current_rate_limit()
    );

    assert_eq!(failed, 0, "no request may be abandoned: {} failed", failed);
    assert_eq!(ok, 60);
}

/// The mechanism behind that result: the limiter finds the server's capacity
/// instead of either sitting at its configured ceiling or collapsing to the floor.
///
/// Long enough to reach steady state on purpose. A short run only shows the
/// descent — halving is coarse and overshoots — and measuring there says nothing
/// about where the control loop actually settles.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_limiter_converges_on_the_servers_capacity() {
    const CAPACITY: usize = 20;
    let server = ThrottlingServer::spawn(CAPACITY).await;
    let edgar = client(64);
    assert_eq!(edgar.current_rate_limit(), 64);

    let started = Instant::now();
    let (ok, failed) = hammer(&edgar, &server.url(), 400, 8).await;
    let elapsed = started.elapsed();

    let settled = edgar.current_rate_limit();
    let achieved = ok as f64 / elapsed.as_secs_f64();
    println!(
        "configured=64 capacity={CAPACITY} settled={settled} req/s \
         achieved={achieved:.1} req/s ok={ok} failed={failed} throttled={}",
        server.throttled.load(Ordering::Relaxed)
    );

    assert_eq!(failed, 0, "no request may be abandoned");
    assert!(
        settled < 64,
        "the limiter must come down off its ceiling, stayed at {settled}"
    );
    assert!(
        settled > 1,
        "the limiter must recover after overshooting, stuck at {settled}"
    );
    // Throughput is the honest measure of convergence: whatever the instantaneous
    // rate reads, the run as a whole should be moving at roughly what the server
    // will accept rather than crawling at the floor.
    assert!(
        achieved > CAPACITY as f64 * 0.4,
        "throughput {achieved:.1} req/s is far below the {CAPACITY} req/s on offer"
    );
}

/// The control. This is the pre-fix behaviour — fixed rate, retry-only — run
/// against the same server, and it is what a 0 % recovery rate looks like.
///
/// Requests are issued at a constant rate regardless of push-back; each rejected
/// one sleeps and retries into the same congestion. It is written out longhand
/// rather than driven through `Edgar` because the client no longer has a mode
/// that behaves this way, which is the point of the test.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fixed_rate_retry_only_is_what_used_to_fail() {
    const MAX_RETRIES: u32 = 5;

    let server = ThrottlingServer::spawn(4).await;
    let http = reqwest::Client::new();
    let url = server.url();

    let ok = Arc::new(AtomicUsize::new(0));
    let exhausted = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..8 {
        let (http, url) = (http.clone(), url.clone());
        let (ok, exhausted) = (Arc::clone(&ok), Arc::clone(&exhausted));
        handles.push(tokio::spawn(async move {
            for _ in 0..5 {
                let mut retries = 0;
                loop {
                    let status = http.get(&url).send().await.map(|r| r.status());
                    match status {
                        Ok(s) if s.is_success() => {
                            ok.fetch_add(1, Ordering::Relaxed);
                            break;
                        }
                        _ if retries >= MAX_RETRIES => {
                            exhausted.fetch_add(1, Ordering::Relaxed);
                            break;
                        }
                        _ => {
                            // Exponential backoff with the old ±10 % band, and
                            // crucially no change to the issue rate.
                            let base = 20u64 << retries;
                            let jitter = fastrand::u64(0..=base / 5);
                            tokio::time::sleep(Duration::from_millis(base - base / 10 + jitter))
                                .await;
                            retries += 1;
                        }
                    }
                }
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }

    println!(
        "control: ok={} exhausted={} throttled_by_server={}",
        ok.load(Ordering::Relaxed),
        exhausted.load(Ordering::Relaxed),
        server.throttled.load(Ordering::Relaxed)
    );

    // Not an assertion about a specific count — the point is that retry-only
    // leaves failures on the table under sustained push-back, which the adaptive
    // client above does not.
    assert!(
        server.throttled.load(Ordering::Relaxed) > 0,
        "the control must actually be throttled for the comparison to mean anything"
    );
}
