use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_core::models::Agent;
use localflow_core::models::AgentInput;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_agents(state: State<AppState>) -> CoreResult<Vec<Agent>> {
    state.storage.agents.list()
}

#[tauri::command]
pub fn get_agent(state: State<AppState>, id: String) -> CoreResult<Agent> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid agent ID: {e}"))
    })?;
    state.storage.agents.get(uuid)
}

#[tauri::command]
pub fn create_agent(state: State<AppState>, input: AgentInput) -> CoreResult<Agent> {
    state.storage.agents.create(input)
}

#[tauri::command]
pub fn update_agent(state: State<AppState>, id: String, input: AgentInput) -> CoreResult<Agent> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid agent ID: {e}"))
    })?;
    state.storage.agents.update(uuid, input)
}

#[tauri::command]
pub fn delete_agent(state: State<AppState>, id: String) -> CoreResult<()> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid agent ID: {e}"))
    })?;
    state.storage.agents.delete(uuid)
}

#[tauri::command]
pub fn export_agent(state: State<AppState>, id: String) -> CoreResult<String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid agent ID: {e}"))
    })?;
    let agent = state.storage.agents.get(uuid)?;
    // Export without API keys - only metadata
    let export = serde_json::json!({
        "name": agent.name,
        "description": agent.description,
        "system_prompt": agent.system_prompt,
        "model": agent.model,
        "temperature": agent.temperature,
        "max_tokens": agent.max_tokens,
    });
    serde_json::to_string_pretty(&export).map_err(|e| {
        localflow_core::error::CoreError::internal(format!("Serialization failed: {e}"))
    })
}

#[tauri::command]
pub fn import_agent(state: State<AppState>, json_data: String) -> CoreResult<Agent> {
    let input: AgentInput = serde_json::from_str(&json_data).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid agent JSON: {e}"))
    })?;
    state.storage.agents.create(input)
}
