use std::sync::Mutex;

use chrono::Utc;
use localflow_core::error::{CoreError, CoreResult};
use localflow_core::models::{NodeRun, NodeStatus, RunStatus, WorkflowRun};
use localflow_core::types::NodeType;
use rusqlite::{Connection, params};
use serde_json::Value;
use uuid::Uuid;

/// Repository for WorkflowRun and NodeRun operations.
#[derive(Clone)]
pub struct RunRepository {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl RunRepository {
    pub fn new(conn: std::sync::Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create a new workflow run.
    pub fn create_run(&self, workflow_id: Uuid, trigger_type: &str) -> CoreResult<WorkflowRun> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO workflow_runs (id, workflow_id, status, trigger_type, created_at)
             VALUES (?1, ?2, 'pending', ?3, ?4)",
            params![
                id.to_string(),
                workflow_id.to_string(),
                trigger_type,
                now.to_rfc3339()
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to create workflow run: {e}")))?;

        Ok(WorkflowRun {
            id,
            workflow_id,
            status: RunStatus::Pending,
            started_at: None,
            completed_at: None,
            error: None,
            trigger_type: trigger_type.to_string(),
            created_at: now,
        })
    }

    /// Get a workflow run by ID.
    pub fn get_run(&self, id: Uuid) -> CoreResult<WorkflowRun> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.query_row(
            "SELECT id, workflow_id, status, started_at, completed_at, error, trigger_type, created_at
             FROM workflow_runs WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let status_str: String = row.get(2)?;
                let status: RunStatus = serde_json::from_str(&format!("\"{status_str}\"")).unwrap_or(RunStatus::Pending);

                Ok(WorkflowRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    status,
                    started_at: row.get::<_, Option<String>>(3)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                    }),
                    completed_at: row.get::<_, Option<String>>(4)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                    }),
                    error: row.get(5)?,
                    trigger_type: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                CoreError::not_found("workflow_run", id.to_string())
            }
            other => CoreError::internal(format!("Failed to get workflow run: {other}")),
        })
    }

    /// Update a workflow run's status.
    pub fn update_run_status(
        &self,
        id: Uuid,
        status: RunStatus,
        error: Option<String>,
    ) -> CoreResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let status_str = serde_json::to_string(&status)
            .map_err(|e| CoreError::internal(format!("Failed to serialize status: {e}")))?
            .trim_matches('"')
            .to_string();

        let now = Utc::now();
        let is_terminal = matches!(
            status,
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled | RunStatus::TimedOut
        );

        if is_terminal {
            conn.execute(
                "UPDATE workflow_runs SET status = ?1, error = ?2, completed_at = ?3 WHERE id = ?4",
                params![status_str, error, now.to_rfc3339(), id.to_string()],
            )
        } else if status == RunStatus::Running {
            conn.execute(
                "UPDATE workflow_runs SET status = ?1, error = ?2, started_at = ?3 WHERE id = ?4",
                params![status_str, error, now.to_rfc3339(), id.to_string()],
            )
        } else {
            conn.execute(
                "UPDATE workflow_runs SET status = ?1, error = ?2 WHERE id = ?3",
                params![status_str, error, id.to_string()],
            )
        }
        .map_err(|e| CoreError::internal(format!("Failed to update workflow run: {e}")))?;

        Ok(())
    }

    /// List workflow runs for a given workflow.
    pub fn list_runs(&self, workflow_id: Uuid) -> CoreResult<Vec<WorkflowRun>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, workflow_id, status, started_at, completed_at, error, trigger_type, created_at
                 FROM workflow_runs WHERE workflow_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare query: {e}")))?;

        let runs = stmt
            .query_map(params![workflow_id.to_string()], |row| {
                let status_str: String = row.get(2)?;
                let status: RunStatus = serde_json::from_str(&format!("\"{status_str}\""))
                    .unwrap_or(RunStatus::Pending);

                Ok(WorkflowRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_id,
                    status,
                    started_at: row.get::<_, Option<String>>(3)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    completed_at: row.get::<_, Option<String>>(4)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    error: row.get(5)?,
                    trigger_type: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query runs: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read run: {e}")))?;

        Ok(runs)
    }

    /// Find paused or pending runs that need recovery (for startup recovery).
    pub fn find_unfinished_runs(&self) -> CoreResult<Vec<WorkflowRun>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, workflow_id, status, started_at, completed_at, error, trigger_type, created_at
                 FROM workflow_runs WHERE status IN ('pending', 'running', 'paused')
                 ORDER BY created_at DESC",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare query: {e}")))?;

        let runs = stmt
            .query_map([], |row| {
                let status_str: String = row.get(2)?;
                let status: RunStatus = serde_json::from_str(&format!("\"{status_str}\""))
                    .unwrap_or(RunStatus::Pending);

                Ok(WorkflowRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    status,
                    started_at: row.get::<_, Option<String>>(3)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    completed_at: row.get::<_, Option<String>>(4)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    error: row.get(5)?,
                    trigger_type: row.get(6)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query unfinished runs: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read run: {e}")))?;

        Ok(runs)
    }

    // ---- NodeRun operations ----

    /// Create a new node run.
    pub fn create_node_run(
        &self,
        workflow_run_id: Uuid,
        node_id: Uuid,
        node_type: NodeType,
        max_attempts: i32,
    ) -> CoreResult<NodeRun> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let node_type_str = serde_json::to_string(&node_type)
            .map_err(|e| CoreError::internal(format!("Failed to serialize node type: {e}")))?
            .trim_matches('"')
            .to_string();

        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO node_runs (id, workflow_run_id, node_id, node_type, status, attempts, max_attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5, ?6)",
            params![
                id.to_string(),
                workflow_run_id.to_string(),
                node_id.to_string(),
                node_type_str,
                max_attempts,
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to create node run: {e}")))?;

        Ok(NodeRun {
            id,
            workflow_run_id,
            node_id,
            node_type,
            status: NodeStatus::Pending,
            input: None,
            output: None,
            error: None,
            started_at: None,
            completed_at: None,
            attempts: 0,
            max_attempts,
            created_at: now,
        })
    }

    /// Get a node run by ID.
    pub fn get_node_run(&self, id: Uuid) -> CoreResult<NodeRun> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.query_row(
            "SELECT id, workflow_run_id, node_id, node_type, status, input, output, error, started_at, completed_at, attempts, max_attempts, created_at
             FROM node_runs WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let node_type_str: String = row.get(3)?;
                let node_type: NodeType = serde_json::from_str(&format!("\"{node_type_str}\"")).unwrap_or(NodeType::Input);
                let status_str: String = row.get(4)?;
                let status: NodeStatus = serde_json::from_str(&format!("\"{status_str}\"")).unwrap_or(NodeStatus::Pending);

                let parse_json = |s: Option<String>| -> Option<Value> {
                    s.and_then(|v| serde_json::from_str(&v).ok())
                };

                Ok(NodeRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_run_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    node_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    node_type,
                    status,
                    input: parse_json(row.get(5)?),
                    output: parse_json(row.get(6)?),
                    error: row.get(7)?,
                    started_at: row.get::<_, Option<String>>(8)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                    }),
                    completed_at: row.get::<_, Option<String>>(9)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))
                    }),
                    attempts: row.get(10)?,
                    max_attempts: row.get(11)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                CoreError::not_found("node_run", id.to_string())
            }
            other => CoreError::internal(format!("Failed to get node run: {other}")),
        })
    }

    /// Update a node run's status and output.
    pub fn update_node_run(
        &self,
        id: Uuid,
        status: NodeStatus,
        input: Option<Value>,
        output: Option<Value>,
        error: Option<String>,
    ) -> CoreResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let status_str = serde_json::to_string(&status)
            .map_err(|e| CoreError::internal(format!("Failed to serialize status: {e}")))?
            .trim_matches('"')
            .to_string();

        let input_json = input
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| CoreError::internal(format!("Failed to serialize input: {e}")))?;

        let output_json = output
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()
            .map_err(|e| CoreError::internal(format!("Failed to serialize output: {e}")))?;

        let now = Utc::now();
        let is_terminal = matches!(
            status,
            NodeStatus::Succeeded | NodeStatus::Failed | NodeStatus::Cancelled
        );

        if is_terminal {
            conn.execute(
                "UPDATE node_runs SET status = ?1, input = ?2, output = ?3, error = ?4, completed_at = ?5 WHERE id = ?6",
                params![status_str, input_json, output_json, error, now.to_rfc3339(), id.to_string()],
            )
        } else if status == NodeStatus::Running {
            conn.execute(
                "UPDATE node_runs SET status = ?1, input = ?2, output = ?3, error = ?4, started_at = ?5 WHERE id = ?6",
                params![status_str, input_json, output_json, error, now.to_rfc3339(), id.to_string()],
            )
        } else {
            conn.execute(
                "UPDATE node_runs SET status = ?1, input = ?2, output = ?3, error = ?4 WHERE id = ?5",
                params![status_str, input_json, output_json, error, id.to_string()],
            )
        }
        .map_err(|e| CoreError::internal(format!("Failed to update node run: {e}")))?;

        Ok(())
    }

    /// Increment attempt counter for a node run.
    pub fn increment_attempt(&self, id: Uuid) -> CoreResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "UPDATE node_runs SET attempts = attempts + 1 WHERE id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| CoreError::internal(format!("Failed to increment attempt: {e}")))?;

        Ok(())
    }

    /// Get all node runs for a workflow run.
    pub fn list_node_runs(&self, workflow_run_id: Uuid) -> CoreResult<Vec<NodeRun>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, workflow_run_id, node_id, node_type, status, input, output, error, started_at, completed_at, attempts, max_attempts, created_at
                 FROM node_runs WHERE workflow_run_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare query: {e}")))?;

        let runs = stmt
            .query_map(params![workflow_run_id.to_string()], |row| {
                let node_type_str: String = row.get(3)?;
                let node_type: NodeType = serde_json::from_str(&format!("\"{node_type_str}\""))
                    .unwrap_or(NodeType::Input);
                let status_str: String = row.get(4)?;
                let status: NodeStatus = serde_json::from_str(&format!("\"{status_str}\""))
                    .unwrap_or(NodeStatus::Pending);

                let parse_json = |s: Option<String>| -> Option<Value> {
                    s.and_then(|v| serde_json::from_str(&v).ok())
                };

                Ok(NodeRun {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_run_id,
                    node_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    node_type,
                    status,
                    input: parse_json(row.get(5)?),
                    output: parse_json(row.get(6)?),
                    error: row.get(7)?,
                    started_at: row.get::<_, Option<String>>(8)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    completed_at: row.get::<_, Option<String>>(9)?.and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .ok()
                            .map(|dt| dt.with_timezone(&Utc))
                    }),
                    attempts: row.get(10)?,
                    max_attempts: row.get(11)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query node runs: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read node run: {e}")))?;

        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_db;

    fn setup_repo(wf_id: Uuid) -> (RunRepository, std::sync::Arc<Mutex<Connection>>) {
        let conn = std::sync::Arc::new(Mutex::new(init_test_db().unwrap()));
        // Insert an agent + workflow so FK constraints pass
        {
            let c = conn.lock().unwrap();
            let agent_id = Uuid::new_v4();
            c.execute(
                "INSERT INTO agents (id, name) VALUES (?1, ?2)",
                rusqlite::params![agent_id.to_string(), "test_agent"],
            )
            .unwrap();
            c.execute(
                "INSERT INTO workflows (id, agent_id, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![wf_id.to_string(), agent_id.to_string(), "test_workflow"],
            )
            .unwrap();
        }
        let repo = RunRepository::new(conn.clone());
        (repo, conn)
    }

    #[test]
    fn test_create_and_get_run() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);
        let run = repo.create_run(wf_id, "manual").unwrap();

        assert_eq!(run.status, RunStatus::Pending);
        assert_eq!(run.trigger_type, "manual");

        let fetched = repo.get_run(run.id).unwrap();
        assert_eq!(fetched.id, run.id);
        assert_eq!(fetched.status, RunStatus::Pending);
    }

    #[test]
    fn test_update_run_status() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);
        let run = repo.create_run(wf_id, "manual").unwrap();

        repo.update_run_status(run.id, RunStatus::Running, None)
            .unwrap();
        let fetched = repo.get_run(run.id).unwrap();
        assert_eq!(fetched.status, RunStatus::Running);
        assert!(fetched.started_at.is_some());

        repo.update_run_status(run.id, RunStatus::Succeeded, None)
            .unwrap();
        let fetched = repo.get_run(run.id).unwrap();
        assert_eq!(fetched.status, RunStatus::Succeeded);
        assert!(fetched.completed_at.is_some());
    }

    #[test]
    fn test_node_run_lifecycle() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);
        let node_id = Uuid::new_v4();
        let run = repo.create_run(wf_id, "manual").unwrap();

        let node_run = repo
            .create_node_run(run.id, node_id, NodeType::Model, 3)
            .unwrap();
        assert_eq!(node_run.status, NodeStatus::Pending);
        assert_eq!(node_run.attempts, 0);

        repo.update_node_run(
            node_run.id,
            NodeStatus::Running,
            Some(serde_json::json!({"prompt": "hello"})),
            None,
            None,
        )
        .unwrap();

        let fetched = repo.get_node_run(node_run.id).unwrap();
        assert_eq!(fetched.status, NodeStatus::Running);
        assert!(fetched.started_at.is_some());
        assert_eq!(fetched.input, Some(serde_json::json!({"prompt": "hello"})));

        repo.update_node_run(
            node_run.id,
            NodeStatus::Succeeded,
            None,
            Some(serde_json::json!({"response": "world"})),
            None,
        )
        .unwrap();

        let fetched = repo.get_node_run(node_run.id).unwrap();
        assert_eq!(fetched.status, NodeStatus::Succeeded);
        assert!(fetched.completed_at.is_some());
        assert_eq!(
            fetched.output,
            Some(serde_json::json!({"response": "world"}))
        );
    }

    #[test]
    fn test_list_runs() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);

        repo.create_run(wf_id, "manual").unwrap();
        repo.create_run(wf_id, "scheduled").unwrap();

        let runs = repo.list_runs(wf_id).unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn test_unfinished_runs() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);

        let run1 = repo.create_run(wf_id, "manual").unwrap();
        let run2 = repo.create_run(wf_id, "manual").unwrap();

        repo.update_run_status(run1.id, RunStatus::Running, None)
            .unwrap();
        repo.update_run_status(run2.id, RunStatus::Succeeded, None)
            .unwrap();

        let unfinished = repo.find_unfinished_runs().unwrap();
        assert_eq!(unfinished.len(), 1);
        assert_eq!(unfinished[0].id, run1.id);
    }

    #[test]
    fn test_increment_attempt() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);
        let run = repo.create_run(wf_id, "manual").unwrap();
        let node_run = repo
            .create_node_run(run.id, Uuid::new_v4(), NodeType::Model, 3)
            .unwrap();

        repo.increment_attempt(node_run.id).unwrap();
        let fetched = repo.get_node_run(node_run.id).unwrap();
        assert_eq!(fetched.attempts, 1);
    }

    #[test]
    fn test_retry_after_failure() {
        let wf_id = Uuid::new_v4();
        let (repo, _conn) = setup_repo(wf_id);
        let run = repo.create_run(wf_id, "manual").unwrap();
        let node_run = repo
            .create_node_run(run.id, Uuid::new_v4(), NodeType::Model, 3)
            .unwrap();

        // Mark as failed
        repo.update_node_run(
            node_run.id,
            NodeStatus::Failed,
            None,
            None,
            Some("API timeout".into()),
        )
        .unwrap();

        // Retry: set back to running
        repo.increment_attempt(node_run.id).unwrap();
        repo.update_node_run(node_run.id, NodeStatus::Running, None, None, None)
            .unwrap();

        let fetched = repo.get_node_run(node_run.id).unwrap();
        assert_eq!(fetched.attempts, 1);
        assert_eq!(fetched.status, NodeStatus::Running);
    }
}
