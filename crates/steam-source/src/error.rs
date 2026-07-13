use thiserror::Error;

/// Stable error categories for Steam source adapters.
///
/// Callers must branch on these variants, not on vendor error strings.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceError {
    #[error("response exceeded maximum size of {max_bytes} bytes")]
    ResponseTooLarge { max_bytes: usize },

    #[error("HTTP status {status} is not successful")]
    HttpStatus { status: u16 },

    #[error("response body is not valid UTF-8")]
    InvalidUtf8,

    #[error("JSON parse failed: {message}")]
    JsonParse { message: String },

    #[error("response structure is invalid: {message}")]
    InvalidStructure { message: String },

    #[error("rate limited; retry after {retry_after_ms:?} ms")]
    RateLimited { retry_after_ms: Option<u64> },

    #[error("entity not found: {entity_key}")]
    NotFound { entity_key: String },

    #[error("temporary failure: {message}")]
    Temporary { message: String },

    #[error("permanent failure: {message}")]
    Permanent { message: String },

    #[error("configuration error: {message}")]
    Config { message: String },
}

impl SourceError {
    pub fn json_parse(error: impl ToString) -> Self {
        Self::JsonParse {
            message: error.to_string(),
        }
    }

    pub fn invalid_structure(message: impl Into<String>) -> Self {
        Self::InvalidStructure {
            message: message.into(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. }
                | Self::Temporary { .. }
                | Self::HttpStatus {
                    status: 408 | 425 | 429 | 500..=599
                }
        )
    }
}
