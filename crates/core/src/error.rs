use serde::Serialize;
use thiserror::Error;

/// Unified error type for LocalFlow core operations.
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("State machine error: {0}")]
    StateMachine(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("CoreError", 2)?;
        match self {
            CoreError::Validation(msg) => {
                state.serialize_field("code", "validation")?;
                state.serialize_field("message", msg)?;
            }
            CoreError::NotFound { entity_type, id } => {
                state.serialize_field("code", "not_found")?;
                state.serialize_field("message", &format!("{} not found: {}", entity_type, id))?;
            }
            CoreError::Conflict(msg) => {
                state.serialize_field("code", "conflict")?;
                state.serialize_field("message", msg)?;
            }
            CoreError::InvalidWorkflow(msg) => {
                state.serialize_field("code", "invalid_workflow")?;
                state.serialize_field("message", msg)?;
            }
            CoreError::StateMachine(msg) => {
                state.serialize_field("code", "state_machine")?;
                state.serialize_field("message", msg)?;
            }
            CoreError::Serialization(e) => {
                state.serialize_field("code", "serialization")?;
                state.serialize_field("message", &e.to_string())?;
            }
            CoreError::Internal(msg) => {
                state.serialize_field("code", "internal")?;
                state.serialize_field("message", msg)?;
            }
        }
        state.end()
    }
}

/// Unified result type for LocalFlow core.
pub type CoreResult<T> = Result<T, CoreError>;

impl CoreError {
    /// Create a validation error.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a not-found error.
    pub fn not_found(entity_type: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity_type,
            id: id.into(),
        }
    }

    /// Create a conflict error.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    /// Create an invalid workflow error.
    pub fn invalid_workflow(msg: impl Into<String>) -> Self {
        Self::InvalidWorkflow(msg.into())
    }

    /// Create a state machine error.
    pub fn state_machine(msg: impl Into<String>) -> Self {
        Self::StateMachine(msg.into())
    }

    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Returns true if the error represents a retryable condition.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}
