use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use localflow_core::types::NodeType;
use localflow_model_providers::ModelProvider;
use localflow_secret_vault::SecretVault;
use localflow_security::validate_url;

use crate::types::*;

/// Abstract trait for node execution.
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute this node with the given context.
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput>;

    /// Get the input/output schema for this node type.
    fn schema(&self) -> NodeSchema;

    /// Validate the node configuration.
    fn validate_config(&self, config: &Value) -> ExecutionResult<()>;
}

// ── Start Node ────────────────────────────────────────────────────

pub struct StartNodeExecutor;

#[async_trait]
impl NodeExecutor for StartNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: StartNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Start config: {e}")))?;

        let data = serde_json::json!({
            "variables": config.variables,
            "started_at": ctx.workflow_run.started_at,
        });

        Ok(NodeOutput::trusted(data))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "variables": {"type": "object"},
                    "started_at": {"type": "string"}
                }
            }),
            input_description: "No input required for Start node".into(),
            output_description: "Initial variables and start timestamp".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        if config.is_null() || config.is_object() {
            Ok(())
        } else {
            Err(WorkflowExecutionError::Config(
                "Start node config must be a JSON object".into(),
            ))
        }
    }
}

// ── Input Node ────────────────────────────────────────────────────

pub struct InputNodeExecutor;

#[async_trait]
impl NodeExecutor for InputNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: InputNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Input config: {e}")))?;

        // Resolve prompt template with upstream outputs
        let prompt = render_template(&config.prompt, &ctx.upstream_outputs, &ctx.variables)?;

        let data = serde_json::json!({
            "prompt": prompt,
        });

        Ok(NodeOutput::trusted(data))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string", "description": "Input prompt template"}
                },
                "required": ["prompt"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string"}
                }
            }),
            input_description: "A prompt template that may reference upstream outputs".into(),
            output_description: "The rendered prompt string".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        let _: InputNodeConfig = serde_json::from_value(config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Input config: {e}")))?;
        Ok(())
    }
}

// ── Model Node ────────────────────────────────────────────────────

pub struct ModelNodeExecutor {
    vault: Arc<dyn SecretVault>,
}

impl ModelNodeExecutor {
    pub fn new(vault: Arc<dyn SecretVault>) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl NodeExecutor for ModelNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: ModelNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Model config: {e}")))?;

        // Build messages from upstream outputs
        let messages = build_chat_messages(ctx, &config)?;

        // Build provider config
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".into());
        let provider_config = localflow_model_providers::ProviderInstanceConfig {
            base_url,
            api_key_vault_key: config.provider_key.clone(),
            default_model: config.model.clone(),
            default_max_tokens: config.max_tokens.unwrap_or(4096),
            default_temperature: config.temperature.unwrap_or(0.7),
            timeout: std::time::Duration::from_secs(
                ctx.node_config
                    .get("timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(DEFAULT_MODEL_TIMEOUT_SECS),
            ),
            max_retries: 3,
            proxy_url: None,
            allowed_hosts: Vec::new(),
            allow_loopback: false,
            max_response_bytes: 10 * 1024 * 1024,
            max_request_bytes: 10 * 1024 * 1024,
        };

        let provider =
            localflow_model_providers::OpenAIProvider::new(provider_config, self.vault.clone())
                .map_err(|e| {
                    WorkflowExecutionError::Config(format!("Failed to create model provider: {e}"))
                })?;

        let request = localflow_model_providers::ChatRequest {
            model: config.model,
            messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            top_p: None,
            stop: None,
            stream: None,
        };

        let response = provider.chat(request, None).await.map_err(|e| {
            WorkflowExecutionError::NodeExecution {
                node_name: ctx.current_node.name.clone(),
                node_type: ctx.current_node.node_type,
                error: e.to_string(),
            }
        })?;

        let text = response.text().unwrap_or("").to_string();

        // Mark model output as untrusted (external API)
        let data = serde_json::json!({
            "text": text,
            "model": response.model,
            "usage": response.usage,
        });

        Ok(NodeOutput::untrusted(data))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "provider_key": {"type": "string"},
                    "model": {"type": "string"},
                    "system_prompt": {"type": "string"},
                    "temperature": {"type": "number"},
                    "max_tokens": {"type": "integer"}
                },
                "required": ["provider_key"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "model": {"type": "string"},
                    "usage": {"type": "object"}
                }
            }),
            input_description: "Model provider key and chat parameters".into(),
            output_description: "Model response text and usage info (untrusted)".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        let _: ModelNodeConfig = serde_json::from_value(config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Model config: {e}")))?;
        Ok(())
    }
}

// ── HTTP Request Node ─────────────────────────────────────────────

pub struct HttpRequestNodeExecutor;

#[async_trait]
impl NodeExecutor for HttpRequestNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: HttpRequestNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| {
                WorkflowExecutionError::Config(format!("Invalid HTTP Request config: {e}"))
            })?;

        // Resolve URL template
        let url_str = render_template(&config.url, &ctx.upstream_outputs, &ctx.variables)?;

        // SSRF protection
        validate_url(&url_str, &[], false).map_err(|e| {
            WorkflowExecutionError::Validation(format!("URL validation failed: {e}"))
        })?;

        // Build headers (resolve templates)
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &config.headers {
            let resolved = render_template(value, &ctx.upstream_outputs, &ctx.variables)?;
            if let Ok(hv) = reqwest::header::HeaderValue::from_str(&resolved) {
                headers.insert(
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
                        WorkflowExecutionError::Config(format!("Invalid header name '{key}': {e}"))
                    })?,
                    hv,
                );
            }
        }

        // Set content type
        if let Ok(ct) = reqwest::header::HeaderValue::from_str(&config.content_type) {
            headers.insert(reqwest::header::CONTENT_TYPE, ct);
        }

        let timeout = std::time::Duration::from_secs(
            config.timeout_secs.unwrap_or(DEFAULT_HTTP_TIMEOUT_SECS),
        );

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent("LocalFlow/0.1.0")
            .build()
            .map_err(|e| {
                WorkflowExecutionError::Internal(format!("Failed to build HTTP client: {e}"))
            })?;

        let method = config.method.to_uppercase();
        let req = match method.as_str() {
            "GET" => client.get(&url_str),
            "POST" => {
                let body = if let Some(body_tpl) = &config.body {
                    render_template(body_tpl, &ctx.upstream_outputs, &ctx.variables)?
                } else {
                    String::new()
                };
                client.post(&url_str).body(body)
            }
            "PUT" => {
                let body = if let Some(body_tpl) = &config.body {
                    render_template(body_tpl, &ctx.upstream_outputs, &ctx.variables)?
                } else {
                    String::new()
                };
                client.put(&url_str).body(body)
            }
            "PATCH" => {
                let body = config
                    .body
                    .as_ref()
                    .map(|b| render_template(b, &ctx.upstream_outputs, &ctx.variables))
                    .transpose()?
                    .unwrap_or_default();
                client.patch(&url_str).body(body)
            }
            "DELETE" => client.delete(&url_str),
            other => {
                return Err(WorkflowExecutionError::Config(format!(
                    "Unsupported HTTP method: {other}"
                )));
            }
        };

        let response = req.headers(headers).send().await.map_err(|e| {
            if e.is_timeout() {
                WorkflowExecutionError::Timeout(
                    config.timeout_secs.unwrap_or(DEFAULT_HTTP_TIMEOUT_SECS),
                )
            } else if e.is_connect() {
                WorkflowExecutionError::NodeExecution {
                    node_name: ctx.current_node.name.clone(),
                    node_type: ctx.current_node.node_type,
                    error: format!("Connection failed: {e}"),
                }
            } else {
                WorkflowExecutionError::NodeExecution {
                    node_name: ctx.current_node.name.clone(),
                    node_type: ctx.current_node.node_type,
                    error: e.to_string(),
                }
            }
        })?;

        let status = response.status().as_u16();
        let resp_headers: std::collections::HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body_bytes =
            response
                .bytes()
                .await
                .map_err(|e| WorkflowExecutionError::NodeExecution {
                    node_name: ctx.current_node.name.clone(),
                    node_type: ctx.current_node.node_type,
                    error: format!("Failed to read response body: {e}"),
                })?;

        // Parse response body
        let body_value: Value = serde_json::from_slice(&body_bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body_bytes).to_string()));

        let output = serde_json::json!({
            "status": status,
            "headers": resp_headers,
            "body": body_value,
        });

        // Mark as untrusted since it came from an external API
        Ok(NodeOutput::untrusted(output))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"]},
                    "headers": {"type": "object"},
                    "body": {"type": "string"}
                },
                "required": ["url"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status": {"type": "integer"},
                    "body": {"type": "object"},
                    "headers": {"type": "object"}
                }
            }),
            input_description: "HTTP request parameters with template support".into(),
            output_description: "HTTP response (status, headers, body) - untrusted data".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        let cfg: HttpRequestNodeConfig = serde_json::from_value(config.clone()).map_err(|e| {
            WorkflowExecutionError::Config(format!("Invalid HTTP Request config: {e}"))
        })?;

        // Validate URL format (but don't resolve templates yet)
        if !cfg.url.contains("{{") {
            validate_url(&cfg.url, &[], false).map_err(|e| {
                WorkflowExecutionError::Config(format!("Invalid URL in config: {e}"))
            })?;
        }

        // Validate method
        match cfg.method.to_uppercase().as_str() {
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" => Ok(()),
            other => Err(WorkflowExecutionError::Config(format!(
                "Unsupported HTTP method: {other}"
            ))),
        }
    }
}

// ── Condition Node ────────────────────────────────────────────────

pub struct ConditionNodeExecutor;

#[async_trait]
impl NodeExecutor for ConditionNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: ConditionNodeConfig =
            serde_json::from_value(ctx.node_config.clone()).map_err(|e| {
                WorkflowExecutionError::Config(format!("Invalid Condition config: {e}"))
            })?;

        // Evaluate condition expression against upstream outputs and variables
        let result = evaluate_condition(&config.expression, &ctx.upstream_outputs, &ctx.variables)?;

        let branch = if result {
            config.true_label
        } else {
            config.false_label
        };

        let data = serde_json::json!({
            "result": result,
            "branch": branch,
        });

        Ok(NodeOutput {
            data,
            untrusted: false,
            error: None,
            branch: Some(branch),
        })
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {"type": "string", "description": "Condition expression"},
                    "true_label": {"type": "string"},
                    "false_label": {"type": "string"}
                },
                "required": ["expression"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "result": {"type": "boolean"},
                    "branch": {"type": "string"}
                }
            }),
            input_description: "A condition expression that evaluates to true/false".into(),
            output_description: "Boolean result and the branch label taken".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        let _: ConditionNodeConfig = serde_json::from_value(config.clone()).map_err(|e| {
            WorkflowExecutionError::Config(format!("Invalid Condition config: {e}"))
        })?;
        Ok(())
    }
}

// ── Template Node ─────────────────────────────────────────────────

pub struct TemplateNodeExecutor;

#[async_trait]
impl NodeExecutor for TemplateNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: TemplateNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Template config: {e}")))?;

        let rendered = render_template(&config.template, &ctx.upstream_outputs, &ctx.variables)?;

        let mut variables = ctx.variables.clone();
        variables.insert(
            config.output_variable.clone(),
            Value::String(rendered.clone()),
        );

        let data = serde_json::json!({
            "rendered": rendered,
            "output_variable": config.output_variable,
        });

        Ok(NodeOutput::trusted(data))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "template": {"type": "string", "description": "Template with {{variable}} placeholders"},
                    "output_variable": {"type": "string"}
                },
                "required": ["template", "output_variable"]
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "rendered": {"type": "string"},
                    "output_variable": {"type": "string"}
                }
            }),
            input_description: "Template string and output variable name".into(),
            output_description: "Rendered template result".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        let _: TemplateNodeConfig = serde_json::from_value(config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid Template config: {e}")))?;
        Ok(())
    }
}

// ── End Node ──────────────────────────────────────────────────────

pub struct EndNodeExecutor;

#[async_trait]
impl NodeExecutor for EndNodeExecutor {
    async fn execute(&self, ctx: &ExecutionContext) -> ExecutionResult<NodeOutput> {
        let config: EndNodeConfig = serde_json::from_value(ctx.node_config.clone())
            .map_err(|e| WorkflowExecutionError::Config(format!("Invalid End config: {e}")))?;

        let mut output = HashMap::new();

        if config.include_all {
            for (key, value) in &ctx.variables {
                output.insert(key.clone(), value.clone());
            }
        } else {
            for var_name in &config.output_variables {
                if let Some(value) = ctx.variables.get(var_name) {
                    output.insert(var_name.clone(), value.clone());
                }
            }
        }

        // Also include final upstream outputs
        for (node_id, output_val) in &ctx.upstream_outputs {
            output.insert(format!("node_{}", node_id), output_val.clone());
        }

        Ok(NodeOutput::trusted(serde_json::json!(output)))
    }

    fn schema(&self) -> NodeSchema {
        NodeSchema {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "output_variables": {"type": "array", "items": {"type": "string"}},
                    "include_all": {"type": "boolean"}
                }
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "description": "Final workflow output"
            }),
            input_description: "Which variables to include in the final output".into(),
            output_description: "Final workflow result containing selected variables".into(),
        }
    }

    fn validate_config(&self, config: &Value) -> ExecutionResult<()> {
        if config.is_null() || config.is_object() {
            Ok(())
        } else {
            Err(WorkflowExecutionError::Config(
                "End node config must be a JSON object".into(),
            ))
        }
    }
}

// ── Factory ───────────────────────────────────────────────────────

/// Create the appropriate executor for a node type.
pub fn create_executor(
    node_type: NodeType,
    vault: Option<Arc<dyn SecretVault>>,
) -> Box<dyn NodeExecutor> {
    match node_type {
        NodeType::Start => Box::new(StartNodeExecutor),
        NodeType::Input => Box::new(InputNodeExecutor),
        NodeType::Model => Box::new(ModelNodeExecutor::new(
            vault.expect("Model node requires a SecretVault"),
        )),
        NodeType::HttpRequest => Box::new(HttpRequestNodeExecutor),
        NodeType::Condition => Box::new(ConditionNodeExecutor),
        NodeType::Template => Box::new(TemplateNodeExecutor),
        NodeType::End => Box::new(EndNodeExecutor),
    }
}

// ── Template rendering ────────────────────────────────────────────

/// Render a template string by replacing {{variable}} and {{outputs.node_id.path}} placeholders.
fn render_template(
    template: &str,
    upstream_outputs: &HashMap<uuid::Uuid, Value>,
    variables: &HashMap<String, Value>,
) -> Result<String, WorkflowExecutionError> {
    let re = regex::Regex::new(r"\{\{(.+?)\}\}")
        .map_err(|e| WorkflowExecutionError::Internal(format!("Regex error: {e}")))?;

    let result = re.replace_all(template, |caps: &regex::Captures| {
        let path = caps[1].trim();
        resolve_value(path, upstream_outputs, variables)
            .map(|v| match v {
                Value::String(s) => s,
                other => serde_json::to_string(&other).unwrap_or_default(),
            })
            .unwrap_or_else(|e| format!("{{{{ERROR:{e}}}}}"))
    });

    Ok(result.to_string())
}

/// Resolve a dotted path like "variables.foo" or "outputs.node_id.field" to a JSON value.
fn resolve_value(
    path: &str,
    upstream_outputs: &HashMap<uuid::Uuid, Value>,
    variables: &HashMap<String, Value>,
) -> Result<Value, WorkflowExecutionError> {
    let parts: Vec<&str> = path.splitn(2, '.').collect();
    if parts.is_empty() {
        return Err(WorkflowExecutionError::TemplateError("Empty path".into()));
    }

    match parts[0] {
        "variables" => {
            if parts.len() < 2 {
                return Err(WorkflowExecutionError::TemplateError(
                    "Missing variable name".into(),
                ));
            }
            variables.get(parts[1]).cloned().ok_or_else(|| {
                WorkflowExecutionError::TemplateError(format!("Variable '{}' not found", parts[1]))
            })
        }
        "outputs" => {
            if parts.len() < 2 {
                return Err(WorkflowExecutionError::TemplateError(
                    "Missing output reference".into(),
                ));
            }
            let sub_parts: Vec<&str> = parts[1].splitn(2, '.').collect();
            if sub_parts.is_empty() {
                return Err(WorkflowExecutionError::TemplateError(
                    "Missing node ID".into(),
                ));
            }
            let node_id = uuid::Uuid::parse_str(sub_parts[0]).map_err(|e| {
                WorkflowExecutionError::TemplateError(format!(
                    "Invalid node ID '{}': {e}",
                    sub_parts[0]
                ))
            })?;
            let output = upstream_outputs.get(&node_id).ok_or_else(|| {
                WorkflowExecutionError::TemplateError(format!(
                    "Output from node '{}' not found",
                    sub_parts[0]
                ))
            })?;
            if sub_parts.len() > 1 {
                // Navigate into the output JSON
                let pointer = format!("/{}", sub_parts[1].replace('.', "/"));
                output.pointer(&pointer).cloned().ok_or_else(|| {
                    WorkflowExecutionError::TemplateError(format!(
                        "Path '{}' not found in node output",
                        sub_parts[1]
                    ))
                })
            } else {
                Ok(output.clone())
            }
        }
        "input" => {
            // Reference the current node's input
            Err(WorkflowExecutionError::TemplateError(
                "'input' references not yet supported".into(),
            ))
        }
        other => Err(WorkflowExecutionError::TemplateError(format!(
            "Unknown reference type '{other}'"
        ))),
    }
}

// ── Condition evaluation ──────────────────────────────────────────

/// Evaluate a simple condition expression.
/// Supports: ==, !=, >, <, >=, <=, &&, ||, !, true, false, number literals, string literals.
fn evaluate_condition(
    expression: &str,
    upstream_outputs: &HashMap<uuid::Uuid, Value>,
    variables: &HashMap<String, Value>,
) -> Result<bool, WorkflowExecutionError> {
    let expr = expression.trim();

    // First, resolve any {{placeholder}} references
    let resolved = render_template(expr, upstream_outputs, variables)?;
    let resolved = resolved.trim().to_string();

    // Handle simple boolean literals
    if resolved.eq_ignore_ascii_case("true") {
        return Ok(true);
    }
    if resolved.eq_ignore_ascii_case("false") {
        return Ok(false);
    }

    // Try to evaluate as a JSON value (quoted strings, numbers)
    if let Ok(val) = serde_json::from_str::<Value>(&resolved) {
        match val {
            Value::Bool(b) => return Ok(b),
            Value::Number(n) => return Ok(n.as_f64().is_some_and(|f| f != 0.0)),
            Value::String(s) => return Ok(!s.is_empty() && !s.eq_ignore_ascii_case("false")),
            _ => return Ok(true),
        }
    }

    // Simple comparison operations: left op right
    let operators = ["==", "!=", ">=", "<=", ">", "<"];
    for op in &operators {
        if let Some(pos) = resolved.find(op) {
            let left = resolved[..pos].trim();
            let right = resolved[pos + op.len()..].trim();

            // Remove surrounding quotes
            let left_val = parse_literal(left);
            let right_val = parse_literal(right);

            return compare_values(&left_val, &right_val, op);
        }
    }

    // If we get here, treat non-empty string as truthy
    Ok(!resolved.is_empty())
}

fn parse_literal(s: &str) -> Value {
    let s = s.trim();
    // Try number
    if let Ok(n) = s.parse::<f64>() {
        return Value::Number(
            serde_json::Number::from_f64(n).unwrap_or_else(|| serde_json::Number::from(0)),
        );
    }
    // Try boolean
    if s.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if s.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    // String (remove quotes)
    let stripped = s.trim_matches('"').trim_matches('\'').to_string();
    Value::String(stripped)
}

fn compare_values(left: &Value, right: &Value, op: &str) -> Result<bool, WorkflowExecutionError> {
    match op {
        "==" => Ok(left == right),
        "!=" => Ok(left != right),
        ">" | ">=" | "<" | "<=" => {
            let l = left.as_f64().ok_or_else(|| {
                WorkflowExecutionError::TemplateError(format!(
                    "Cannot compare non-numeric value: {left}"
                ))
            })?;
            let r = right.as_f64().ok_or_else(|| {
                WorkflowExecutionError::TemplateError(format!(
                    "Cannot compare non-numeric value: {right}"
                ))
            })?;
            match op {
                ">" => Ok(l > r),
                ">=" => Ok(l >= r),
                "<" => Ok(l < r),
                "<=" => Ok(l <= r),
                _ => unreachable!(),
            }
        }
        _ => Err(WorkflowExecutionError::TemplateError(format!(
            "Unknown operator: {op}"
        ))),
    }
}

// ── Chat message builder ──────────────────────────────────────────

/// Build chat messages from upstream outputs and model config.
fn build_chat_messages(
    ctx: &ExecutionContext,
    config: &ModelNodeConfig,
) -> Result<Vec<localflow_model_providers::ChatMessage>, WorkflowExecutionError> {
    let mut messages = Vec::new();

    // Add system prompt if configured
    if let Some(system_prompt) = &config.system_prompt {
        let rendered = render_template(system_prompt, &ctx.upstream_outputs, &ctx.variables)?;
        messages.push(localflow_model_providers::ChatMessage::system(rendered));
    }

    // Add upstream outputs as context
    for output in ctx.upstream_outputs.values() {
        let text = match output {
            Value::String(s) => s.clone(),
            other => serde_json::to_string_pretty(other).unwrap_or_default(),
        };
        messages.push(localflow_model_providers::ChatMessage::user(text));
    }

    Ok(messages)
}
