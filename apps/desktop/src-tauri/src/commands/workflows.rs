use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_core::models::Workflow;
use localflow_storage::WorkflowInput;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_workflows(
    state: State<AppState>,
    agent_id: Option<String>,
) -> CoreResult<Vec<Workflow>> {
    let uuid = match agent_id {
        Some(id) => Some(Uuid::parse_str(&id).map_err(|e| {
            localflow_core::error::CoreError::validation(format!("Invalid agent ID: {e}"))
        })?),
        None => None,
    };
    state.storage.workflows.list(uuid)
}

#[tauri::command]
pub fn get_workflow(state: State<AppState>, id: String) -> CoreResult<Workflow> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    state.storage.workflows.get(uuid)
}

#[tauri::command]
pub fn create_workflow(state: State<AppState>, input: WorkflowInput) -> CoreResult<Workflow> {
    state.storage.workflows.create(input)
}

#[tauri::command]
pub fn update_workflow(
    state: State<AppState>,
    id: String,
    input: WorkflowInput,
) -> CoreResult<Workflow> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    state.storage.workflows.update(uuid, input)
}

#[tauri::command]
pub fn delete_workflow(state: State<AppState>, id: String) -> CoreResult<()> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    state.storage.workflows.delete(uuid)
}
