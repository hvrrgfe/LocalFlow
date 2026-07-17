use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_core::models::AuditLog;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct UrlValidationResult {
    pub valid: bool,
    pub message: String,
}

#[tauri::command]
pub fn get_audit_logs(state: State<AppState>, limit: Option<usize>) -> CoreResult<Vec<AuditLog>> {
    state.storage.audit.list(None, None, limit.unwrap_or(50))
}

#[tauri::command]
pub fn validate_url(_state: State<AppState>, url: String) -> CoreResult<UrlValidationResult> {
    let result = localflow_security::validate_url(&url, &[], false);
    match result {
        Ok(_) => Ok(UrlValidationResult {
            valid: true,
            message: "URL is valid and safe".into(),
        }),
        Err(e) => Ok(UrlValidationResult {
            valid: false,
            message: e.to_string(),
        }),
    }
}
