use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Permission policy that controls what an agent/workflow is allowed to do.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionPolicy {
    /// Allowed HTTP hostnames (wildcard * supported).
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    /// Allowed network CIDR ranges.
    #[serde(default)]
    pub allowed_networks: Vec<String>,
    /// Whether file:// access is allowed.
    #[serde(default)]
    pub allow_file_access: bool,
    /// Whether loopback/localhost access is allowed.
    #[serde(default)]
    pub allow_loopback: bool,
    /// Maximum number of nodes in a workflow.
    #[serde(default = "default_max_nodes")]
    pub max_nodes: i32,
    /// Maximum number of loop iterations.
    #[serde(default = "default_max_loops")]
    pub max_loops: i32,
    /// Maximum HTTP request body size in bytes.
    #[serde(default = "default_max_size")]
    pub max_request_size: i64,
    /// Maximum HTTP response body size in bytes.
    #[serde(default = "default_max_size")]
    pub max_response_size: i64,
    /// Maximum workflow execution time in seconds.
    #[serde(default = "default_max_execution_seconds")]
    pub max_execution_seconds: i64,
}

const fn default_max_nodes() -> i32 {
    50
}
const fn default_max_loops() -> i32 {
    10
}
const fn default_max_size() -> i64 {
    10 * 1024 * 1024
} // 10 MB
const fn default_max_execution_seconds() -> i64 {
    600
} // 10 minutes

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            allowed_networks: Vec::new(),
            allow_file_access: false,
            allow_loopback: false,
            max_nodes: default_max_nodes(),
            max_loops: default_max_loops(),
            max_request_size: default_max_size(),
            max_response_size: default_max_size(),
            max_execution_seconds: default_max_execution_seconds(),
        }
    }
}

/// An AI Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub permissions: PermissionPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating or updating an Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInput {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub permissions: Option<PermissionPolicy>,
}
