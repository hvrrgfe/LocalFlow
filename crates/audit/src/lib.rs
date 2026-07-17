use localflow_core::error::CoreResult;
use localflow_core::models::AuditLog;
use localflow_core::types::AuditEventType;
use serde_json::Value;
use uuid::Uuid;

/// Service for creating audit log entries.
/// This is a lightweight wrapper that can be used with any storage backend
/// that implements the AuditStorage trait.
pub trait AuditStorage: Send + Sync {
    fn write_audit(
        &self,
        event_type: AuditEventType,
        entity_type: &str,
        entity_id: Option<Uuid>,
        user: Option<&str>,
        details: Option<Value>,
    ) -> CoreResult<AuditLog>;

    fn query_audit(
        &self,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        limit: usize,
    ) -> CoreResult<Vec<AuditLog>>;
}

/// Audit service that enriches audit events with context before storage.
pub struct AuditService {
    storage: Box<dyn AuditStorage>,
}

impl AuditService {
    pub fn new(storage: Box<dyn AuditStorage>) -> Self {
        Self { storage }
    }

    /// Record an agent creation event.
    pub fn agent_created(
        &self,
        agent_id: Uuid,
        user: Option<&str>,
        details: Option<Value>,
    ) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::AgentCreated,
            "agent",
            Some(agent_id),
            user,
            details,
        )
    }

    /// Record an agent update event.
    pub fn agent_updated(
        &self,
        agent_id: Uuid,
        user: Option<&str>,
        details: Option<Value>,
    ) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::AgentUpdated,
            "agent",
            Some(agent_id),
            user,
            details,
        )
    }

    /// Record an agent deletion event.
    pub fn agent_deleted(&self, agent_id: Uuid, user: Option<&str>) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::AgentDeleted,
            "agent",
            Some(agent_id),
            user,
            None,
        )
    }

    /// Record a workflow run start event.
    pub fn workflow_run_started(
        &self,
        workflow_id: Uuid,
        run_id: Uuid,
        details: Option<Value>,
    ) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::WorkflowRunStarted,
            "workflow_run",
            Some(run_id),
            None,
            Some(details.unwrap_or(serde_json::json!({"workflow_id": workflow_id.to_string()}))),
        )
    }

    /// Record a security violation event.
    pub fn security_violation(&self, details: Value) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::SecurityViolation,
            "security",
            None,
            None,
            Some(details),
        )
    }

    /// Record a permission denied event.
    pub fn permission_denied(&self, details: Value) -> CoreResult<AuditLog> {
        self.storage.write_audit(
            AuditEventType::PermissionDenied,
            "security",
            None,
            None,
            Some(details),
        )
    }

    /// Query recent audit logs.
    pub fn query(
        &self,
        entity_type: Option<&str>,
        entity_id: Option<Uuid>,
        limit: usize,
    ) -> CoreResult<Vec<AuditLog>> {
        self.storage.query_audit(entity_type, entity_id, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockAuditStorage {
        logs: Mutex<Vec<AuditLog>>,
    }

    impl MockAuditStorage {
        fn new() -> Self {
            Self {
                logs: Mutex::new(Vec::new()),
            }
        }
    }

    impl AuditStorage for MockAuditStorage {
        fn write_audit(
            &self,
            event_type: AuditEventType,
            entity_type: &str,
            entity_id: Option<Uuid>,
            user: Option<&str>,
            details: Option<Value>,
        ) -> CoreResult<AuditLog> {
            let log = AuditLog {
                id: Uuid::new_v4(),
                event_type,
                entity_type: entity_type.to_string(),
                entity_id,
                user: user.map(String::from),
                details,
                created_at: chrono::Utc::now(),
            };
            self.logs.lock().unwrap().push(log.clone());
            Ok(log)
        }

        fn query_audit(
            &self,
            _entity_type: Option<&str>,
            _entity_id: Option<Uuid>,
            _limit: usize,
        ) -> CoreResult<Vec<AuditLog>> {
            Ok(self.logs.lock().unwrap().clone())
        }
    }

    #[test]
    fn test_audit_service_agent_created() {
        let storage = MockAuditStorage::new();
        let service = AuditService::new(Box::new(storage));
        let agent_id = Uuid::new_v4();

        let log = service
            .agent_created(agent_id, Some("admin"), None)
            .unwrap();
        assert_eq!(log.event_type, AuditEventType::AgentCreated);
        assert_eq!(log.entity_id, Some(agent_id));
    }

    #[test]
    fn test_audit_service_security_violation() {
        let storage = MockAuditStorage::new();
        let service = AuditService::new(Box::new(storage));

        let log = service
            .security_violation(serde_json::json!({
                "reason": "SSRF attempt blocked",
                "url": "http://169.254.169.254/latest/meta-data/"
            }))
            .unwrap();

        assert_eq!(log.event_type, AuditEventType::SecurityViolation);
    }

    #[test]
    fn test_audit_service_query() {
        let storage = MockAuditStorage::new();
        let service = AuditService::new(Box::new(storage));

        service.agent_created(Uuid::new_v4(), None, None).unwrap();
        service.agent_created(Uuid::new_v4(), None, None).unwrap();

        let logs = service.query(Some("agent"), None, 10).unwrap();
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn test_audit_service_empty_query() {
        let storage = MockAuditStorage::new();
        let service = AuditService::new(Box::new(storage));

        let logs = service.query(None, None, 10).unwrap();
        assert_eq!(logs.len(), 0);
    }
}
