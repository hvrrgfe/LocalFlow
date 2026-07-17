use std::sync::Mutex;

use chrono::Utc;
use localflow_core::error::{CoreError, CoreResult};
use localflow_core::models::AuditLog;
use localflow_core::types::AuditEventType;
use rusqlite::{Connection, params, types::ToSql};
use serde_json::Value;
use uuid::Uuid;

/// Repository for audit log operations.
pub struct AuditRepository {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl AuditRepository {
    pub fn new(conn: std::sync::Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create an audit log entry.
    pub fn create(
        &self,
        event_type: AuditEventType,
        entity_type: &str,
        entity_id: Option<Uuid>,
        user: Option<&str>,
        details: Option<Value>,
    ) -> CoreResult<AuditLog> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let event_type_str = serde_json::to_string(&event_type)
            .map_err(|e| CoreError::internal(format!("Failed to serialize event type: {e}")))?
            .trim_matches('"')
            .to_string();

        let details_json = details
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| CoreError::internal(format!("Failed to serialize details: {e}")))?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO audit_logs (id, event_type, entity_type, entity_id, user, details, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id.to_string(),
                event_type_str,
                entity_type,
                entity_id.map(|e| e.to_string()),
                user,
                details_json,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to create audit log: {e}")))?;

        Ok(AuditLog {
            id,
            event_type,
            entity_type: entity_type.to_string(),
            entity_id,
            user: user.map(String::from),
            details,
            created_at: now,
        })
    }

    /// List audit logs, optionally filtered by entity type and id.
    pub fn list(
        &self,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        limit: usize,
    ) -> CoreResult<Vec<AuditLog>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let sql: String;
        let mut params_list: Vec<Box<dyn ToSql>> = Vec::new();

        match (entity_type, entity_id) {
            (Some(etype), Some(eid)) => {
                sql = format!(
                    "SELECT id, event_type, entity_type, entity_id, user, details, created_at
                     FROM audit_logs WHERE entity_type = ?1 AND entity_id = ?2
                     ORDER BY created_at DESC LIMIT ?3"
                );
                params_list.push(Box::new(etype.to_string()));
                params_list.push(Box::new(eid.to_string()));
                params_list.push(Box::new(limit as i64));
            }
            (Some(etype), None) => {
                sql = format!(
                    "SELECT id, event_type, entity_type, entity_id, user, details, created_at
                     FROM audit_logs WHERE entity_type = ?1
                     ORDER BY created_at DESC LIMIT ?2"
                );
                params_list.push(Box::new(etype.to_string()));
                params_list.push(Box::new(limit as i64));
            }
            (None, _) => {
                sql = format!(
                    "SELECT id, event_type, entity_type, entity_id, user, details, created_at
                     FROM audit_logs ORDER BY created_at DESC LIMIT ?1"
                );
                params_list.push(Box::new(limit as i64));
            }
        }

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| CoreError::internal(format!("Failed to prepare audit query: {e}")))?;

        let param_refs: Vec<&dyn ToSql> = params_list.iter().map(|p| p.as_ref()).collect();

        let logs = stmt
            .query_map(param_refs.as_slice(), |row| {
                let event_type_str: String = row.get(1)?;
                let event_type: AuditEventType =
                    serde_json::from_str(&format!("\"{event_type_str}\""))
                        .unwrap_or(AuditEventType::SystemError);

                let parse_json = |s: Option<String>| -> Option<Value> {
                    s.and_then(|v| serde_json::from_str(&v).ok())
                };

                Ok(AuditLog {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    event_type,
                    entity_type: row.get(2)?,
                    entity_id: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| Uuid::parse_str(&s).ok()),
                    user: row.get(4)?,
                    details: parse_json(row.get(5)?),
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query audit logs: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read audit log: {e}")))?;

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_db;

    fn setup_repo() -> (AuditRepository, std::sync::Arc<Mutex<Connection>>) {
        let conn = std::sync::Arc::new(Mutex::new(init_test_db().unwrap()));
        let repo = AuditRepository::new(conn.clone());
        (repo, conn)
    }

    #[test]
    fn test_create_audit_log() {
        let (repo, _conn) = setup_repo();
        let entity_id = Uuid::new_v4();

        let log = repo
            .create(
                AuditEventType::AgentCreated,
                "agent",
                Some(entity_id),
                Some("test_user"),
                Some(serde_json::json!({"name": "Test Agent"})),
            )
            .unwrap();

        assert_eq!(log.event_type, AuditEventType::AgentCreated);
        assert_eq!(log.entity_type, "agent");
        assert_eq!(log.entity_id, Some(entity_id));
    }

    #[test]
    fn test_list_audit_logs() {
        let (repo, _conn) = setup_repo();
        let entity_id = Uuid::new_v4();

        repo.create(
            AuditEventType::AgentCreated,
            "agent",
            Some(entity_id),
            None,
            None,
        )
        .unwrap();

        repo.create(
            AuditEventType::AgentUpdated,
            "agent",
            Some(entity_id),
            None,
            None,
        )
        .unwrap();

        let logs = repo.list(Some("agent"), Some(entity_id), 10).unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_list_audit_logs_with_limit() {
        let (repo, _conn) = setup_repo();
        let entity_id = Uuid::new_v4();

        for _ in 0..5 {
            repo.create(
                AuditEventType::AgentCreated,
                "agent",
                Some(entity_id),
                None,
                None,
            )
            .unwrap();
        }

        let logs = repo.list(Some("agent"), Some(entity_id), 3).unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[test]
    fn test_audit_log_without_entity() {
        let (repo, _conn) = setup_repo();

        let log = repo
            .create(
                AuditEventType::SystemError,
                "system",
                None,
                None,
                Some(serde_json::json!({"error": "test"})),
            )
            .unwrap();

        assert!(log.entity_id.is_none());
        assert_eq!(log.event_type, AuditEventType::SystemError);
    }
}
