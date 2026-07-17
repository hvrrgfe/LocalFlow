use std::sync::Mutex;

use chrono::Utc;
use localflow_core::error::{CoreError, CoreResult};
use localflow_core::models::{Workflow, WorkflowEdge, WorkflowNode};
use localflow_core::types::NodeType;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Input for creating a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub agent_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNodeInput>,
    pub edges: Vec<WorkflowEdgeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodeInput {
    pub node_type: NodeType,
    pub name: String,
    pub config: Value,
    pub position_x: f64,
    pub position_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdgeInput {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub source_handle: Option<String>,
    pub target_handle: Option<String>,
    pub condition_expression: Option<String>,
}

/// Repository for Workflow CRUD operations.
pub struct WorkflowRepository {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl WorkflowRepository {
    pub fn new(conn: std::sync::Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create a new workflow with nodes and edges.
    pub fn create(&self, input: WorkflowInput) -> CoreResult<Workflow> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let workflow_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert workflow
        conn.execute(
            "INSERT INTO workflows (id, agent_id, name, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                workflow_id.to_string(),
                input.agent_id.to_string(),
                input.name,
                input.description,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to create workflow: {e}")))?;

        // Insert nodes
        let mut nodes = Vec::new();
        for node_input in &input.nodes {
            let node_id = Uuid::new_v4();
            let config_json = serde_json::to_string(&node_input.config)
                .map_err(|e| CoreError::internal(format!("Failed to serialize config: {e}")))?;

            let node_type_str = serde_json::to_string(&node_input.node_type)
                .map_err(|e| CoreError::internal(format!("Failed to serialize node type: {e}")))?
                .trim_matches('"')
                .to_string();

            conn.execute(
                "INSERT INTO workflow_nodes (id, workflow_id, node_type, name, config, position_x, position_y)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    node_id.to_string(),
                    workflow_id.to_string(),
                    node_type_str,
                    node_input.name,
                    config_json,
                    node_input.position_x,
                    node_input.position_y,
                ],
            )
            .map_err(|e| CoreError::internal(format!("Failed to create node: {e}")))?;

            nodes.push(WorkflowNode {
                id: node_id,
                workflow_id,
                node_type: node_input.node_type,
                name: node_input.name.clone(),
                config: node_input.config.clone(),
                position_x: node_input.position_x,
                position_y: node_input.position_y,
            });
        }

        // Insert edges
        let mut edges = Vec::new();
        for edge_input in &input.edges {
            let edge_id = Uuid::new_v4();

            conn.execute(
                "INSERT INTO workflow_edges (id, workflow_id, source_node_id, target_node_id, source_handle, target_handle, condition_expression)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    edge_id.to_string(),
                    workflow_id.to_string(),
                    edge_input.source_node_id.to_string(),
                    edge_input.target_node_id.to_string(),
                    edge_input.source_handle,
                    edge_input.target_handle,
                    edge_input.condition_expression,
                ],
            )
            .map_err(|e| CoreError::internal(format!("Failed to create edge: {e}")))?;

            edges.push(WorkflowEdge {
                id: edge_id,
                workflow_id,
                source_node_id: edge_input.source_node_id,
                target_node_id: edge_input.target_node_id,
                source_handle: edge_input.source_handle.clone(),
                target_handle: edge_input.target_handle.clone(),
                condition_expression: edge_input.condition_expression.clone(),
            });
        }

        Ok(Workflow {
            id: workflow_id,
            agent_id: input.agent_id,
            name: input.name,
            description: input.description,
            nodes,
            edges,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get a workflow by ID, including nodes and edges.
    pub fn get(&self, id: Uuid) -> CoreResult<Workflow> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let workflow = conn
            .query_row(
                "SELECT id, agent_id, name, description, created_at, updated_at
                 FROM workflows WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    CoreError::not_found("workflow", id.to_string())
                }
                other => CoreError::internal(format!("Failed to get workflow: {other}")),
            })?;

        let (wf_id, agent_id, name, description, created_at_str, updated_at_str) = workflow;

        // Load nodes
        let mut node_stmt = conn
            .prepare(
                "SELECT id, node_type, name, config, position_x, position_y
                 FROM workflow_nodes WHERE workflow_id = ?1",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare node query: {e}")))?;

        let node_results: Vec<WorkflowNode> = node_stmt
            .query_map(params![id.to_string()], |row| {
                let node_type_str: String = row.get(1)?;
                let node_type: NodeType = serde_json::from_str(&format!("\"{node_type_str}\""))
                    .unwrap_or(NodeType::Input);
                let config_str: String = row.get(3)?;
                let config: Value = serde_json::from_str(&config_str).unwrap_or(Value::Null);

                Ok(WorkflowNode {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_id: id,
                    node_type,
                    name: row.get(2)?,
                    config,
                    position_x: row.get(4)?,
                    position_y: row.get(5)?,
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query nodes: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read node: {e}")))?;

        // Load edges
        let mut edge_stmt = conn
            .prepare(
                "SELECT id, source_node_id, target_node_id, source_handle, target_handle, condition_expression
                 FROM workflow_edges WHERE workflow_id = ?1",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare edge query: {e}")))?;

        let edge_results: Vec<WorkflowEdge> = edge_stmt
            .query_map(params![id.to_string()], |row| {
                Ok(WorkflowEdge {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    workflow_id: id,
                    source_node_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    target_node_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                    source_handle: row.get(3)?,
                    target_handle: row.get(4)?,
                    condition_expression: row.get(5)?,
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query edges: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read edge: {e}")))?;

        let parse_dt = |s: &str| -> chrono::DateTime<Utc> {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_default()
        };

        Ok(Workflow {
            id: Uuid::parse_str(&wf_id).unwrap_or_default(),
            agent_id: Uuid::parse_str(&agent_id).unwrap_or_default(),
            name,
            description,
            nodes: node_results,
            edges: edge_results,
            created_at: parse_dt(&created_at_str),
            updated_at: parse_dt(&updated_at_str),
        })
    }

    /// List all workflows, optionally filtered by agent_id.
    pub fn list(&self, agent_id: Option<Uuid>) -> CoreResult<Vec<Workflow>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) =
            if let Some(aid) = agent_id {
                (
                    "SELECT id FROM workflows WHERE agent_id = ?1 ORDER BY created_at DESC".into(),
                    vec![Box::new(aid.to_string())],
                )
            } else {
                (
                    "SELECT id FROM workflows ORDER BY created_at DESC".into(),
                    vec![],
                )
            };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| CoreError::internal(format!("Failed to prepare query: {e}")))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let ids: Vec<String> = stmt
            .query_map(param_refs.as_slice(), |row| row.get::<_, String>(0))
            .map_err(|e| CoreError::internal(format!("Failed to list workflows: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CoreError::internal(format!("Failed to read workflow ids: {e}")))?;

        drop(stmt);
        drop(conn);

        let repo = WorkflowRepository::new(self.conn.clone());
        let mut workflows = Vec::new();
        for id_str in ids {
            if let Ok(wf_id) = Uuid::parse_str(&id_str) {
                if let Ok(workflow) = repo.get(wf_id) {
                    workflows.push(workflow);
                }
            }
        }
        Ok(workflows)
    }

    /// Update a workflow (replaces nodes and edges).
    pub fn update(&self, id: Uuid, input: WorkflowInput) -> CoreResult<Workflow> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        // Start transaction
        conn.execute_batch("BEGIN TRANSACTION")
            .map_err(|e| CoreError::internal(format!("Failed to begin transaction: {e}")))?;

        let result = self.update_internal(&conn, id, input);

        match result {
            Ok(workflow) => {
                conn.execute_batch("COMMIT")
                    .map_err(|e| CoreError::internal(format!("Failed to commit: {e}")))?;
                Ok(workflow)
            }
            Err(e) => {
                conn.execute_batch("ROLLBACK")
                    .map_err(|_| ()) // Ignore rollback errors
                    .ok();
                Err(e)
            }
        }
    }

    fn update_internal(
        &self,
        conn: &Connection,
        id: Uuid,
        input: WorkflowInput,
    ) -> CoreResult<Workflow> {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM workflows WHERE id = ?1",
                params![id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|e| CoreError::internal(format!("Failed to check workflow: {e}")))?;

        if !exists {
            return Err(CoreError::not_found("workflow", id.to_string()));
        }

        let now = Utc::now();

        // Update workflow header
        conn.execute(
            "UPDATE workflows SET agent_id = ?1, name = ?2, description = ?3, updated_at = ?4 WHERE id = ?5",
            params![
                input.agent_id.to_string(),
                input.name,
                input.description,
                now.to_rfc3339(),
                id.to_string(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to update workflow: {e}")))?;

        // Delete existing nodes and edges (CASCADE handles edges)
        conn.execute(
            "DELETE FROM workflow_nodes WHERE workflow_id = ?1",
            params![id.to_string()],
        )
        .map_err(|e| CoreError::internal(format!("Failed to delete old nodes: {e}")))?;

        // Re-insert nodes
        let mut nodes = Vec::new();
        for node_input in &input.nodes {
            let node_id = Uuid::new_v4();
            let config_json = serde_json::to_string(&node_input.config)
                .map_err(|e| CoreError::internal(format!("Failed to serialize config: {e}")))?;

            let node_type_str = serde_json::to_string(&node_input.node_type)
                .map_err(|e| CoreError::internal(format!("Failed to serialize node type: {e}")))?
                .trim_matches('"')
                .to_string();

            conn.execute(
                "INSERT INTO workflow_nodes (id, workflow_id, node_type, name, config, position_x, position_y)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    node_id.to_string(),
                    id.to_string(),
                    node_type_str,
                    node_input.name,
                    config_json,
                    node_input.position_x,
                    node_input.position_y,
                ],
            )
            .map_err(|e| CoreError::internal(format!("Failed to create node: {e}")))?;

            nodes.push(WorkflowNode {
                id: node_id,
                workflow_id: id,
                node_type: node_input.node_type,
                name: node_input.name.clone(),
                config: node_input.config.clone(),
                position_x: node_input.position_x,
                position_y: node_input.position_y,
            });
        }

        // Re-insert edges
        let mut edges = Vec::new();
        for edge_input in &input.edges {
            let edge_id = Uuid::new_v4();

            conn.execute(
                "INSERT INTO workflow_edges (id, workflow_id, source_node_id, target_node_id, source_handle, target_handle, condition_expression)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    edge_id.to_string(),
                    id.to_string(),
                    edge_input.source_node_id.to_string(),
                    edge_input.target_node_id.to_string(),
                    edge_input.source_handle,
                    edge_input.target_handle,
                    edge_input.condition_expression,
                ],
            )
            .map_err(|e| CoreError::internal(format!("Failed to create edge: {e}")))?;

            edges.push(WorkflowEdge {
                id: edge_id,
                workflow_id: id,
                source_node_id: edge_input.source_node_id,
                target_node_id: edge_input.target_node_id,
                source_handle: edge_input.source_handle.clone(),
                target_handle: edge_input.target_handle.clone(),
                condition_expression: edge_input.condition_expression.clone(),
            });
        }

        Ok(Workflow {
            id,
            agent_id: input.agent_id,
            name: input.name,
            description: input.description,
            nodes,
            edges,
            created_at: Utc::now(),
            updated_at: now,
        })
    }

    /// Delete a workflow by ID.
    pub fn delete(&self, id: Uuid) -> CoreResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let affected = conn
            .execute(
                "DELETE FROM workflows WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(|e| CoreError::internal(format!("Failed to delete workflow: {e}")))?;

        if affected == 0 {
            return Err(CoreError::not_found("workflow", id.to_string()));
        }
        Ok(())
    }

    /// Validate the DAG structure of a workflow.
    pub fn validate_dag(&self, workflow: &Workflow) -> CoreResult<()> {
        if !workflow.is_valid_dag() {
            return Err(CoreError::invalid_workflow(
                "Workflow contains cycles; only DAGs are supported",
            ));
        }
        if workflow.start_node().is_none() {
            return Err(CoreError::invalid_workflow(
                "Workflow must have exactly one Start node",
            ));
        }
        if workflow.end_nodes().is_empty() {
            return Err(CoreError::invalid_workflow(
                "Workflow must have at least one End node",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_db;

    fn setup_repo() -> (WorkflowRepository, std::sync::Arc<Mutex<Connection>>, Uuid) {
        let conn = std::sync::Arc::new(Mutex::new(init_test_db().unwrap()));
        let agent_id = Uuid::new_v4();
        {
            let c = conn.lock().unwrap();
            c.execute(
                "INSERT INTO agents (id, name) VALUES (?1, ?2)",
                rusqlite::params![agent_id.to_string(), "test_agent"],
            )
            .unwrap();
        }
        let repo = WorkflowRepository::new(conn.clone());
        (repo, conn, agent_id)
    }

    #[test]
    fn test_create_and_get_workflow() {
        let (repo, _conn, agent_id) = setup_repo();

        let input = WorkflowInput {
            agent_id,
            name: "Test Workflow".into(),
            description: Some("A test workflow".into()),
            nodes: vec![
                WorkflowNodeInput {
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        let workflow = repo.create(input).unwrap();
        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.nodes.len(), 2);

        let fetched = repo.get(workflow.id).unwrap();
        assert_eq!(fetched.name, workflow.name);
        assert_eq!(fetched.nodes.len(), 2);
    }

    #[test]
    fn test_get_nonexistent_workflow() {
        let (repo, _conn, _agent_id) = setup_repo();
        let _result = repo.get(Uuid::new_v4());
    }

    #[test]
    fn test_list_workflows() {
        let (repo, _conn, agent_id) = setup_repo();

        let input = WorkflowInput {
            agent_id,
            name: "WF 1".into(),
            description: None,
            nodes: vec![
                WorkflowNodeInput {
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        repo.create(input).unwrap();

        let workflows = repo.list(None).unwrap();
        assert_eq!(workflows.len(), 1);
    }

    #[test]
    fn test_update_workflow() {
        let (repo, _conn, agent_id) = setup_repo();

        let input = WorkflowInput {
            agent_id,
            name: "Original".into(),
            description: None,
            nodes: vec![
                WorkflowNodeInput {
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        let workflow = repo.create(input).unwrap();

        let update_input = WorkflowInput {
            agent_id,
            name: "Updated".into(),
            description: Some("Updated desc".into()),
            nodes: vec![
                WorkflowNodeInput {
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::Model,
                    name: "GPT".into(),
                    config: serde_json::json!({"model": "gpt-4"}),
                    position_x: 100.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 300.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        let updated = repo.update(workflow.id, update_input).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.nodes.len(), 3);
    }

    #[test]
    fn test_delete_workflow() {
        let (repo, _conn, agent_id) = setup_repo();

        let input = WorkflowInput {
            agent_id,
            name: "To Delete".into(),
            description: None,
            nodes: vec![
                WorkflowNodeInput {
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNodeInput {
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![],
        };

        let workflow = repo.create(input).unwrap();
        repo.delete(workflow.id).unwrap();

        let result = repo.get(workflow.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_dag() {
        let (repo, _conn, agent_id) = setup_repo();

        let start_id = Uuid::new_v4();
        let end_id = Uuid::new_v4();

        let workflow = Workflow {
            id: Uuid::new_v4(),
            agent_id,
            name: "Valid DAG".into(),
            description: None,
            nodes: vec![
                WorkflowNode {
                    id: start_id,
                    workflow_id: Uuid::new_v4(),
                    node_type: NodeType::Start,
                    name: "Start".into(),
                    config: Value::Null,
                    position_x: 0.0,
                    position_y: 0.0,
                },
                WorkflowNode {
                    id: end_id,
                    workflow_id: Uuid::new_v4(),
                    node_type: NodeType::End,
                    name: "End".into(),
                    config: Value::Null,
                    position_x: 200.0,
                    position_y: 0.0,
                },
            ],
            edges: vec![WorkflowEdge {
                id: Uuid::new_v4(),
                workflow_id: Uuid::new_v4(),
                source_node_id: start_id,
                target_node_id: end_id,
                source_handle: None,
                target_handle: None,
                condition_expression: None,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(repo.validate_dag(&workflow).is_ok());
    }
}
