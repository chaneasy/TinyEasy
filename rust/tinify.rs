use crate::{credentials::Account, error::Error};
use futures_util::{StreamExt, TryStreamExt};
use reqwest::{Client, Response, Url, header};
use std::{path::Path, time::Duration};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

pub const LIMIT: u32 = 500;
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub valid: bool,
    pub used: u32,
    pub error: Option<Error>,
}
impl Usage {
    pub fn remaining(&self) -> u32 {
        if self.valid {
            LIMIT.saturating_sub(self.used)
        } else {
            0
        }
    }
}
#[derive(Clone)]
pub struct Tinify {
    client: Client,
    endpoint: Url,
}
impl std::fmt::Debug for Tinify {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tinify")
    }
}
impl Tinify {
    pub fn new() -> Result<Self, Error> {
        Self::at("https://api.tinify.com/")
    }
    pub(crate) fn at(endpoint: &str) -> Result<Self, Error> {
        let endpoint = Url::parse(endpoint).map_err(|_| Error::Service)?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(180))
            .user_agent(concat!("TinyEasy/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| Error::Network)?;
        Ok(Self { client, endpoint })
    }
    pub async fn validate(&self, account: &Account) -> Usage {
        let response = self
            .client
            .post(
                self.endpoint
                    .join("shrink")
                    .unwrap_or_else(|_| self.endpoint.clone()),
            )
            .basic_auth("api", Some(&account.key))
            .body(Vec::new())
            .timeout(Duration::from_secs(15))
            .send()
            .await;
        match response {
            Ok(r) => {
                let status = r.status().as_u16();
                let used = count(&r).unwrap_or(0);
                if status == 429 {
                    Usage {
                        valid: true,
                        used: LIMIT,
                        error: Some(Error::Quota),
                    }
                } else if status == 401 || status == 403 {
                    Usage {
                        error: Some(Error::InvalidKey),
                        ..Usage::default()
                    }
                } else if (200..300).contains(&status) || (status == 400 && count(&r).is_some()) {
                    Usage {
                        valid: true,
                        used,
                        error: None,
                    }
                } else {
                    Usage {
                        error: Some(classify(status)),
                        ..Usage::default()
                    }
                }
            }
            Err(_) => Usage {
                error: Some(Error::Network),
                ..Usage::default()
            },
        }
    }
    pub async fn compress(
        &self,
        account: &Account,
        input: &Path,
        destination: &Path,
        progress: impl Fn(f32) + Send + Sync + Clone + 'static,
        usage: impl Fn(u32) + Send,
    ) -> Result<(), Error> {
        let file = tokio::fs::File::open(input).await?;
        let size = file.metadata().await?.len();
        let mut uploaded = 0u64;
        let upload_progress = progress.clone();
        let stream = ReaderStream::with_capacity(file, 64 * 1024).inspect_ok(move |chunk| {
            uploaded += chunk.len() as u64;
            upload_progress((uploaded as f32 / size.max(1) as f32 * 70.).min(70.));
        });
        let response = self
            .client
            .post(self.endpoint.join("shrink").map_err(|_| Error::Service)?)
            .basic_auth("api", Some(&account.key))
            .header(header::CONTENT_LENGTH, size)
            .body(reqwest::Body::wrap_stream(stream))
            .send()
            .await
            .map_err(|_| Error::Network)?;
        if let Some(used) = count(&response) {
            usage(used);
        }
        let status = response.status().as_u16();
        if status == 429 {
            usage(LIMIT);
        }
        if !(200..300).contains(&status) {
            return Err(classify(status));
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::Service)?;
        let url = Url::parse(location).map_err(|_| Error::Service)?;
        // Never forward credentials to a server-selected foreign host or insecure URL.
        if url.scheme() != self.endpoint.scheme()
            || url.host_str() != self.endpoint.host_str()
            || url.port_or_known_default() != self.endpoint.port_or_known_default()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::Service);
        }
        let response = self
            .client
            .get(url)
            .basic_auth("api", Some(&account.key))
            .send()
            .await
            .map_err(|_| Error::Network)?;
        if let Some(used) = count(&response) {
            usage(used);
        }
        if !response.status().is_success() {
            return Err(classify(response.status().as_u16()));
        }
        progress(72.);
        let expected = response.content_length();
        let mut output = tokio::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(destination)
            .await?;
        let mut stream = response.bytes_stream();
        let mut received = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| Error::Network)?;
            received += chunk.len() as u64;
            output.write_all(&chunk).await?;
            progress(
                expected
                    .map(|n| (72. + received as f32 / n.max(1) as f32 * 26.).min(98.))
                    .unwrap_or(90.),
            );
        }
        if received == 0 || expected.is_some_and(|len| len != received) {
            return Err(Error::Network);
        }
        output.flush().await?;
        output.sync_all().await?;
        Ok(())
    }
}
fn count(response: &Response) -> Option<u32> {
    response
        .headers()
        .get("compression-count")?
        .to_str()
        .ok()?
        .parse()
        .ok()
}
fn classify(status: u16) -> Error {
    match status {
        401 | 403 => Error::InvalidKey,
        429 => Error::Quota,
        400 | 415 | 422 => Error::Unsupported,
        _ => Error::Service,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chacha20poly1305::aead::{OsRng, rand_core::RngCore};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    pub fn random_account() -> Account {
        let mut b = [0; 16];
        OsRng.fill_bytes(&mut b);
        Account {
            key: b.iter().map(|b| format!("{b:02x}")).collect(),
            note: String::new(),
        }
    }
    async fn server(status: u16, count: u32) -> Tinify {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 {status} Test\r\nCompression-Count: {count}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        Tinify::at(&format!("http://{addr}/")).unwrap()
    }
    #[tokio::test]
    async fn validation_distinguishes_invalid_exhausted_and_valid_keys() {
        let account = random_account();
        assert_eq!(
            server(401, 0).await.validate(&account).await.error,
            Some(Error::InvalidKey)
        );
        assert_eq!(
            server(429, 500).await.validate(&account).await.remaining(),
            0
        );
        assert_eq!(
            server(400, 10).await.validate(&account).await.remaining(),
            490
        );
        assert_eq!(
            server(503, 10).await.validate(&account).await.error,
            Some(Error::Service)
        );
    }
    #[tokio::test]
    async fn connection_failure_is_recoverable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        assert_eq!(
            Tinify::at(&format!("http://{addr}/"))
                .unwrap()
                .validate(&random_account())
                .await
                .error,
            Some(Error::Network)
        );
    }
}

#[cfg(test)]
pub(crate) mod mock {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    pub struct Mock {
        pub client: Tinify,
        pub active: Arc<AtomicUsize>,
        pub peak: Arc<AtomicUsize>,
        pub uploads: Arc<AtomicUsize>,
        task: tokio::task::JoinHandle<()>,
    }
    impl Drop for Mock {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    pub async fn start(status: u16, partial: bool, foreign: bool) -> Mock {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let uploads = Arc::new(AtomicUsize::new(0));
        let (a, p, u) = (active.clone(), peak.clone(), uploads.clone());
        let task = tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let (a, p, u) = (a.clone(), p.clone(), u.clone());
                tokio::spawn(async move {
                    let mut request = vec![];
                    let mut buf = [0; 4096];
                    let (is_post, length, header_len) = loop {
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        if n == 0 {
                            return;
                        }
                        request.extend_from_slice(&buf[..n]);
                        if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                            // Headers contain generated test credentials; never log or snapshot them.
                            let header = String::from_utf8_lossy(&request[..end]);
                            let length = header
                                .lines()
                                .find_map(|l| {
                                    let (k, v) = l.split_once(':')?;
                                    k.eq_ignore_ascii_case("content-length")
                                        .then(|| v.trim().parse::<usize>().ok())
                                        .flatten()
                                })
                                .unwrap_or(0);
                            break (header.starts_with("POST"), length, end + 4);
                        }
                    };
                    while request.len() < header_len + length {
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        request.extend_from_slice(&buf[..n]);
                    }
                    if is_post {
                        u.fetch_add(1, Ordering::SeqCst);
                        let current = a.fetch_add(1, Ordering::SeqCst) + 1;
                        p.fetch_max(current, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        let location = if foreign {
                            "http://localhost:1/foreign".to_owned()
                        } else {
                            format!("http://{addr}/output")
                        };
                        let response = format!(
                            "HTTP/1.1 {status} Test\r\nCompression-Count: 1\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        let _ = socket.write_all(response.as_bytes()).await;
                        a.fetch_sub(1, Ordering::SeqCst);
                    } else {
                        let bytes = if partial {
                            b"partial".as_slice()
                        } else {
                            b"compressed image".as_slice()
                        };
                        let expected = if partial { 500 } else { bytes.len() };
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {expected}\r\nConnection: close\r\n\r\n"
                        );
                        let _ = socket.write_all(response.as_bytes()).await;
                        let _ = socket.write_all(bytes).await;
                    }
                });
            }
        });
        Mock {
            client: Tinify::at(&format!("http://{addr}/")).unwrap(),
            active,
            peak,
            uploads,
            task,
        }
    }
}
