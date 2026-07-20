//! Controlled web discovery provider abstraction (PRD AI-013 / AI-016).
//!
//! Chat/Responses completions are **not** web search. Only an independent
//! search provider (or a probed tool) may produce Web evidence rows.

use std::collections::HashSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AiError;

/// Source tier for web evidence ranking (never becomes Steam authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceTier {
    Official,
    Developer,
    Community,
    Unknown,
}

impl SourceTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Developer => "developer",
            Self::Community => "community",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_host(host: &str, whitelist: &SourceWhitelist) -> Self {
        let host = host.trim().to_ascii_lowercase();
        if whitelist.official.iter().any(|h| host_matches(&host, h)) {
            return Self::Official;
        }
        if whitelist.developer.iter().any(|h| host_matches(&host, h)) {
            return Self::Developer;
        }
        if whitelist.community.iter().any(|h| host_matches(&host, h)) {
            return Self::Community;
        }
        Self::Unknown
    }
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let pattern = pattern.trim().to_ascii_lowercase();
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceWhitelist {
    pub official: HashSet<String>,
    pub developer: HashSet<String>,
    pub community: HashSet<String>,
}

impl Default for SourceWhitelist {
    fn default() -> Self {
        Self {
            official: [
                "store.steampowered.com",
                "steamcommunity.com",
                "partner.steamgames.com",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            developer: HashSet::new(),
            community: ["pcgamingwiki.com", "wikipedia.org", "github.com"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}

impl SourceWhitelist {
    pub fn allows(&self, host: &str) -> bool {
        !matches!(SourceTier::from_host(host, self), SourceTier::Unknown)
            || self.community.iter().any(|h| host_matches(host, h))
            || self.official.iter().any(|h| host_matches(host, h))
            || self.developer.iter().any(|h| host_matches(host, h))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchQuery {
    pub query: String,
    pub app_id: Option<u32>,
    pub game_name: Option<String>,
    pub missing_features: Vec<String>,
    pub limit: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchHit {
    pub url: String,
    pub host: String,
    pub title: String,
    pub snippet: String,
    pub source_tier: SourceTier,
    pub content_hash: String,
    pub fetched_at_ms: i64,
}

#[async_trait]
pub trait WebSearchProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
    async fn search(&self, query: &WebSearchQuery) -> Result<Vec<WebSearchHit>, AiError>;
}

/// Always-off provider (default in production until a search key is configured).
#[derive(Debug, Default, Clone)]
pub struct DisabledWebSearchProvider;

#[async_trait]
impl WebSearchProvider for DisabledWebSearchProvider {
    fn name(&self) -> &str {
        "disabled"
    }

    fn is_available(&self) -> bool {
        false
    }

    async fn search(&self, _query: &WebSearchQuery) -> Result<Vec<WebSearchHit>, AiError> {
        Err(AiError::Disabled)
    }
}

/// Deterministic fake search provider for unit/integration tests.
#[derive(Debug, Clone)]
pub struct FakeWebSearchProvider {
    pub hits: Vec<WebSearchHit>,
    pub available: bool,
}

impl Default for FakeWebSearchProvider {
    fn default() -> Self {
        Self {
            hits: Vec::new(),
            available: true,
        }
    }
}

#[async_trait]
impl WebSearchProvider for FakeWebSearchProvider {
    fn name(&self) -> &str {
        "fake_web"
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn search(&self, query: &WebSearchQuery) -> Result<Vec<WebSearchHit>, AiError> {
        if !self.available {
            return Err(AiError::Disabled);
        }
        let limit = usize::from(query.limit.max(1));
        Ok(self.hits.iter().take(limit).cloned().collect())
    }
}

/// Build a stable content hash for dedupe / cache keys.
pub fn web_content_hash(url: &str, title: &str, snippet: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.update([0]);
    hasher.update(title.as_bytes());
    hasher.update([0]);
    hasher.update(snippet.as_bytes());
    let digest = hasher.finalize();
    format!("sha256:{}", hex_encode(&digest))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Extract host from an absolute http(s) URL; rejects credentials/query fragments
/// for evidence storage safety.
pub fn host_from_url(raw: &str) -> Result<String, AiError> {
    let parsed = url::Url::parse(raw.trim())
        .map_err(|error| AiError::InvalidOutput(format!("invalid web URL: {error}")))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err(AiError::InvalidOutput(
            "web evidence URL must be http(s)".into(),
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(AiError::InvalidOutput(
            "web evidence URL must not contain credentials".into(),
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| AiError::InvalidOutput("web evidence URL missing host".into()))?
        .to_ascii_lowercase();
    Ok(host)
}

/// Filter hits to whitelist hosts and attach tiers + content hashes.
pub fn normalize_search_hits(
    hits: impl IntoIterator<Item = (String, String, String)>,
    whitelist: &SourceWhitelist,
    now_ms: i64,
    allow_unknown: bool,
) -> Result<Vec<WebSearchHit>, AiError> {
    let mut out = Vec::new();
    let mut seen_hashes = HashSet::new();
    for (url, title, snippet) in hits {
        let host = host_from_url(&url)?;
        let tier = SourceTier::from_host(&host, whitelist);
        if matches!(tier, SourceTier::Unknown) && !allow_unknown {
            continue;
        }
        if !allow_unknown && !whitelist.allows(&host) {
            continue;
        }
        let content_hash = web_content_hash(&url, &title, &snippet);
        if !seen_hashes.insert(content_hash.clone()) {
            continue;
        }
        out.push(WebSearchHit {
            url,
            host,
            title,
            snippet,
            source_tier: tier,
            content_hash,
            fetched_at_ms: now_ms,
        });
    }
    Ok(out)
}

/// Build a controlled discovery query for a catalog gap.
pub fn discovery_query_for_app(
    app_id: u32,
    game_name: &str,
    missing_features: &[&str],
) -> WebSearchQuery {
    let mut parts = vec![game_name.to_owned(), format!("steam appid {app_id}")];
    for feature in missing_features {
        parts.push((*feature).to_owned());
    }
    WebSearchQuery {
        query: parts.join(" "),
        app_id: Some(app_id),
        game_name: Some(game_name.to_owned()),
        missing_features: missing_features.iter().map(|s| (*s).to_owned()).collect(),
        limit: 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_stable_and_dedupes() {
        let a = web_content_hash("https://a.example/x", "t", "s");
        let b = web_content_hash("https://a.example/x", "t", "s");
        let c = web_content_hash("https://a.example/x", "t", "other");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("sha256:"));
    }

    #[test]
    fn whitelist_classifies_official_and_rejects_unknown_by_default() {
        let wl = SourceWhitelist::default();
        assert_eq!(
            SourceTier::from_host("store.steampowered.com", &wl),
            SourceTier::Official
        );
        let hits = normalize_search_hits(
            [
                (
                    "https://store.steampowered.com/app/1".into(),
                    "Game".into(),
                    "private lobby".into(),
                ),
                (
                    "https://evil.example/phish".into(),
                    "nope".into(),
                    "nope".into(),
                ),
            ],
            &wl,
            100,
            false,
        )
        .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_tier, SourceTier::Official);
    }

    #[test]
    fn discovery_query_includes_missing_features() {
        let q = discovery_query_for_app(548430, "Deep Rock Galactic", &["private_session"]);
        assert!(q.query.contains("Deep Rock"));
        assert!(q.query.contains("548430"));
        assert_eq!(q.missing_features, vec!["private_session".to_owned()]);
    }

    #[tokio::test]
    async fn fake_provider_returns_capped_hits() {
        let provider = FakeWebSearchProvider {
            hits: (0..10)
                .map(|i| WebSearchHit {
                    url: format!("https://store.steampowered.com/app/{i}"),
                    host: "store.steampowered.com".into(),
                    title: format!("t{i}"),
                    snippet: "s".into(),
                    source_tier: SourceTier::Official,
                    content_hash: format!("h{i}"),
                    fetched_at_ms: 1,
                })
                .collect(),
            available: true,
        };
        let hits = provider
            .search(&WebSearchQuery {
                query: "test".into(),
                app_id: None,
                game_name: None,
                missing_features: vec![],
                limit: 3,
            })
            .await
            .unwrap();
        assert_eq!(hits.len(), 3);
    }
}
