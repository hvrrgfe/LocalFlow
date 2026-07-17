use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_core::models::{NodeRun, WorkflowRun};
use localflow_workflow_engine::DagRunner;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn list_runs(state: State<AppState>, workflow_id: String) -> CoreResult<Vec<WorkflowRun>> {
    let uuid = Uuid::parse_str(&workflow_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    state.storage.runs.list_runs(uuid)
}

#[tauri::command]
pub fn get_run(state: State<AppState>, id: String) -> CoreResult<WorkflowRun> {
    let uuid = Uuid::parse_str(&id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid run ID: {e}"))
    })?;
    state.storage.runs.get_run(uuid)
}

#[tauri::command]
pub async fn start_run(state: State<'_, AppState>, workflow_id: String) -> CoreResult<WorkflowRun> {
    let uuid = Uuid::parse_str(&workflow_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    let workflow = state.storage.workflows.get(uuid)?;
    let storage = Arc::new(state.storage.runs.clone());
    let runner = DagRunner::new(storage, state.vault.clone());
    runner
        .run_workflow_from_start(workflow, "manual")
        .await
        .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))
}

#[tauri::command]
pub async fn cancel_run(state: State<'_, AppState>, run_id: String) -> CoreResult<()> {
    let uuid = Uuid::parse_str(&run_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid run ID: {e}"))
    })?;
    state.storage.runs.update_run_status(
        uuid,
        localflow_core::models::RunStatus::Cancelled,
        Some("Cancelled by user".into()),
    )
}

#[tauri::command]
pub async fn retry_run(
    state: State<'_, AppState>,
    workflow_id: String,
    run_id: String,
) -> CoreResult<WorkflowRun> {
    let wf_uuid = Uuid::parse_str(&workflow_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid workflow ID: {e}"))
    })?;
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid run ID: {e}"))
    })?;

    let workflow = state.storage.workflows.get(wf_uuid)?;
    let existing_run = state.storage.runs.get_run(run_uuid)?;
    let storage = Arc::new(state.storage.runs.clone());
    let runner = DagRunner::new(storage, state.vault.clone());
    runner
        .resume_workflow(workflow, &existing_run)
        .await
        .map_err(|e| localflow_core::error::CoreError::internal(e.to_string()))
}

#[tauri::command]
pub fn get_node_runs(state: State<AppState>, run_id: String) -> CoreResult<Vec<NodeRun>> {
    let uuid = Uuid::parse_str(&run_id).map_err(|e| {
        localflow_core::error::CoreError::validation(format!("Invalid run ID: {e}"))
    })?;
    state.storage.runs.list_node_runs(uuid)
}
