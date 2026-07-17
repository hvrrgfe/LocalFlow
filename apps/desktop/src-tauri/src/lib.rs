mod commands;

use localflow_secret_vault::{EncryptedFileVault, SecretVault};
use localflow_storage::StorageEngine;
use std::sync::Arc;
use tauri::Manager;

/// Application state shared across Tauri commands.
pub struct AppState {
    pub storage: StorageEngine,
    pub vault: Arc<dyn SecretVault>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("LOCALFLOW_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Resolve app data directory for persistent storage
            let app_data_dir = app.path().app_data_dir().expect("failed to resolve app data dir");
            tracing::info!("App data directory: {}", app_data_dir.display());

            std::fs::create_dir_all(&app_data_dir)
                .expect("failed to create app data directory");

            // Persistent SQLite database
            let db_path = app_data_dir.join("localflow.db");
            let storage = StorageEngine::new(
                db_path.to_str().expect("invalid db path"),
            )
            .expect("Failed to initialize storage");

            // Encrypted file vault for secrets
            let vault_dir = app_data_dir.join("vault");
            let vault = Arc::new(
                EncryptedFileVault::new(&vault_dir)
                    .expect("Failed to initialize encrypted vault"),
            );

            app.manage(AppState {
                storage,
                vault: vault as Arc<dyn SecretVault>,
            });

            Ok(())
        })
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
            commands::openapi::import_openapi,
            commands::chat::chat_with_agent,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
