use crate::error::SourceError;
use crate::hash::content_hash;

/// Validated raw HTTP response before source-specific parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub content_hash: String,
    pub content_type: Option<String>,
}

impl RawResponse {
    pub const DEFAULT_MAX_BYTES: usize = 8 * 1024 * 1024;

    pub fn validate(
        status: u16,
        body: Vec<u8>,
        content_type: Option<String>,
        max_bytes: usize,
    ) -> Result<Self, SourceError> {
        if body.len() > max_bytes {
            return Err(SourceError::ResponseTooLarge { max_bytes });
        }

        if status == 429 {
            return Err(SourceError::RateLimited {
                retry_after_ms: None,
            });
        }

        if !(200..300).contains(&status) {
            return Err(SourceError::HttpStatus { status });
        }

        let hash = content_hash(&body);
        Ok(Self {
            status,
            body,
            content_hash: hash,
            content_type,
        })
    }

    pub fn as_str(&self) -> Result<&str, SourceError> {
        std::str::from_utf8(&self.body).map_err(|_| SourceError::InvalidUtf8)
    }

    pub fn parse_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, SourceError> {
        let text = self.as_str()?;
        serde_json::from_str(text).map_err(SourceError::json_parse)
    }
}

#[cfg(test)]
mod tests {
    use super::RawResponse;
    use crate::error::SourceError;

    #[test]
    fn rejects_oversized_body() {
        let err = RawResponse::validate(200, vec![0; 16], None, 8).unwrap_err();
        assert_eq!(err, SourceError::ResponseTooLarge { max_bytes: 8 });
    }

    #[test]
    fn accepts_ok_json_body() {
        let raw = RawResponse::validate(200, br#"{"ok":true}"#.to_vec(), None, 1024).unwrap();
        assert_eq!(raw.status, 200);
        assert!(!raw.content_hash.is_empty());
    }
}
