use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::types::AuditEventType;

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub event_type: AuditEventType,
    pub entity_type: String,
    pub entity_id: Option<Uuid>,
    pub user: Option<String>,
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}
