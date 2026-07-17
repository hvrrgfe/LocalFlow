use std::sync::Mutex;

use chrono::Utc;
use localflow_core::error::{CoreError, CoreResult};
use localflow_core::models::{Agent, AgentInput, PermissionPolicy};
use rusqlite::{Connection, params};
use uuid::Uuid;

/// Repository for Agent CRUD operations.
pub struct AgentRepository {
    conn: std::sync::Arc<Mutex<Connection>>,
}

impl AgentRepository {
    pub fn new(conn: std::sync::Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Create a new agent.
    pub fn create(&self, input: AgentInput) -> CoreResult<Agent> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let permissions = input.permissions.unwrap_or_default();
        let permissions_json = serde_json::to_string(&permissions)
            .map_err(|e| CoreError::internal(format!("Failed to serialize permissions: {e}")))?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO agents (id, name, description, system_prompt, model, temperature, max_tokens, permissions, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.to_string(),
                input.name,
                input.description,
                input.system_prompt,
                input.model,
                input.temperature,
                input.max_tokens,
                permissions_json,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to create agent: {e}")))?;

        Ok(Agent {
            id,
            name: input.name,
            description: input.description,
            system_prompt: input.system_prompt,
            model: input.model,
            temperature: input.temperature,
            max_tokens: input.max_tokens,
            permissions,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get an agent by ID.
    pub fn get(&self, id: Uuid) -> CoreResult<Agent> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let result = conn.query_row(
            "SELECT id, name, description, system_prompt, model, temperature, max_tokens, permissions, created_at, updated_at
             FROM agents WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let permissions_str: String = row.get(7)?;
                let permissions: PermissionPolicy = serde_json::from_str(&permissions_str)
                    .unwrap_or_default();

                Ok(Agent {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?)
                        .unwrap_or_default(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    system_prompt: row.get(3)?,
                    model: row.get(4)?,
                    temperature: row.get(5)?,
                    max_tokens: row.get(6)?,
                    permissions,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            },
        );

        result.map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => CoreError::not_found("agent", id.to_string()),
            other => CoreError::internal(format!("Failed to get agent: {other}")),
        })
    }

    /// List all agents.
    pub fn list(&self) -> CoreResult<Vec<Agent>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, system_prompt, model, temperature, max_tokens, permissions, created_at, updated_at
                 FROM agents ORDER BY created_at DESC",
            )
            .map_err(|e| CoreError::internal(format!("Failed to prepare query: {e}")))?;

        let agents = stmt
            .query_map([], |row| {
                let permissions_str: String = row.get(7)?;
                let permissions: PermissionPolicy =
                    serde_json::from_str(&permissions_str).unwrap_or_default();

                Ok(Agent {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    system_prompt: row.get(3)?,
                    model: row.get(4)?,
                    temperature: row.get(5)?,
                    max_tokens: row.get(6)?,
                    permissions,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| CoreError::internal(format!("Failed to query agents: {e}")))?;

        let mut result = Vec::new();
        for agent in agents {
            result.push(
                agent.map_err(|e| CoreError::internal(format!("Failed to read agent row: {e}")))?,
            );
        }
        Ok(result)
    }

    /// Update an existing agent.
    pub fn update(&self, id: Uuid, input: AgentInput) -> CoreResult<Agent> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        // Verify existence
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM agents WHERE id = ?1",
                params![id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count > 0)
            .map_err(|e| CoreError::internal(format!("Failed to check agent: {e}")))?;

        if !exists {
            return Err(CoreError::not_found("agent", id.to_string()));
        }

        let now = Utc::now();
        let permissions = input.permissions.unwrap_or_default();
        let permissions_json = serde_json::to_string(&permissions)
            .map_err(|e| CoreError::internal(format!("Failed to serialize permissions: {e}")))?;

        conn.execute(
            "UPDATE agents SET name = ?1, description = ?2, system_prompt = ?3, model = ?4,
             temperature = ?5, max_tokens = ?6, permissions = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                input.name,
                input.description,
                input.system_prompt,
                input.model,
                input.temperature,
                input.max_tokens,
                permissions_json,
                now.to_rfc3339(),
                id.to_string(),
            ],
        )
        .map_err(|e| CoreError::internal(format!("Failed to update agent: {e}")))?;

        drop(conn);
        // Re-fetch the updated agent
        let repo = AgentRepository::new(self.conn.clone());
        repo.get(id)
    }

    /// Delete an agent by ID.
    pub fn delete(&self, id: Uuid) -> CoreResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire database lock: {e}")))?;

        let affected = conn
            .execute("DELETE FROM agents WHERE id = ?1", params![id.to_string()])
            .map_err(|e| CoreError::internal(format!("Failed to delete agent: {e}")))?;

        if affected == 0 {
            return Err(CoreError::not_found("agent", id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_test_db;

    fn setup_repo() -> (AgentRepository, std::sync::Arc<Mutex<Connection>>) {
        let conn = std::sync::Arc::new(Mutex::new(init_test_db().unwrap()));
        let repo = AgentRepository::new(conn.clone());
        (repo, conn)
    }

    #[test]
    fn test_create_and_get_agent() {
        let (repo, _conn) = setup_repo();
        let input = AgentInput {
            name: "Test Agent".into(),
            description: Some("A test agent".into()),
            system_prompt: Some("You are helpful".into()),
            model: Some("gpt-4".into()),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            permissions: None,
        };

        let agent = repo.create(input).unwrap();
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.description.as_deref(), Some("A test agent"));

        let fetched = repo.get(agent.id).unwrap();
        assert_eq!(fetched.name, agent.name);
        assert_eq!(fetched.id, agent.id);
    }

    #[test]
    fn test_get_nonexistent_agent() {
        let (repo, _conn) = setup_repo();
        let id = Uuid::new_v4();
        let result = repo.get(id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::NotFound { .. }));
    }

    #[test]
    fn test_list_agents() {
        let (repo, _conn) = setup_repo();
        let input1 = AgentInput {
            name: "Agent 1".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: None,
        };
        let input2 = AgentInput {
            name: "Agent 2".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: None,
        };

        repo.create(input1).unwrap();
        repo.create(input2).unwrap();

        let agents = repo.list().unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_update_agent() {
        let (repo, _conn) = setup_repo();
        let input = AgentInput {
            name: "Original".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: None,
        };

        let agent = repo.create(input).unwrap();

        let update = AgentInput {
            name: "Updated".into(),
            description: Some("Updated description".into()),
            system_prompt: None,
            model: Some("gpt-4o".into()),
            temperature: Some(0.5),
            max_tokens: None,
            permissions: None,
        };

        let updated = repo.update(agent.id, update).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.model.as_deref(), Some("gpt-4o"));
        assert_eq!(updated.temperature, Some(0.5));
    }

    #[test]
    fn test_delete_agent() {
        let (repo, _conn) = setup_repo();
        let input = AgentInput {
            name: "To Delete".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: None,
        };

        let agent = repo.create(input).unwrap();
        repo.delete(agent.id).unwrap();

        let result = repo.get(agent.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent_agent() {
        let (repo, _conn) = setup_repo();
        let result = repo.delete(Uuid::new_v4());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_agent_with_permissions() {
        let (repo, _conn) = setup_repo();
        let permissions = PermissionPolicy {
            allowed_hosts: vec!["api.openai.com".into()],
            allow_loopback: false,
            max_nodes: 100,
            ..Default::default()
        };

        let input = AgentInput {
            name: "Restricted Agent".into(),
            description: None,
            system_prompt: None,
            model: None,
            temperature: None,
            max_tokens: None,
            permissions: Some(permissions.clone()),
        };

        let agent = repo.create(input).unwrap();
        assert_eq!(agent.permissions.allowed_hosts, vec!["api.openai.com"]);
        assert_eq!(agent.permissions.max_nodes, 100);
        assert!(!agent.permissions.allow_loopback);
    }
}
