use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f64>,
    pub stop: Option<Vec<String>>,
    pub stream: Option<bool>,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            stream: None,
        }
    }
    pub fn estimated_bytes(&self) -> usize {
        let mut total = self.model.len();
        for msg in &self.messages {
            total += msg.content.len() + 32;
        }
        total
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(default)]
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
    #[serde(default)]
    pub created: u64,
}

impl ChatResponse {
    pub fn text(&self) -> Option<&str> {
        self.choices.first().map(|c| c.message.content.as_str())
    }
}

#[derive(Debug, Clone)]
pub enum ChatStreamEvent {
    Chunk {
        content: String,
        finish_reason: Option<String>,
    },
    Done {
        id: String,
        model: String,
        usage: Option<TokenUsage>,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ProviderInstanceConfig {
    pub base_url: String,
    pub api_key_vault_key: String,
    pub default_model: String,
    pub default_max_tokens: u32,
    pub default_temperature: f64,
    pub timeout: Duration,
    pub max_retries: u32,
    pub proxy_url: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub allow_loopback: bool,
    pub max_response_bytes: u64,
    pub max_request_bytes: u64,
}

impl Default for ProviderInstanceConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key_vault_key: String::new(),
            default_model: "gpt-4o".into(),
            default_max_tokens: 4096,
            default_temperature: 0.7,
            timeout: Duration::from_secs(120),
            max_retries: 3,
            proxy_url: None,
            allowed_hosts: Vec::new(),
            allow_loopback: false,
            max_response_bytes: 10 * 1024 * 1024,
            max_request_bytes: 10 * 1024 * 1024,
        }
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelProviderError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Authentication failed: {0}")]
    Authentication(String),
    #[error("Rate limited (retry_after: {retry_after:?})")]
    RateLimited { retry_after: Option<f64> },
    #[error("Request timed out after {0}")]
    Timeout(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Server error: {status} - {body}")]
    ServerError { status: u16, body: String },
    #[error("Invalid response from API: {0}")]
    InvalidResponse(String),
    #[error("Request too large: {0}")]
    RequestTooLarge(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Operation was cancelled")]
    Cancelled,
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ModelProviderError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_)
                | Self::Connection(_)
                | Self::ServerError {
                    status: 500..=599,
                    ..
                }
                | Self::RateLimited { .. }
        )
    }
    pub fn retry_after_secs(&self) -> Option<f64> {
        if let Self::RateLimited { retry_after } = self {
            *retry_after
        } else {
            None
        }
    }
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::Config(_)
                | Self::Authentication(_)
                | Self::RequestTooLarge(_)
                | Self::PermissionDenied(_)
        )
    }
}

#[allow(clippy::if_same_then_else)]
impl From<reqwest::Error> for ModelProviderError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout(e.to_string())
        } else if e.is_connect() {
            Self::Connection(e.to_string())
        } else if e.is_body() || e.is_decode() {
            Self::Connection(e.to_string())
        } else {
            Self::Connection(e.to_string())
        }
    }
}
