use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

#[derive(Debug, Clone)]
pub enum PublicCorsConfig {
    Disabled,
    AllowAnyOrigin,
}

impl Default for PublicCorsConfig {
    fn default() -> Self {
        Self::Disabled
    }
}

impl PublicCorsConfig {
    pub fn allow_any_origin() -> Self {
        Self::AllowAnyOrigin
    }

    pub fn preflight_response(&self) -> Response {
        let mut response = StatusCode::NO_CONTENT.into_response();
        self.insert_public_headers(response.headers_mut());
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, OPTIONS"),
        );
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("If-None-Match, Content-Type"),
        );
        response
    }

    pub fn insert_public_headers(&self, headers: &mut HeaderMap) {
        match self {
            Self::Disabled => {}
            Self::AllowAnyOrigin => {
                headers.insert(
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );
                headers.insert(
                    header::ACCESS_CONTROL_EXPOSE_HEADERS,
                    HeaderValue::from_static("ETag"),
                );
            }
        }
    }
}
