use crate::types::*;
use std::time::Duration;
use tokio::sync::watch;

/// Abstract interface for a model provider.
#[async_trait::async_trait]
pub trait ModelProvider: Send + Sync {
    /// Send a chat request and receive the full response.
    async fn chat(
        &self,
        request: ChatRequest,
        cancel: Option<watch::Receiver<bool>>,
    ) -> Result<ChatResponse, ModelProviderError>;

    /// Send a chat request and receive a stream of events.
    async fn chat_stream(
        &self,
        request: ChatRequest,
        cancel: Option<watch::Receiver<bool>>,
    ) -> Result<
        tokio::sync::mpsc::Receiver<Result<ChatStreamEvent, ModelProviderError>>,
        ModelProviderError,
    >;

    /// Validate that the provider configuration is correct.
    fn validate_config(&self) -> Result<(), ModelProviderError>;
}

/// Retry configuration.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Execute a fallible async operation with exponential backoff retry.
pub async fn retry_with_backoff<F, Fut, T>(
    mut f: F,
    config: &RetryConfig,
    mut cancel: Option<&mut watch::Receiver<bool>>,
) -> Result<T, ModelProviderError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ModelProviderError>>,
{
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        if let Some(ref mut c) = cancel
            && *c.borrow_and_update()
        {
            return Err(ModelProviderError::Cancelled);
        }

        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if !e.is_retryable() || attempt == config.max_attempts {
                    return Err(e);
                }

                let delay_secs = (config.base_delay.as_secs_f64() * 2u64.pow(attempt - 1) as f64)
                    .min(config.max_delay.as_secs_f64());

                let jitter = fastrand::f64() * 0.5 - 0.25;
                let actual_delay = delay_secs * (1.0 + jitter);
                let actual_delay = e.retry_after_secs().unwrap_or(actual_delay);

                tracing::warn!(
                    attempt,
                    max_attempts = config.max_attempts,
                    delay_secs = actual_delay,
                    error = %e,
                    "Retrying model provider call"
                );

                let delay = tokio::time::sleep(Duration::from_secs_f64(actual_delay.max(0.1)));
                tokio::pin!(delay);

                if let Some(ref mut c) = cancel {
                    tokio::select! {
                        _ = &mut delay => {}
                        _ = c.changed() => {
                            if *c.borrow_and_update() {
                                return Err(ModelProviderError::Cancelled);
                            }
                        }
                    }
                } else {
                    delay.await;
                }

                last_error = Some(e);
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| ModelProviderError::Internal("All retry attempts exhausted".into())))
}

/// Build a reqwest Client from a [ProviderInstanceConfig].
pub fn build_http_client(
    config: &ProviderInstanceConfig,
) -> Result<reqwest::Client, ModelProviderError> {
    let mut builder = reqwest::Client::builder()
        .timeout(config.timeout)
        .connect_timeout(Duration::from_secs(10))
        .user_agent("LocalFlow/0.1.0");

    if let Some(ref proxy_url) = config.proxy_url {
        let proxy = reqwest::Proxy::all(proxy_url).map_err(|e| {
            ModelProviderError::Config(format!("Invalid proxy URL '{proxy_url}': {e}"))
        })?;
        builder = builder.proxy(proxy);
    }

    builder
        .build()
        .map_err(|e| ModelProviderError::Internal(format!("Failed to build HTTP client: {e}")))
}
