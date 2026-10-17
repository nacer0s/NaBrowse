//! Minimal URL wrapper shared across crates.
//!
//! The full URL parsing lives in `nabrowse-network` (using the `url` crate).
//! This provides a lightweight struct used by the shell and renderer.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NabrowseUrl {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
}

impl NabrowseUrl {
    pub fn parse(input: &str) -> Option<Self> {
        let parsed = url::Url::parse(input).ok()?;
        Some(Self {
            scheme: parsed.scheme().to_string(),
            host: parsed.host_str()?.to_string(),
            port: parsed.port(),
            path: parsed.path().to_string(),
            query: parsed.query().map(|q| q.to_string()),
        })
    }

    pub fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.scheme);
        s.push_str("://");
        s.push_str(&self.host);
        if let Some(port) = self.port {
            s.push(':');
            s.push_str(&port.to_string());
        }
        s.push_str(&self.path);
        if let Some(query) = &self.query {
            s.push('?');
            s.push_str(query);
        }
        s
    }
}

impl Default for NabrowseUrl {
    fn default() -> Self {
        Self {
            scheme: "https".to_string(),
            host: "example.com".to_string(),
            port: None,
            path: "/".to_string(),
            query: None,
        }
    }
}
