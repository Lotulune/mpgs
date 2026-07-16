use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::error::AiError;
use crate::provider::{AiProvider, EmbeddingProvider};
use crate::types::{
    Embedding, EmbeddingInput, ProviderCapabilities, StructuredRequest, StructuredResponse,
};
use crate::vector::l2_normalize;

const MAX_COMPLETION_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_EMBEDDING_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

fn validated_base_url(raw: impl Into<String>, label: &str) -> Result<String, AiError> {
    let raw = raw.into();
    let parsed = reqwest::Url::parse(raw.trim())
        .map_err(|error| AiError::Config(format!("{label} is invalid: {error}")))?;
    let local_http = parsed.scheme() == "http"
        && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if parsed.scheme() != "https" && !local_http {
        return Err(AiError::Config(format!(
            "{label} must be https:// or exact localhost/127.0.0.1/[::1] HTTP"
        )));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AiError::Config(format!(
            "{label} must not contain credentials, query, or fragment"
        )));
    }
    Ok(parsed.as_str().trim_end_matches('/').to_owned())
}

async fn read_limited_body(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<(reqwest::StatusCode, Vec<u8>), AiError> {
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(AiError::InvalidOutput(format!(
            "provider response exceeds {max_bytes} bytes"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| AiError::Transport(error.to_string()))?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            return Err(AiError::InvalidOutput(format!(
                "provider response exceeds {max_bytes} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok((status, body))
}

/// OpenAI-compatible chat completions adapter (official OpenAI or compatible gateways).
#[derive(Debug, Clone)]
pub struct OpenAiCompatProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout: Duration,
    client: reqwest::Client,
}

impl OpenAiCompatProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, AiError> {
        let base_url = validated_base_url(base_url, "AI base URL")?;
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(AiError::Config("AI API key is empty".into()));
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| AiError::Config(error.to_string()))?;
        Ok(Self {
            name: "openai_compat".into(),
            base_url,
            api_key,
            model: model.into(),
            timeout,
            client,
        })
    }
}

#[async_trait]
impl AiProvider for OpenAiCompatProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn cache_identity(&self) -> String {
        format!("{}:{}:{}", self.name, self.base_url, self.model)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            name: self.name.clone(),
            structured_output: true,
            embeddings: false,
            max_context_tokens: 128_000,
            embedding_dimensions: None,
        }
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn structured_completion(
        &self,
        request: StructuredRequest,
    ) -> Result<StructuredResponse, AiError> {
        let started = Instant::now();
        let url = format!("{}/chat/completions", self.base_url);
        let body = json!({
            "model": self.model,
            "temperature": request.temperature,
            "max_tokens": request.max_output_tokens,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": request.json_schema_name,
                    "schema": request.json_schema,
                    "strict": true
                }
            },
            "messages": [
                { "role": "system", "content": request.system_prompt },
                { "role": "user", "content": request.data_prompt }
            ]
        });

        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::Transport(error.to_string())
                }
            })?;

        let (status, bytes) = read_limited_body(response, MAX_COMPLETION_RESPONSE_BYTES).await?;
        if status.as_u16() == 429 {
            return Err(AiError::RateLimited);
        }
        if !status.is_success() {
            let snippet = String::from_utf8_lossy(&bytes);
            let safe = snippet.chars().take(200).collect::<String>();
            return Err(AiError::ProviderRejected(format!(
                "HTTP {} body={safe}",
                status.as_u16()
            )));
        }

        let payload: Value = serde_json::from_slice(&bytes)
            .map_err(|error| AiError::InvalidOutput(error.to_string()))?;
        let content_text = payload
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| AiError::InvalidOutput("missing choices[0].message.content".into()))?;
        let content: Value = serde_json::from_str(content_text)
            .map_err(|error| AiError::InvalidOutput(format!("content is not JSON: {error}")))?;
        let usage_input = payload
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;
        let usage_output = payload
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;

        Ok(StructuredResponse {
            provider: self.name.clone(),
            model: self.model.clone(),
            content,
            usage_input,
            usage_output,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}

/// OpenAI-compatible `/embeddings` adapter.
#[derive(Debug, Clone)]
pub struct OpenAiCompatEmbeddingProvider {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub dimensions: usize,
    pub timeout: Duration,
    client: reqwest::Client,
}

impl OpenAiCompatEmbeddingProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        dimensions: usize,
        timeout: Duration,
    ) -> Result<Self, AiError> {
        let base_url = validated_base_url(base_url, "AI embedding base URL")?;
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(AiError::Config("AI embedding API key is empty".into()));
        }
        if dimensions == 0 {
            return Err(AiError::Config(
                "AI embedding dimensions must be > 0".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| AiError::Config(error.to_string()))?;
        Ok(Self {
            name: "openai_compat_embed".into(),
            base_url,
            api_key,
            model: model.into(),
            dimensions,
            timeout,
            client,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiCompatEmbeddingProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Embedding>, AiError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        if inputs.len() > 64 {
            return Err(AiError::ProviderRejected(
                "embedding batch exceeds 64 inputs".into(),
            ));
        }
        let url = format!("{}/embeddings", self.base_url);
        let body = json!({
            "model": self.model,
            "input": inputs.iter().map(|i| &i.text).collect::<Vec<_>>(),
            "dimensions": self.dimensions,
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    AiError::Timeout
                } else {
                    AiError::Transport(error.to_string())
                }
            })?;
        let (status, bytes) = read_limited_body(response, MAX_EMBEDDING_RESPONSE_BYTES).await?;
        if status.as_u16() == 429 {
            return Err(AiError::RateLimited);
        }
        if !status.is_success() {
            let snippet = String::from_utf8_lossy(&bytes);
            let safe = snippet.chars().take(200).collect::<String>();
            return Err(AiError::ProviderRejected(format!(
                "HTTP {} body={safe}",
                status.as_u16()
            )));
        }
        let payload: Value = serde_json::from_slice(&bytes)
            .map_err(|error| AiError::InvalidOutput(error.to_string()))?;
        let data = payload
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| AiError::InvalidOutput("missing embeddings data array".into()))?;
        if data.len() != inputs.len() {
            return Err(AiError::InvalidOutput(format!(
                "embedding count {} != input count {}",
                data.len(),
                inputs.len()
            )));
        }
        let mut out: Vec<Option<Embedding>> = vec![None; inputs.len()];
        for (position, item) in data.iter().enumerate() {
            let idx = item
                .get("index")
                .and_then(Value::as_u64)
                .map(usize::try_from)
                .transpose()
                .map_err(|_| AiError::InvalidOutput("embedding index is too large".into()))?
                .unwrap_or(position);
            let input = inputs.get(idx).ok_or_else(|| {
                AiError::InvalidOutput(format!("embedding index {idx} is out of range"))
            })?;
            if out[idx].is_some() {
                return Err(AiError::InvalidOutput(format!(
                    "duplicate embedding index {idx}"
                )));
            }
            let values = item
                .get("embedding")
                .and_then(Value::as_array)
                .ok_or_else(|| AiError::InvalidOutput(format!("data[{idx}].embedding missing")))?;
            let mut vector = Vec::with_capacity(values.len());
            for value in values {
                let f = value.as_f64().ok_or_else(|| {
                    AiError::InvalidOutput(format!("data[{idx}].embedding non-float"))
                })? as f32;
                vector.push(f);
            }
            if vector.len() != self.dimensions {
                return Err(AiError::InvalidOutput(format!(
                    "data[{idx}].embedding dimensions {} != configured {}",
                    vector.len(),
                    self.dimensions
                )));
            }
            l2_normalize(&mut vector);
            out[idx] = Some(Embedding {
                id: input.id.clone(),
                model: self.model.clone(),
                dimensions: vector.len(),
                vector,
            });
        }
        out.into_iter()
            .enumerate()
            .map(|(idx, item)| {
                item.ok_or_else(|| AiError::InvalidOutput(format!("missing embedding index {idx}")))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_requires_exact_local_http_host() {
        for invalid in [
            "http://localhost.evil.example/v1",
            "http://localhost@evil.example/v1",
            "http://127.0.0.1.evil.example/v1",
        ] {
            assert!(
                OpenAiCompatProvider::new(invalid, "key", "model", Duration::from_secs(1)).is_err()
            );
        }
        assert!(
            OpenAiCompatProvider::new(
                "http://localhost:1234/v1",
                "key",
                "model",
                Duration::from_secs(1)
            )
            .is_ok()
        );
    }

    #[test]
    fn cache_identity_changes_with_model_or_endpoint() {
        let first = OpenAiCompatProvider::new(
            "https://provider.example/v1",
            "key",
            "model-a",
            Duration::from_secs(1),
        )
        .unwrap();
        let second = OpenAiCompatProvider::new(
            "https://provider.example/v1",
            "key",
            "model-b",
            Duration::from_secs(1),
        )
        .unwrap();
        assert_ne!(first.cache_identity(), second.cache_identity());
    }
}
