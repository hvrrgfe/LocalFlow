use std::sync::Arc;
use tokio::sync::watch;

use localflow_core::error::CoreResult;
use localflow_secret_vault::SecretVault;
use localflow_security::validate_url;

use crate::r#trait::{ModelProvider, RetryConfig, build_http_client, retry_with_backoff};
use crate::types::*;

/// Custom HTTP provider for arbitrary REST API calls.
pub struct CustomHttpProvider {
    config: ProviderInstanceConfig,
    http_client: reqwest::Client,
    vault: Arc<dyn SecretVault>,
    retry_config: RetryConfig,
    vault_key: String,
}

impl std::fmt::Debug for CustomHttpProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomHttpProvider")
            .field("base_url", &self.config.base_url)
            .field("timeout", &self.config.timeout)
            .finish()
    }
}

impl CustomHttpProvider {
    /// Create a new custom HTTP provider.
    pub fn new(config: ProviderInstanceConfig, vault: Arc<dyn SecretVault>) -> CoreResult<Self> {
        Self::validate_config_inner(&config)
            .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))?;

        let http_client = build_http_client(&config)
            .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))?;

        let vault_key = config.api_key_vault_key.clone();

        let retry_config = RetryConfig {
            max_attempts: 1,
            ..Default::default()
        };

        Ok(Self {
            config,
            http_client,
            vault,
            retry_config,
            vault_key,
        })
    }

    fn validate_config_inner(config: &ProviderInstanceConfig) -> Result<(), ModelProviderError> {
        if config.base_url.is_empty() {
            return Err(ModelProviderError::Config(
                "base_url cannot be empty".into(),
            ));
        }
        if !config.base_url.starts_with("http://") && !config.base_url.starts_with("https://") {
            return Err(ModelProviderError::Config(format!(
                "base_url must start with http:// or https://, got '{}'",
                config.base_url
            )));
        }
        Ok(())
    }

    fn check_url(&self, url: &str) -> Result<(), ModelProviderError> {
        validate_url(url, &self.config.allowed_hosts, self.config.allow_loopback)
            .map_err(|e| ModelProviderError::PermissionDenied(e.to_string()))?;
        Ok(())
    }

    fn log_safe(&self, msg: &str) {
        let safe = localflow_security::redact_sensitive(msg);
        tracing::info!("{safe}");
    }
}

#[async_trait::async_trait]
impl ModelProvider for CustomHttpProvider {
    fn validate_config(&self) -> Result<(), ModelProviderError> {
        Self::validate_config_inner(&self.config)
    }

    async fn chat(
        &self,
        request: ChatRequest,
        cancel: Option<watch::Receiver<bool>>,
    ) -> Result<ChatResponse, ModelProviderError> {
        let url_text = request
            .messages
            .first()
            .map(|m| m.content.as_str())
            .unwrap_or(&self.config.base_url);

        let url = if url_text.starts_with("http") {
            url_text.to_string()
        } else {
            format!("{}{}", self.config.base_url.trim_end_matches('/'), url_text)
        };

        self.check_url(&url)?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        if !self.vault_key.is_empty() {
            match self.vault.get(&self.vault_key) {
                Ok(key) => {
                    if let Ok(auth_value) =
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {key}"))
                    {
                        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
                    }
                }
                Err(e) => {
                    return Err(ModelProviderError::Config(format!(
                        "Failed to retrieve API key: {e}"
                    )));
                }
            }
        }

        let body = serde_json::json!({
            "messages": request.messages,
            "temperature": request.temperature.unwrap_or(self.config.default_temperature),
            "max_tokens": request.max_tokens.unwrap_or(self.config.default_max_tokens),
        });

        let max_response_bytes = self.config.max_response_bytes;
        let http_client = self.http_client.clone();

        self.log_safe(&format!("Sending custom HTTP request to url='{url}'"));

        let mut cancel_mut = cancel;
        let cancel_ref = cancel_mut.as_mut();

        let result = retry_with_backoff(
            || {
                let url = url.clone();
                let headers = headers.clone();
                let body = body.clone();
                let client = http_client.clone();
                async move {
                    let response = client
                        .post(&url)
                        .headers(headers)
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| {
                            #[allow(clippy::if_same_then_else)]
                            if e.is_timeout() {
                                ModelProviderError::Timeout(e.to_string())
                            } else if e.is_connect() {
                                ModelProviderError::Connection(e.to_string())
                            } else if e.is_body() || e.is_decode() {
                                ModelProviderError::Connection(e.to_string())
                            } else {
                                ModelProviderError::Connection(e.to_string())
                            }
                        })?;

                    let status = response.status();
                    let body_bytes = response
                        .bytes()
                        .await
                        .map_err(|e| ModelProviderError::Connection(e.to_string()))?;

                    if body_bytes.len() as u64 > max_response_bytes {
                        return Err(ModelProviderError::InvalidResponse(format!(
                            "Response body ({} bytes) exceeds limit of {} bytes",
                            body_bytes.len(),
                            max_response_bytes,
                        )));
                    }

                    if !status.is_success() {
                        let body_str = String::from_utf8_lossy(&body_bytes);
                        let redacted = localflow_security::redact_sensitive(&body_str);
                        return match status.as_u16() {
                            401 | 403 => Err(ModelProviderError::Authentication(redacted)),
                            429 => Err(ModelProviderError::RateLimited { retry_after: None }),
                            413 => Err(ModelProviderError::RequestTooLarge(redacted)),
                            s if (500..=599).contains(&s) => Err(ModelProviderError::ServerError {
                                status: s,
                                body: redacted,
                            }),
                            s => Err(ModelProviderError::ServerError {
                                status: s,
                                body: redacted,
                            }),
                        };
                    }

                    let response_text = String::from_utf8_lossy(&body_bytes).to_string();

                    Ok(ChatResponse {
                        id: uuid::Uuid::new_v4().to_string(),
                        model: "custom-http".into(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0),
                        usage: None,
                        choices: vec![ChatChoice {
                            index: 0,
                            message: ChatMessage::assistant(response_text),
                            finish_reason: "stop".into(),
                        }],
                    })
                }
            },
            &self.retry_config,
            cancel_ref,
        )
        .await;

        match &result {
            Ok(_) => self.log_safe("Custom HTTP request completed successfully"),
            Err(e) => {
                let safe_err = localflow_security::redact_sensitive(&e.to_string());
                tracing::error!("Custom HTTP request failed: {safe_err}");
            }
        }

        result
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
        _cancel: Option<watch::Receiver<bool>>,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<ChatStreamEvent, ModelProviderError>>,
        ModelProviderError,
    > {
        Err(ModelProviderError::Internal(
            "Streaming not yet implemented".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use localflow_secret_vault::InMemoryVault;
    use std::sync::Arc;

    fn make_provider() -> CustomHttpProvider {
        let vault = Arc::new(InMemoryVault::new());
        vault.store("test-custom-key", "sk-test-key").unwrap();
        let config = ProviderInstanceConfig {
            base_url: "https://my-api.example.com".into(),
            api_key_vault_key: "test-custom-key".into(),
            default_model: "custom".into(),
            timeout: std::time::Duration::from_secs(5),
            ..Default::default()
        };
        CustomHttpProvider::new(config, vault).unwrap()
    }

    #[test]
    fn test_validate_valid_config() {
        let provider = make_provider();
        assert!(provider.validate_config().is_ok());
    }

    #[test]
    fn test_validate_empty_base_url() {
        let vault = Arc::new(InMemoryVault::new());
        let config = ProviderInstanceConfig {
            base_url: "".into(),
            ..Default::default()
        };
        let result = CustomHttpProvider::new(config, vault);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_protocol() {
        let vault = Arc::new(InMemoryVault::new());
        let config = ProviderInstanceConfig {
            base_url: "ftp://evil.com".into(),
            ..Default::default()
        };
        let result = CustomHttpProvider::new(config, vault);
        assert!(result.is_err());
    }

    #[test]
    fn test_non_idempotent_no_retry_by_default() {
        let provider = make_provider();
        assert_eq!(provider.retry_config.max_attempts, 1);
    }

    #[test]
    fn test_ssrf_rejection_via_security() {
        let vault = Arc::new(InMemoryVault::new());
        let config = ProviderInstanceConfig {
            base_url: "https://api.example.com".into(),
            api_key_vault_key: "k".into(),
            ..Default::default()
        };
        let provider = CustomHttpProvider::new(config, vault).unwrap();
        let result = provider.check_url("http://localhost:8080/admin");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ModelProviderError::PermissionDenied(_)
        ));
    }

    #[test]
    fn test_public_url_allowed() {
        let provider = make_provider();
        let result = provider.check_url("https://api.example.com/v1/chat");
        assert!(result.is_ok());
    }
}
