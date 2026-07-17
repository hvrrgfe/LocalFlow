use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_secret_vault::SecretVault;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    pub has_api_key: bool,
}

#[tauri::command]
pub fn list_providers(state: State<AppState>) -> CoreResult<Vec<ProviderInfo>> {
    let agents = state.storage.agents.list()?;
    let mut infos = Vec::new();
    for agent in agents {
        if let Some(model) = agent.model {
            infos.push(ProviderInfo {
                id: agent.id.to_string(),
                name: format!("{} ({})", agent.name, model),
                provider_type: "openai_compatible".into(),
                base_url: "https://api.openai.com/v1".into(),
                has_api_key: state
                    .vault
                    .exists(&format!("provider/{}", agent.id))
                    .unwrap_or(false),
            });
        }
    }
    let default_keys = ["provider/openai", "provider/deepseek", "provider/custom"];
    for key in &default_keys {
        if !infos.iter().any(|i| i.id == *key) {
            infos.push(ProviderInfo {
                id: key.to_string(),
                name: key.strip_prefix("provider/").unwrap_or(key).to_string(),
                provider_type: "openai_compatible".into(),
                base_url: match *key {
                    "provider/openai" => "https://api.openai.com/v1".into(),
                    "provider/deepseek" => "https://api.deepseek.com/v1".into(),
                    _ => "https://api.example.com/v1".into(),
                },
                has_api_key: state.vault.exists(key).unwrap_or(false),
            });
        }
    }
    Ok(infos)
}

#[tauri::command]
pub fn save_provider(
    state: State<AppState>,
    id: String,
    name: String,
    base_url: String,
) -> CoreResult<()> {
    let meta_key = format!("provider_meta/{}", id);
    let meta = serde_json::json!({"name": name, "base_url": base_url});
    state.vault.store(&meta_key, &meta.to_string())
}

#[tauri::command]
pub fn delete_provider(state: State<AppState>, id: String) -> CoreResult<()> {
    state.vault.delete(&format!("provider/{}", id))?;
    let _ = state.vault.delete(&format!("provider_meta/{}", id));
    Ok(())
}
