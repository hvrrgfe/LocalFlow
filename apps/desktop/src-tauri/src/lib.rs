mod commands;

use localflow_secret_vault::InMemoryVault;
use localflow_storage::StorageEngine;
use std::sync::Arc;

/// Application state shared across Tauri commands.
pub struct AppState {
    pub storage: StorageEngine,
    pub vault: Arc<InMemoryVault>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("LOCALFLOW_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let storage = StorageEngine::new_in_memory().expect("Failed to initialize storage");

    let vault = Arc::new(InMemoryVault::with_test_secrets());

    let app_state = AppState { storage, vault };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::agents::list_agents,
            commands::agents::get_agent,
            commands::agents::create_agent,
            commands::agents::update_agent,
            commands::agents::delete_agent,
            commands::agents::export_agent,
            commands::agents::import_agent,
            commands::workflows::list_workflows,
            commands::workflows::get_workflow,
            commands::workflows::create_workflow,
            commands::workflows::update_workflow,
            commands::workflows::delete_workflow,
            commands::runs::list_runs,
            commands::runs::get_run,
            commands::runs::start_run,
            commands::runs::cancel_run,
            commands::runs::retry_run,
            commands::runs::get_node_runs,
            commands::providers::list_providers,
            commands::providers::save_provider,
            commands::providers::delete_provider,
            commands::secrets::store_secret,
            commands::secrets::delete_secret,
            commands::secrets::list_secrets,
            commands::secrets::check_secret_exists,
            commands::security::get_audit_logs,
            commands::security::validate_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
