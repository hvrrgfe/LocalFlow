use crate::AppState;
use localflow_core::error::CoreResult;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct SecretInfo {
    pub key: String,
    pub exists: bool,
}

#[tauri::command]
pub fn store_secret(state: State<AppState>, key: String, value: String) -> CoreResult<()> {
    if !key.starts_with("provider/") {
        return Err(localflow_core::error::CoreError::validation(
            "Secret key must start with 'provider/'",
        ));
    }
    state.vault.store(&key, &value)
}

#[tauri::command]
pub fn delete_secret(state: State<AppState>, key: String) -> CoreResult<()> {
    state.vault.delete(&key)
}

#[tauri::command]
pub fn list_secrets(state: State<AppState>) -> CoreResult<Vec<SecretInfo>> {
    let known_keys = ["provider/openai", "provider/deepseek", "provider/custom"];
    let infos = known_keys
        .iter()
        .map(|key| SecretInfo {
            key: key.to_string(),
            exists: state.vault.exists(key).unwrap_or(false),
        })
        .collect();
    Ok(infos)
}

#[tauri::command]
pub fn check_secret_exists(state: State<AppState>, key: String) -> CoreResult<bool> {
    state.vault.exists(&key)
}
