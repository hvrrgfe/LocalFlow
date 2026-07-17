use serde::{Deserialize, Serialize};

/// Supported node types in a workflow DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Start,
    Input,
    Model,
    HttpRequest,
    Condition,
    Template,
    End,
}

/// Supported provider types for model and API configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    OpenaiCompatible,
    CustomHttp,
}

/// Trigger type for a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Manual,
    Scheduled,
    Webhook,
}

/// Secret types stored in the vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    ApiKey,
    Token,
    Password,
    Custom,
}

/// Category for audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    AgentCreated,
    AgentUpdated,
    AgentDeleted,
    AgentExported,
    AgentImported,
    WorkflowCreated,
    WorkflowUpdated,
    WorkflowDeleted,
    WorkflowRunStarted,
    WorkflowRunCompleted,
    WorkflowRunFailed,
    WorkflowRunCancelled,
    WorkflowRunRetried,
    ProviderConfigCreated,
    ProviderConfigUpdated,
    ProviderConfigDeleted,
    SecretStored,
    SecretDeleted,
    NodeRunStarted,
    NodeRunCompleted,
    NodeRunFailed,
    NodeRunRetried,
    PermissionDenied,
    SecurityViolation,
    SystemError,
    ConfigExported,
    ConfigImported,
}
