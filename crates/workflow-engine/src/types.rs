use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

use localflow_core::models::{NodeRun, Workflow, WorkflowNode, WorkflowRun};

/// Maximum number of nodes allowed in a single workflow.
pub const DEFAULT_MAX_NODES: usize = 100;

/// Maximum workflow execution time in seconds.
pub const DEFAULT_WORKFLOW_TIMEOUT_SECS: u64 = 600;

/// Default maximum retry attempts per node.
pub const DEFAULT_MAX_RETRY: i32 = 3;

/// Default HTTP request timeout in seconds.
pub const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 10;

/// Default model call timeout in seconds.
pub const DEFAULT_MODEL_TIMEOUT_SECS: u64 = 120;

/// Execution context passed to each node executor.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// The full workflow definition.
    pub workflow: Workflow,
    /// The current workflow run.
    pub workflow_run: WorkflowRun,
    /// Mapping of node_id -> NodeRun for all completed/in-progress nodes.
    pub node_runs: HashMap<Uuid, NodeRun>,
    /// Node-specific configuration (parsed from WorkflowNode.config).
    pub node_config: Value,
    /// Input values from upstream nodes (source_node_id -> output Value).
    pub upstream_outputs: HashMap<Uuid, Value>,
    /// Global variables set during execution.
    pub variables: HashMap<String, Value>,
    /// The current node being executed.
    pub current_node: WorkflowNode,
}

/// Output from a single node execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeOutput {
    /// The output data produced by this node.
    pub data: Value,
    /// Whether this data comes from an external/untrusted source.
    pub untrusted: bool,
    /// Optional error message.
    pub error: Option<String>,
    /// Optional branch taken (for Condition nodes).
    pub branch: Option<String>,
}

impl NodeOutput {
    /// Create a trusted output.
    pub fn trusted(data: Value) -> Self {
        Self {
            data,
            untrusted: false,
            error: None,
            branch: None,
        }
    }

    /// Create an untrusted output (from external API).
    pub fn untrusted(data: Value) -> Self {
        Self {
            data,
            untrusted: true,
            error: None,
            branch: None,
        }
    }
}

/// Schema definition for a node's input and output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSchema {
    /// JSON Schema for the input.
    pub input_schema: Value,
    /// JSON Schema for the output.
    pub output_schema: Value,
    /// Human-readable description of expected inputs.
    pub input_description: String,
    /// Human-readable description of produced outputs.
    pub output_description: String,
}

/// Configuration for the Start node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartNodeConfig {
    /// Initial variables to set.
    #[serde(default)]
    pub variables: HashMap<String, Value>,
}

/// Configuration for the Input node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputNodeConfig {
    /// The input prompt or instructions.
    pub prompt: String,
    /// Expected input schema (JSON Schema).
    #[serde(default)]
    pub schema: Option<Value>,
}

/// Configuration for the Model node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelNodeConfig {
    /// Provider key in secret vault (e.g. "provider/openai").
    pub provider_key: String,
    /// Model name.
    #[serde(default = "default_model")]
    pub model: String,
    /// System prompt override.
    pub system_prompt: Option<String>,
    /// Temperature.
    pub temperature: Option<f64>,
    /// Max tokens.
    pub max_tokens: Option<u32>,
    /// Base URL override.
    pub base_url: Option<String>,
}

fn default_model() -> String {
    "gpt-4o".into()
}

/// Configuration for the HTTP Request node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestNodeConfig {
    /// HTTP method.
    #[serde(default = "default_method")]
    pub method: String,
    /// URL. Can be a template with {variable} placeholders.
    pub url: String,
    /// Headers as key-value pairs (values can be templates).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body template.
    pub body: Option<String>,
    /// Content type.
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// Timeout in seconds.
    pub timeout_secs: Option<u64>,
}

fn default_method() -> String {
    "GET".into()
}
fn default_content_type() -> String {
    "application/json".into()
}

/// Configuration for the Condition node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionNodeConfig {
    /// JavaScript-like condition expression.
    /// Evaluated against upstream outputs and variables.
    /// Can reference: {{variables.xxx}}, {{outputs.node_id}}
    pub expression: String,
    /// Label for the true branch.
    #[serde(default = "default_true_label")]
    pub true_label: String,
    /// Label for the false branch.
    #[serde(default = "default_false_label")]
    pub false_label: String,
}

fn default_true_label() -> String {
    "True".into()
}
fn default_false_label() -> String {
    "False".into()
}

/// Configuration for the Template node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateNodeConfig {
    /// Template string with {{variable}} placeholders.
    pub template: String,
    /// Output variable name to store the result.
    pub output_variable: String,
}

/// Configuration for the End node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndNodeConfig {
    /// Variables to include in the final output.
    #[serde(default)]
    pub output_variables: Vec<String>,
    /// Whether to include all variables.
    #[serde(default)]
    pub include_all: bool,
}

/// Wrapper for untrusted data from external APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UntrustedData<T> {
    pub data: T,
    pub source: String,
}

/// Errors that can occur during workflow execution.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowExecutionError {
    #[error("Workflow validation failed: {0}")]
    Validation(String),

    #[error("DAG validation failed: {0}")]
    DagValidation(String),

    #[error("Node execution failed: node='{node_name}' type={node_type:?} error={error}")]
    NodeExecution {
        node_name: String,
        node_type: localflow_core::types::NodeType,
        error: String,
    },

    #[error("Workflow timed out after {0} seconds")]
    Timeout(u64),

    #[error("Workflow was cancelled")]
    Cancelled,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Template render error: {0}")]
    TemplateError(String),
}

impl WorkflowExecutionError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_) | Self::Internal(_) | Self::Storage(_) | Self::NodeExecution { .. }
        )
    }
}

impl From<localflow_core::error::CoreError> for WorkflowExecutionError {
    fn from(e: localflow_core::error::CoreError) -> Self {
        Self::Storage(e.to_string())
    }
}

/// Result type for workflow execution.
pub type ExecutionResult<T> = Result<T, WorkflowExecutionError>;
