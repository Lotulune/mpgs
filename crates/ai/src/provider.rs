use async_trait::async_trait;
use std::time::Duration;

use crate::error::AiError;
use crate::types::{
    Embedding, EmbeddingInput, ProviderCapabilities, StructuredRequest, StructuredResponse,
};

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    fn cache_identity(&self) -> String {
        self.name().to_owned()
    }
    fn capabilities(&self) -> ProviderCapabilities;
    fn is_available(&self) -> bool;
    async fn structured_completion(
        &self,
        request: StructuredRequest,
    ) -> Result<StructuredResponse, AiError>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn is_available(&self) -> bool;
    async fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Embedding>, AiError>;
}

/// Always-off provider used when no key is configured or AI is force-disabled.
#[derive(Debug, Default, Clone)]
pub struct DisabledProvider;

#[async_trait]
impl AiProvider for DisabledProvider {
    fn name(&self) -> &str {
        "disabled"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            name: "disabled".into(),
            structured_output: false,
            embeddings: false,
            max_context_tokens: 0,
            embedding_dimensions: None,
        }
    }

    fn is_available(&self) -> bool {
        false
    }

    async fn structured_completion(
        &self,
        _request: StructuredRequest,
    ) -> Result<StructuredResponse, AiError> {
        Err(AiError::Disabled)
    }
}

#[async_trait]
impl EmbeddingProvider for DisabledProvider {
    fn name(&self) -> &str {
        "disabled"
    }

    fn dimensions(&self) -> usize {
        0
    }

    fn is_available(&self) -> bool {
        false
    }

    async fn embed(&self, _inputs: &[EmbeddingInput]) -> Result<Vec<Embedding>, AiError> {
        Err(AiError::Disabled)
    }
}

/// Deterministic fake provider for unit/integration tests.
#[derive(Debug, Clone)]
pub struct FakeProvider {
    pub response: serde_json::Value,
    pub fail_with: Option<AiError>,
    pub available: bool,
    pub delay: Duration,
}

impl Default for FakeProvider {
    fn default() -> Self {
        Self {
            response: serde_json::json!({
                "recommendations": [],
                "summary": "",
                "summary_evidence_ids": []
            }),
            fail_with: None,
            available: true,
            delay: Duration::ZERO,
        }
    }
}

#[async_trait]
impl AiProvider for FakeProvider {
    fn name(&self) -> &str {
        "fake"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            name: "fake".into(),
            structured_output: true,
            embeddings: false,
            max_context_tokens: 8_192,
            embedding_dimensions: None,
        }
    }

    fn cache_identity(&self) -> String {
        "fake:fake-model".into()
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn structured_completion(
        &self,
        request: StructuredRequest,
    ) -> Result<StructuredResponse, AiError> {
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        if let Some(error) = &self.fail_with {
            return Err(error.clone());
        }
        if !self.available {
            return Err(AiError::Disabled);
        }
        Ok(StructuredResponse {
            provider: "fake".into(),
            model: "fake-model".into(),
            content: self.response.clone(),
            usage_input: request.system_prompt.len() as u32,
            usage_output: 32,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
            latency_ms: 1,
        })
    }
}

/// Deterministic local embedding for tests and offline smoke (hash bag-of-chars).
#[derive(Debug, Clone)]
pub struct HashEmbeddingProvider {
    pub dimensions: usize,
}

impl Default for HashEmbeddingProvider {
    fn default() -> Self {
        Self { dimensions: 32 }
    }
}

#[async_trait]
impl EmbeddingProvider for HashEmbeddingProvider {
    fn name(&self) -> &str {
        "hash-embed"
    }

    fn dimensions(&self) -> usize {
        self.dimensions.max(1)
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn embed(&self, inputs: &[EmbeddingInput]) -> Result<Vec<Embedding>, AiError> {
        let dims = self.dimensions();
        let mut out = Vec::with_capacity(inputs.len());
        for input in inputs {
            let mut vector = vec![0.0f32; dims];
            for (i, ch) in input.text.chars().enumerate() {
                let idx = (ch as usize).wrapping_add(i).wrapping_mul(2_654_435_761) % dims;
                vector[idx] += 1.0;
            }
            crate::vector::l2_normalize(&mut vector);
            out.push(Embedding {
                id: input.id.clone(),
                model: "hash-embed-v2".into(),
                dimensions: dims,
                vector,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_embedding_uses_the_shared_v2_mapping() {
        let provider = HashEmbeddingProvider { dimensions: 64 };
        let result = provider
            .embed(&[EmbeddingInput {
                id: "one".into(),
                text: "a".into(),
            }])
            .await
            .unwrap();
        assert_eq!(result[0].vector[17], 1.0);
        assert_eq!(
            result[0]
                .vector
                .iter()
                .filter(|value| **value != 0.0)
                .count(),
            1
        );
    }
}
