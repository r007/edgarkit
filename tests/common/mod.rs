use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub fn fixture_path(relative: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn read_fixture(relative: impl AsRef<Path>) -> String {
    fs::read_to_string(fixture_path(relative)).expect("fixture file should be readable")
}

#[allow(dead_code)]
pub fn edgar() -> edgarkit::Edgar {
    edgarkit::Edgar::new("test_agent example@example.com").unwrap()
}

/// A local stand-in for sec.gov. It answers every GET with whatever `respond`
/// returns for the request's path and query, and records each one it saw.
#[allow(dead_code)]
pub struct FakeSec {
    base: String,
    requests: Arc<Mutex<Vec<String>>>,
}

#[allow(dead_code)]
impl FakeSec {
    pub async fn spawn(respond: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("local addr"));
        let requests = Arc::new(Mutex::new(Vec::new()));

        let seen = Arc::clone(&requests);
        let respond = Arc::new(respond);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let (seen, respond) = (Arc::clone(&seen), Arc::clone(&respond));
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 8192];
                    let read = socket.read(&mut buf).await.unwrap_or(0);
                    let head = String::from_utf8_lossy(&buf[..read]);
                    let target = head.split_whitespace().nth(1).unwrap_or("/").to_string();

                    let body = respond(&target);
                    seen.lock().expect("requests lock").push(target);

                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        Self { base, requests }
    }

    /// Base URLs laid out the way the real hosts are, all pointing at this server.
    pub fn urls(&self) -> edgarkit::EdgarUrls {
        edgarkit::EdgarUrls {
            archives: format!("{}/Archives/edgar", self.base),
            data: self.base.clone(),
            files: format!("{}/files", self.base),
            search: format!("{}/LATEST/search-index/", self.base),
            site: self.base.clone(),
        }
    }

    /// A client wired to this server.
    pub fn edgar(&self) -> edgarkit::Edgar {
        edgarkit::Edgar::with_config(edgarkit::EdgarConfig::new(
            "test_agent example@example.com",
            100,
            Duration::from_secs(10),
            Some(self.urls()),
        ))
        .expect("edgar config")
    }

    /// Path and query of every request served so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("requests lock").clone()
    }
}
