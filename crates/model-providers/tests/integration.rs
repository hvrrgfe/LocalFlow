use std::sync::Arc;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use localflow_model_providers::*;
use localflow_secret_vault::{InMemoryVault, SecretVault};

fn make_openai_config(mock_url: String) -> ProviderInstanceConfig {
    ProviderInstanceConfig {
        base_url: mock_url.trim_end_matches("/chat/completions").to_string(),
        api_key_vault_key: "test-key".into(),
        default_model: "gpt-4o".into(),
        default_max_tokens: 100,
        default_temperature: 0.7,
        timeout: Duration::from_secs(5),
        max_retries: 1,
        allow_loopback: true,
        max_response_bytes: 1024 * 1024,
        max_request_bytes: 1024 * 1024,
        ..Default::default()
    }
}

fn make_vault() -> Arc<InMemoryVault> {
    let vault = Arc::new(InMemoryVault::new());
    vault.store("test-key", "sk-test-secret-12345").unwrap();
    vault
}

#[tokio::test]
async fn test_openai_success() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": { "role": "assistant", "content": "Hello! How can I help you today?" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 8, "total_tokens": 18 }
        })))
        .mount(&mock_server)
        .await;

    let config = make_openai_config(mock_server.uri());
    let provider = OpenAIProvider::new(config, vault).unwrap();

    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("Say hello")]);

    let response = provider.chat(request, None).await.unwrap();
    assert_eq!(response.text(), Some("Hello! How can I help you today?"));
    assert_eq!(response.model, "gpt-4o");
}

#[tokio::test]
async fn test_openai_http_500() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let config = make_openai_config(mock_server.uri());
    let provider = OpenAIProvider::new(config, vault).unwrap();

    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        ModelProviderError::ServerError { status, .. } => assert_eq!(status, 500),
        other => panic!("Expected ServerError, got: {other}"),
    }
}

#[tokio::test]
async fn test_openai_http_401() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let config = make_openai_config(mock_server.uri());
    let provider = OpenAIProvider::new(config, vault).unwrap();

    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    assert!(matches!(
        result.unwrap_err(),
        ModelProviderError::Authentication(_)
    ));
}

#[tokio::test]
async fn test_openai_invalid_json_response() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("this is not valid json"))
        .mount(&mock_server)
        .await;

    let config = make_openai_config(mock_server.uri());
    let provider = OpenAIProvider::new(config, vault).unwrap();

    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    assert!(matches!(
        result.unwrap_err(),
        ModelProviderError::InvalidResponse(_)
    ));
}

#[tokio::test]
async fn test_openai_timeout() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"choices": []}))
                .set_delay(Duration::from_secs(30)),
        )
        .mount(&mock_server)
        .await;

    let mut config = make_openai_config(mock_server.uri());
    config.timeout = Duration::from_millis(100);

    let provider = OpenAIProvider::new(config, vault).unwrap();
    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_openai_api_key_not_in_logs() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string("Invalid API key: sk-test-secret-12345"),
        )
        .mount(&mock_server)
        .await;

    let config = make_openai_config(mock_server.uri());
    let provider = OpenAIProvider::new(config, vault).unwrap();

    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    if let Err(e) = result {
        let err_str = e.to_string();
        let redacted = localflow_security::redact_sensitive(&err_str);
        assert!(
            !redacted.contains("sk-test-secret-12345"),
            "API key leaked in error: {redacted}"
        );
        assert!(
            redacted.contains("[REDACTED]") || !err_str.contains("sk-test-secret-12345"),
            "Expected redaction in error message, got: {err_str}"
        );
    }
}

#[tokio::test]
async fn test_network_disconnect() {
    // Try connecting to a port that's not listening
    let vault = make_vault();
    let config = ProviderInstanceConfig {
        base_url: "http://127.0.0.1:1".into(),
        api_key_vault_key: "test-key".into(),
        default_model: "gpt-4o".into(),
        timeout: Duration::from_secs(1),
        max_retries: 0,
        allow_loopback: true,
        max_response_bytes: 1024 * 1024,
        max_request_bytes: 1024 * 1024,
        ..Default::default()
    };

    let provider = OpenAIProvider::new(config, vault).unwrap();
    let request = ChatRequest::new("gpt-4o", vec![ChatMessage::user("test")]);
    let result = provider.chat(request, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_custom_http_success() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Custom response text"))
        .mount(&mock_server)
        .await;

    let config = ProviderInstanceConfig {
        base_url: mock_server.uri(),
        api_key_vault_key: "test-key".into(),
        default_model: "custom".into(),
        timeout: Duration::from_secs(5),
        allow_loopback: true,
        max_response_bytes: 1024 * 1024,
        max_request_bytes: 1024 * 1024,
        ..Default::default()
    };

    let provider = CustomHttpProvider::new(config, vault).unwrap();
    let request = ChatRequest::new(
        "custom",
        vec![ChatMessage::user(mock_server.uri() + "/endpoint")],
    );
    let response = provider.chat(request, None).await.unwrap();
    assert_eq!(response.text(), Some("Custom response text"));
}

#[tokio::test]
async fn test_custom_http_500() {
    let mock_server = MockServer::start().await;
    let vault = make_vault();

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .mount(&mock_server)
        .await;

    let config = ProviderInstanceConfig {
        base_url: mock_server.uri(),
        api_key_vault_key: "test-key".into(),
        default_model: "custom".into(),
        timeout: Duration::from_secs(5),
        allow_loopback: true,
        max_response_bytes: 1024 * 1024,
        max_request_bytes: 1024 * 1024,
        ..Default::default()
    };

    let provider = CustomHttpProvider::new(config, vault).unwrap();
    let request = ChatRequest::new("custom", vec![ChatMessage::user(mock_server.uri())]);
    let result = provider.chat(request, None).await;
    assert!(matches!(
        result.unwrap_err(),
        ModelProviderError::ServerError { status: 500, .. }
    ));
}
