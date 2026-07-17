use crate::AppState;
use localflow_core::error::{CoreError, CoreResult};
use localflow_model_providers::openai::OpenAIProvider;
use localflow_model_providers::r#trait::ModelProvider;
use localflow_model_providers::types::{ChatMessage, ChatRequest};
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[tauri::command]
pub async fn chat_with_agent(
    state: State<'_, AppState>,
    agent_id: String,
    message: String,
) -> CoreResult<ChatResponse> {
    let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
        CoreError::validation(format!("Invalid agent ID: {e}"))
    })?;

    let agent = state.storage.agents.get(agent_uuid)?;
    let provider_id = "provider/openai";
    let api_key = state.vault.get(provider_id).map_err(|_| {
        CoreError::validation("API Key not configured. Go to API Management to set it up.")
    })?;

    // Ensure key is in vault for provider lookup
    state.vault.store(provider_id, &api_key).ok();

    let provider_config = localflow_model_providers::types::ProviderInstanceConfig {
        base_url: "https://api.openai.com/v1".to_string(),
        api_key_vault_key: provider_id.to_string(),
        default_model: agent.model.clone().unwrap_or_else(|| "gpt-4o".into()),
        default_max_tokens: agent.max_tokens.unwrap_or(4096) as u32,
        default_temperature: agent.temperature.unwrap_or(0.7),
        timeout: std::time::Duration::from_secs(120),
        max_retries: 3,
        proxy_url: None,
        allowed_hosts: vec![],
        allow_loopback: false,
        max_response_bytes: 10 * 1024 * 1024,
        max_request_bytes: 2 * 1024 * 1024,
    };

    let vault_for_provider = state.vault.clone();
    let provider = OpenAIProvider::new(provider_config, vault_for_provider).map_err(|e| {
        CoreError::internal(format!("Failed to create model provider: {e}"))
    })?;

    let mut messages = Vec::new();
    if let Some(system_prompt) = &agent.system_prompt {
        if !system_prompt.is_empty() {
            messages.push(ChatMessage::system(system_prompt.clone()));
        }
    }
    messages.push(ChatMessage::user(message));

    let request = ChatRequest {
        model: agent.model.clone().unwrap_or_else(|| "gpt-4o".into()),
        messages,
        temperature: agent.temperature,
        max_tokens: agent.max_tokens.map(|t| t as u32),
        top_p: None,
        stop: None,
        stream: Some(false),
    };

    let response = provider.chat(request, None).await.map_err(|e| {
        CoreError::internal(format!("Model call failed: {e}"))
    })?;

    let content = response.text().unwrap_or_default().to_string();

    Ok(ChatResponse {
        content,
        model: response.model,
        usage: response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
        }),
    })
}
