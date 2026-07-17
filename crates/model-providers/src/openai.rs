use std::sync::Arc;
use tokio::sync::watch;

use localflow_core::error::CoreResult;
use localflow_secret_vault::SecretVault;
use localflow_security::validate_url;

use crate::r#trait::{ModelProvider, RetryConfig, build_http_client, retry_with_backoff};
use crate::types::*;

/// OpenAI-compatible chat model provider.
pub struct OpenAIProvider {
    config: ProviderInstanceConfig,
    http_client: reqwest::Client,
    vault: Arc<dyn SecretVault>,
    retry_config: RetryConfig,
    vault_key: String,
}

impl std::fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("base_url", &self.config.base_url)
            .field("default_model", &self.config.default_model)
            .field("timeout", &self.config.timeout)
            .finish()
    }
}

impl OpenAIProvider {
    /// Create a new OpenAI-compatible provider.
    pub fn new(config: ProviderInstanceConfig, vault: Arc<dyn SecretVault>) -> CoreResult<Self> {
        Self::validate_config_inner(&config)
            .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))?;

        let http_client = build_http_client(&config)
            .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))?;

        let vault_key = config.api_key_vault_key.clone();

        let retry_config = RetryConfig {
            max_attempts: config.max_retries.max(1),
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
        if config.api_key_vault_key.is_empty() {
            return Err(ModelProviderError::Config(
                "api_key_vault_key cannot be empty".into(),
            ));
        }
        if config.default_model.is_empty() {
            return Err(ModelProviderError::Config(
                "default_model cannot be empty".into(),
            ));
        }
        Ok(())
    }

    fn chat_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        format!("{base}/chat/completions")
    }

    fn build_headers(&self, api_key: &str) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if let Ok(auth_value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {api_key}"))
        {
            headers.insert(reqwest::header::AUTHORIZATION, auth_value);
        }
        headers
    }

    fn check_url(&self, url: &str) -> Result<(), ModelProviderError> {
        validate_url(url, &self.config.allowed_hosts, self.config.allow_loopback)
            .map_err(|e| ModelProviderError::PermissionDenied(e.to_string()))?;
        Ok(())
    }

    fn check_request_size(&self, request: &ChatRequest) -> Result<(), ModelProviderError> {
        let bytes = request.estimated_bytes();
        if bytes as u64 > self.config.max_request_bytes {
            return Err(ModelProviderError::RequestTooLarge(format!(
                "Request body estimated at {bytes} bytes, exceeds limit of {} bytes",
                self.config.max_request_bytes,
            )));
        }
        Ok(())
    }

    fn log_safe(&self, msg: &str) {
        let safe = localflow_security::redact_sensitive(msg);
        tracing::info!("{safe}");
    }
}

#[async_trait::async_trait]
impl ModelProvider for OpenAIProvider {
    fn validate_config(&self) -> Result<(), ModelProviderError> {
        Self::validate_config_inner(&self.config)
    }

    async fn chat(
        &self,
        request: ChatRequest,
        cancel: Option<watch::Receiver<bool>>,
    ) -> Result<ChatResponse, ModelProviderError> {
        self.check_request_size(&request)?;

        let api_key = self
            .vault
            .get(&self.vault_key)
            .map_err(|e| ModelProviderError::Config(format!("Failed to retrieve API key: {e}")))?;

        let url = self.chat_url();
        self.check_url(&url)?;

        let headers = self.build_headers(&api_key);

        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature.unwrap_or(self.config.default_temperature),
            "max_tokens": request.max_tokens.unwrap_or(self.config.default_max_tokens),
            "stream": false,
        });

        let max_response_bytes = self.config.max_response_bytes;
        let http_client = self.http_client.clone();

        self.log_safe(&format!(
            "Sending chat request to url='{url}' model='{}'",
            request.model
        ));

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
                    let retry_after_hdr = response
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<f64>().ok());
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
                            429 => Err(ModelProviderError::RateLimited {
                                retry_after: retry_after_hdr,
                            }),
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

                    #[derive(serde::Deserialize)]
                    struct OpenAIChoice {
                        index: u32,
                        message: OpenAIMessage,
                        finish_reason: Option<String>,
                    }
                    #[derive(serde::Deserialize)]
                    struct OpenAIMessage {
                        role: String,
                        content: String,
                    }
                    #[derive(serde::Deserialize)]
                    struct OpenAIResponse {
                        id: String,
                        created: u64,
                        model: String,
                        choices: Vec<OpenAIChoice>,
                        usage: Option<TokenUsage>,
                    }

                    let response: OpenAIResponse =
                        serde_json::from_slice(&body_bytes).map_err(|e| {
                            let preview =
                                String::from_utf8_lossy(&body_bytes[..body_bytes.len().min(200)]);
                            ModelProviderError::InvalidResponse(format!(
                                "Failed to parse API response: {e}. Preview: {preview}"
                            ))
                        })?;

                    Ok(ChatResponse {
                        id: response.id,
                        model: response.model,
                        created: response.created,
                        usage: response.usage,
                        choices: response
                            .choices
                            .into_iter()
                            .map(|c| ChatChoice {
                                index: c.index,
                                message: ChatMessage {
                                    role: match c.message.role.as_str() {
                                        "assistant" => MessageRole::Assistant,
                                        "system" => MessageRole::System,
                                        "user" => MessageRole::User,
                                        "tool" => MessageRole::Tool,
                                        _ => MessageRole::Assistant,
                                    },
                                    content: c.message.content,
                                    name: None,
                                },
                                finish_reason: c.finish_reason.unwrap_or_default(),
                            })
                            .collect(),
                    })
                }
            },
            &self.retry_config,
            cancel_ref,
        )
        .await;

        match &result {
            Ok(_) => self.log_safe("Chat request completed successfully"),
            Err(e) => {
                let safe_err = localflow_security::redact_sensitive(&e.to_string());
                tracing::error!("Chat request failed: {safe_err}");
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

    fn make_provider() -> OpenAIProvider {
        let vault = Arc::new(InMemoryVault::new());
        vault.store("test-openai-key", "sk-test-key-12345").unwrap();
        let config = ProviderInstanceConfig {
            base_url: "https://api.openai.com/v1".into(),
            api_key_vault_key: "test-openai-key".into(),
            default_model: "gpt-4o".into(),
            timeout: std::time::Duration::from_secs(5),
            max_retries: 1,
            ..Default::default()
        };
        OpenAIProvider::new(config, vault).unwrap()
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
            api_key_vault_key: "key".into(),
            default_model: "gpt-4".into(),
            ..Default::default()
        };
        let result = OpenAIProvider::new(config, vault);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_protocol() {
        let vault = Arc::new(InMemoryVault::new());
        let config = ProviderInstanceConfig {
            base_url: "ftp://api.example.com".into(),
            api_key_vault_key: "key".into(),
            default_model: "gpt-4".into(),
            ..Default::default()
        };
        let result = OpenAIProvider::new(config, vault);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_vault_key() {
        let vault = Arc::new(InMemoryVault::new());
        let config = ProviderInstanceConfig {
            base_url: "https://api.openai.com".into(),
            api_key_vault_key: "".into(),
            default_model: "gpt-4".into(),
            ..Default::default()
        };
        let result = OpenAIProvider::new(config, vault);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_headers_no_api_key_exposure() {
        let provider = make_provider();
        let headers = provider.build_headers("sk-test-secret-12345");
        let auth = headers.get(reqwest::header::AUTHORIZATION).unwrap();
        let auth_str = auth.to_str().unwrap();
        assert_eq!(auth_str, "Bearer sk-test-secret-12345");
        let log_input = format!("Authorization: {auth_str}");
        let redacted = localflow_security::redact_sensitive(&log_input);
        assert!(!redacted.contains("sk-test-secret-12345"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_chat_url_construction() {
        let provider = make_provider();
        let url = provider.chat_url();
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn test_url_trailing_slash_handled() {
        let vault = Arc::new(InMemoryVault::new());
        vault.store("k", "v").unwrap();
        let config = ProviderInstanceConfig {
            base_url: "https://api.openai.com/v1/".into(),
            api_key_vault_key: "k".into(),
            default_model: "gpt-4".into(),
            ..Default::default()
        };
        let provider = OpenAIProvider::new(config, vault).unwrap();
        assert_eq!(
            provider.chat_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_request_size_validation() {
        let vault = Arc::new(InMemoryVault::new());
        vault.store("k", "v").unwrap();
        let config = ProviderInstanceConfig {
            base_url: "https://api.openai.com/v1".into(),
            api_key_vault_key: "k".into(),
            default_model: "gpt-4".into(),
            max_request_bytes: 100,
            ..Default::default()
        };
        let provider = OpenAIProvider::new(config, vault).unwrap();
        let large_msg = "A".repeat(1000);
        let request = ChatRequest::new("gpt-4", vec![ChatMessage::user(large_msg)]);
        let result = provider.check_request_size(&request);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ModelProviderError::RequestTooLarge(_)
        ));
    }

    #[tokio::test]
    async fn test_retryable_error_detection() {
        assert!(ModelProviderError::Timeout("connection took too long".into()).is_retryable());
        assert!(ModelProviderError::Connection("connection refused".into()).is_retryable());
        assert!(
            ModelProviderError::ServerError {
                status: 500,
                body: "internal error".into()
            }
            .is_retryable()
        );
        assert!(
            ModelProviderError::ServerError {
                status: 503,
                body: "service unavailable".into()
            }
            .is_retryable()
        );
        assert!(ModelProviderError::RateLimited { retry_after: None }.is_retryable());
        assert!(!ModelProviderError::Authentication("invalid key".into()).is_retryable());
        assert!(!ModelProviderError::Config("bad config".into()).is_retryable());
        assert!(!ModelProviderError::RequestTooLarge("too big".into()).is_retryable());
        assert!(!ModelProviderError::PermissionDenied("denied".into()).is_retryable());
        assert!(!ModelProviderError::InvalidResponse("bad JSON".into()).is_retryable());
    }

    #[test]
    fn test_message_constructors() {
        let sys = ChatMessage::system("You are helpful");
        assert_eq!(sys.role, MessageRole::System);
        assert_eq!(sys.content, "You are helpful");
        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, MessageRole::User);
        let asst = ChatMessage::assistant("Hi there");
        assert_eq!(asst.role, MessageRole::Assistant);
    }

    #[test]
    fn test_chat_response_text() {
        let response = ChatResponse {
            id: "chatcmpl-123".into(),
            model: "gpt-4o".into(),
            created: 1700000000,
            usage: None,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage::assistant("Hello, world!"),
                finish_reason: "stop".into(),
            }],
        };
        assert_eq!(response.text(), Some("Hello, world!"));
    }

    #[test]
    fn test_chat_response_empty_choices() {
        let response = ChatResponse {
            id: "chatcmpl-123".into(),
            model: "gpt-4o".into(),
            created: 1700000000,
            usage: None,
            choices: vec![],
        };
        assert!(response.text().is_none());
    }
}
