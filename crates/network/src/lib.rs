//! Networking stack for NaBrowse.
//!
//! HTTP/1.1 client over TCP with optional TLS (rustls). DNS resolution via
//! the OS resolver (tokio). No telemetry, no tracking, no phone-home. This
//! crate is privacy by design: it only connects to URLs the user requests.

use http::Request;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::debug;
use url::Url;

use std::sync::Arc;

/// Result type for network operations.
pub type NetworkResult<T> = Result<T, NetworkError>;

/// Network errors.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("URL parse error: {0}")]
    UrlParse(String),
    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("TLS handshake failed: {0}")]
    TlsHandshake(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP parse error: {0}")]
    HttpParse(String),
}

/// A fetched HTTP response.
pub struct FetchResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: String,
}

/// The network client.
pub struct NetworkClient {
    tls_connector: TlsConnector,
}

impl NetworkClient {
    /// Create a new network client with system root certificates.
    pub fn new() -> NetworkResult<Self> {
        let root_store: rustls::RootCertStore =
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned().collect();

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let tls_connector = TlsConnector::from(Arc::new(config));

        Ok(Self { tls_connector })
    }

    /// Fetch a URL and return the full response (headers + body).
    pub async fn fetch(&self, url_str: &str) -> NetworkResult<FetchResponse> {
        let parsed = Url::parse(url_str).map_err(|e| NetworkError::UrlParse(e.to_string()))?;

        let scheme = parsed.scheme();
        let host = parsed
            .host_str()
            .ok_or_else(|| NetworkError::UrlParse("no host".to_string()))?;
        let port = parsed.port_or_known_default().unwrap_or(if scheme == "https" { 443 } else { 80 });
        let path = if parsed.path().is_empty() { "/" } else { parsed.path() };
        let query = parsed.query().map(|q| format!("?{}", q)).unwrap_or_default();
        let full_path = format!("{}{}", path, query);

        debug!("Fetching {}://{}:{} {}", scheme, host, port, full_path);

        let addr = format!("{}:{}", host, port);
        let tcp = TcpStream::connect(addr)
            .await
            .map_err(|e| NetworkError::Connection(e.to_string()))?;

        let request = Request::builder()
            .method("GET")
            .uri(full_path)
            .version(http::Version::HTTP_11)
            .header("Host", host)
            .header("Connection", "close")
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) NaBrowse/0.1.0 Chrome/120.0.0.0 Safari/537.36")
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Accept-Encoding", "identity")
            .body(())
            .map_err(|e| NetworkError::Http(e.to_string()))?;

        let request_bytes = format!(
            "{} {}\r\n{}\r\n",
            request.method(),
            request.uri(),
            request
                .headers()
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.to_str().unwrap_or("")))
                .collect::<Vec<_>>()
                .join("\r\n")
        )
        .into_bytes();

        let buffer = if scheme == "https" {
            let domain = rustls::pki_types::ServerName::try_from(host.to_string())
                .map_err(|_| NetworkError::TlsHandshake("invalid hostname".to_string()))?;
            let tls_stream = self
                .tls_connector
                .connect(domain, tcp)
                .await
                .map_err(|e| NetworkError::TlsHandshake(e.to_string()))?;
            http_transfer(tls_stream, &request_bytes).await?
        } else {
            http_transfer(tcp, &request_bytes).await?
        };

        let response_text = String::from_utf8_lossy(&buffer);

        // Split headers and body. If no separator found (partial response),
        // treat entire buffer as head with empty body.
        let (head, body) = match response_text.split_once("\r\n\r\n") {
            Some((h, b)) => (h, b),
            None => (response_text.as_ref(), ""),
        };

        // Parse status line.
        let status_line = head.lines().next().unwrap_or("HTTP/1.1 200 OK");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(200);

        // Parse headers.
        let mut headers = Vec::new();
        for line in head.lines().skip(1) {
            if let Some((key, value)) = line.split_once(':') {
                headers.push((key.trim().to_string(), value.trim().to_string()));
            }
        }

        Ok(FetchResponse {
            status,
            headers,
            body: body.as_bytes().to_vec(),
            url: url_str.to_string(),
        })
    }
}

/// Perform an HTTP request/response transfer over any async stream.
async fn http_transfer<S>(
    stream: S,
    request_bytes: &[u8],
) -> NetworkResult<Vec<u8>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    writer
        .write_all(request_bytes)
        .await
        .map_err(|e| NetworkError::Io(e))?;
    writer.shutdown().await.map_err(|e| NetworkError::Io(e))?;
    let mut buffer = Vec::new();
    let mut tmp = [0u8; 8192];
    loop {
        match reader.read(&mut tmp).await {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&tmp[..n]),
            Err(e) => {
                let msg = e.to_string();
                // Some servers close TLS without sending close_notify.
                // If we already received data, treat this as success.
                if msg.contains("close_notify") && !buffer.is_empty() {
                    break;
                }
                // If buffer is empty and we got a close_notify error,
                // the server closed before sending any data.
                if buffer.is_empty() && msg.contains("close_notify") {
                    return Err(NetworkError::Connection(
                        "Server closed connection without sending data".to_string(),
                    ));
                }
                return Err(NetworkError::Io(e));
            }
        }
    }
    if buffer.is_empty() {
        return Err(NetworkError::Connection(
            "Server closed connection without sending data".to_string(),
        ));
    }
    Ok(buffer)
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new().expect("failed to create network client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parsing_splits_components() {
        let url = Url::parse("https://example.com:8443/path?q=1").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
        assert_eq!(url.port_or_known_default(), Some(8443));
        assert_eq!(url.path(), "/path");
        assert_eq!(url.query(), Some("q=1"));
    }
}
