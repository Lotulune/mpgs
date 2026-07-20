//! Cross-origin resource sharing for the desktop webview.
//!
//! The packaged Tauri client is served from a webview origin (e.g.
//! `http://tauri.localhost` on Windows, `tauri://localhost` elsewhere) and calls
//! this server cross-origin, so a strict allowlist must echo the ACAO headers.
//!
//! Hand-rolled (no `tower-http` `cors` feature) to keep the dependency set and
//! `Cargo.lock` unchanged. Policy:
//! - exact-origin allowlist only, never `*`
//! - credentials are never allowed (Bearer tokens travel in the Authorization
//!   header, not cookies)
//! - unknown origins receive no ACAO header, so browsers block them; non-browser
//!   clients are unaffected because they ignore CORS entirely.

use std::env;
use std::io;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "http://tauri.localhost",
    "tauri://localhost",
    "http://localhost:5173",
];

const ALLOW_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";
const ALLOW_HEADERS: &str =
    "authorization, content-type, idempotency-key, if-none-match, x-device-id, x-request-id";
const EXPOSE_HEADERS: &str =
    "etag, x-request-id, retry-after, x-ratelimit-limit, x-ratelimit-remaining";
const MAX_AGE: &str = "600";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfig {
    pub enabled: bool,
    allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_origins: DEFAULT_ALLOWED_ORIGINS
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        }
    }
}

impl CorsConfig {
    /// Build from `MPGS_CORS_ENABLED` and `MPGS_CORS_ALLOWED_ORIGINS`
    /// (comma-separated exact origins). Invalid origins fail startup.
    pub fn from_env() -> Result<Self, io::Error> {
        let enabled = match env::var("MPGS_CORS_ENABLED") {
            Ok(value) => match value.to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" => true,
                "0" | "false" | "no" | "" => false,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "MPGS_CORS_ENABLED must be true/false or 1/0",
                    ));
                }
            },
            Err(_) => true,
        };

        let allowed_origins = match env::var("MPGS_CORS_ALLOWED_ORIGINS") {
            Ok(raw) => {
                let mut origins = Vec::new();
                for part in raw.split(',') {
                    let origin = part.trim();
                    if origin.is_empty() {
                        continue;
                    }
                    validate_origin(origin)?;
                    origins.push(origin.to_owned());
                }
                origins
            }
            Err(_) => DEFAULT_ALLOWED_ORIGINS
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
        };

        Ok(Self {
            enabled,
            allowed_origins,
        })
    }

    fn allows(&self, origin: &str) -> bool {
        self.enabled && self.allowed_origins.iter().any(|allowed| allowed == origin)
    }
}

/// Reject origins that are not `scheme://host[:port]` with no path/query/space.
fn validate_origin(origin: &str) -> Result<(), io::Error> {
    let invalid = |reason: &str| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("MPGS_CORS_ALLOWED_ORIGINS entry '{origin}' is invalid: {reason}"),
        )
    };
    if origin.chars().any(|c| c.is_whitespace()) {
        return Err(invalid("must not contain whitespace"));
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return Err(invalid("must be scheme://host"));
    };
    if !matches!(scheme, "http" | "https" | "tauri") {
        return Err(invalid("scheme must be http, https, or tauri"));
    }
    if rest.is_empty() {
        return Err(invalid("host must not be empty"));
    }
    if rest.contains('/') {
        return Err(invalid("must not contain a path"));
    }
    // Header value safety: the origin is echoed verbatim into ACAO.
    if HeaderValue::from_str(origin).is_err() {
        return Err(invalid("not a valid header value"));
    }
    Ok(())
}

/// CORS middleware. Answers allowed preflights with 204 and decorates simple
/// responses with the ACAO/expose headers. Placed outermost so preflight never
/// reaches auth or rate limiting.
pub async fn middleware(
    State(config): State<Arc<CorsConfig>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let allowed = origin.as_deref().is_some_and(|o| config.allows(o));

    // Preflight: OPTIONS carrying Access-Control-Request-Method.
    let is_preflight = request.method() == Method::OPTIONS
        && request
            .headers()
            .contains_key(header::ACCESS_CONTROL_REQUEST_METHOD);

    if is_preflight {
        let mut response = StatusCode::NO_CONTENT.into_response();
        add_vary(&mut response);
        if allowed && let Some(origin) = origin.as_deref() {
            decorate_allowed(&mut response, origin, true);
        }
        return response;
    }

    let mut response = next.run(request).await;
    add_vary(&mut response);
    if allowed && let Some(origin) = origin.as_deref() {
        decorate_allowed(&mut response, origin, false);
    }
    response
}

fn add_vary(response: &mut Response) {
    // Origin-dependent responses must not be cached across origins.
    response
        .headers_mut()
        .append(header::VARY, HeaderValue::from_static("Origin"));
}

fn decorate_allowed(response: &mut Response, origin: &str, preflight: bool) {
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(origin) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    if preflight {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static(ALLOW_METHODS),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(ALLOW_HEADERS),
        );
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static(MAX_AGE),
        );
    } else {
        headers.insert(
            header::ACCESS_CONTROL_EXPOSE_HEADERS,
            HeaderValue::from_static(EXPOSE_HEADERS),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_tauri_origins() {
        let config = CorsConfig::default();
        assert!(config.allows("http://tauri.localhost"));
        assert!(config.allows("tauri://localhost"));
        assert!(config.allows("http://localhost:5173"));
        assert!(!config.allows("https://evil.example"));
    }

    #[test]
    fn disabled_allows_nothing() {
        let config = CorsConfig {
            enabled: false,
            ..CorsConfig::default()
        };
        assert!(!config.allows("http://tauri.localhost"));
    }

    #[test]
    fn validate_origin_rejects_paths_and_bad_schemes() {
        assert!(validate_origin("http://tauri.localhost").is_ok());
        assert!(validate_origin("https://store.example:8443").is_ok());
        assert!(validate_origin("tauri://localhost").is_ok());
        assert!(validate_origin("http://host/path").is_err());
        assert!(validate_origin("ftp://host").is_err());
        assert!(validate_origin("no-scheme").is_err());
        assert!(validate_origin("http://").is_err());
        assert!(validate_origin("http://ho st").is_err());
    }
}
