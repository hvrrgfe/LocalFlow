mod agent_repo;
mod audit_repo;
mod migrations;
mod run_repo;
mod workflow_repo;

pub use agent_repo::AgentRepository;
pub use audit_repo::AuditRepository;
pub use migrations::Migrations;
pub use run_repo::RunRepository;
pub use workflow_repo::{WorkflowEdgeInput, WorkflowInput, WorkflowNodeInput, WorkflowRepository};

use localflow_core::error::CoreResult;
use rusqlite::Connection;
use std::sync::Mutex;

/// The main LocalFlow storage engine.
/// Provides access to all repositories and database initialization.
pub struct StorageEngine {
    conn: std::sync::Arc<Mutex<Connection>>,
    pub agents: AgentRepository,
    pub workflows: WorkflowRepository,
    pub runs: RunRepository,
    pub audit: AuditRepository,
}

impl StorageEngine {
    /// Initialize the storage engine with a database connection.
    /// If the database file does not exist, it will be created.
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file. Use `:memory:` for in-memory database.
    pub fn new(db_path: &str) -> CoreResult<Self> {
        let conn = Connection::open(db_path).map_err(|e| {
            localflow_core::error::CoreError::internal(format!(
                "Failed to open database at '{db_path}': {e}"
            ))
        })?;

        Self::from_connection(conn)
    }

    /// Initialize the storage engine with an existing connection.
    pub fn from_connection(conn: Connection) -> CoreResult<Self> {
        // Enable WAL mode and foreign keys
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| {
            localflow_core::error::CoreError::internal(format!("Failed to configure database: {e}"))
        })?;

        // Run migrations
        Migrations::run(&conn).map_err(|e| {
            localflow_core::error::CoreError::internal(format!("Migration failed: {e}"))
        })?;

        let conn = std::sync::Arc::new(Mutex::new(conn));

        Ok(Self {
            agents: AgentRepository::new(conn.clone()),
            workflows: WorkflowRepository::new(conn.clone()),
            runs: RunRepository::new(conn.clone()),
            audit: AuditRepository::new(conn.clone()),
            conn,
        })
    }

    /// Get a raw connection for direct queries (use with caution).
    pub fn connection(&self) -> &std::sync::Arc<Mutex<Connection>> {
        &self.conn
    }

    /// Create an in-memory database for testing purposes.
    pub fn new_in_memory() -> CoreResult<Self> {
        Self::new(":memory:")
    }
}

/// Initialize an in-memory database for unit tests.
pub fn init_test_db() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;",
    )?;
    Migrations::run(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use localflow_core::models::AgentInput;
    use uuid::Uuid;

    #[test]
    fn test_storage_engine_new_in_memory() {
        let engine = StorageEngine::new_in_memory().unwrap();
        let conn = engine.conn.lock().unwrap();
        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"agents".to_string()));
        assert!(tables.contains(&"workflows".to_string()));
        assert!(tables.contains(&"workflow_nodes".to_string()));
        assert!(tables.contains(&"workflow_edges".to_string()));
        assert!(tables.contains(&"workflow_runs".to_string()));
        assert!(tables.contains(&"node_runs".to_string()));
        assert!(tables.contains(&"audit_logs".to_string()));
        assert!(tables.contains(&"secret_references".to_string()));
        assert!(tables.contains(&"provider_configs".to_string()));
        assert!(tables.contains(&"_migrations".to_string()));
    }

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(Migrations::run(&conn).is_ok());
        // Running migrations again should be idempotent
        assert!(Migrations::run(&conn).is_ok());
    }

    #[test]
    fn test_storage_engine_persistence() {
        let engine = StorageEngine::new_in_memory().unwrap();

        let input = AgentInput {
            name: "Persist Test".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: None,
        };

        let agent = engine.agents.create(input).unwrap();
        let fetched = engine.agents.get(agent.id).unwrap();
        assert_eq!(fetched.name, "Persist Test");
    }

    #[test]
    fn test_workflow_with_edges() {
        let engine = StorageEngine::new_in_memory().unwrap();
        let agent_id = Uuid::new_v4();
        // Insert an agent so FK constraint passes
        {
            let conn = engine.connection().lock().unwrap();
            conn.execute(
                "INSERT INTO agents (id, name) VALUES (?1, ?2)",
                rusqlite::params![agent_id.to_string(), "test_agent"],
            )
            .unwrap();
        }
        let input = WorkflowInput {
            agent_id,
            name: "Edged Workflow".into(),
            description: None,
            nodes: vec![
                WorkflowNodeInput {
                    node_type: localflow_core::types::NodeType::Start,
                    name: "Start".into(),
                    config: serde_json::Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: localflow_core::types::NodeType::End,
                    name: "End".into(),
                    config: serde_json::Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        let wf = engine.workflows.create(input).unwrap();
        assert_eq!(wf.nodes.len(), 2);
        assert_eq!(wf.edges.len(), 0);
    }

    #[test]
    fn test_audit_integration() {
        let engine = StorageEngine::new_in_memory().unwrap();
        let agent_id = Uuid::new_v4();

        let log = engine
            .audit
            .create(
                localflow_core::types::AuditEventType::AgentCreated,
                "agent",
                Some(agent_id),
                Some("test"),
                None,
            )
            .unwrap();

        assert_eq!(log.entity_id, Some(agent_id));

        let logs = engine
            .audit
            .list(Some("agent"), Some(agent_id), 10)
            .unwrap();
        assert_eq!(logs.len(), 1);
    }
}
