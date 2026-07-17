use crate::AppState;
use localflow_core::error::CoreResult;
use localflow_api_tools::OpenApiParser;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct OpenApiImportResult {
    pub valid: bool,
    pub message: String,
    pub endpoints: Vec<ToolDefinition>,
}

#[tauri::command]
pub fn import_openapi(state: State<AppState>, raw_document: String) -> CoreResult<OpenApiImportResult> {
    let _ = &state.storage;

    match OpenApiParser::validate_raw(&raw_document) {
        Ok(()) => {
            let endpoints = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw_document) {
                let paths = json.get("paths").and_then(|p| p.as_object()).map(|p| p.clone()).unwrap_or_default();
                let mut tools = Vec::new();
                for (path, path_item) in &paths {
                    if let Some(obj) = path_item.as_object() {
                        for method in ["get", "post", "put", "delete", "patch"] {
                            if let Some(op) = obj.get(method) {
                                let op_name = op.get("operationId")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_")));
                                let desc = op.get("description")
                                    .or_else(|| op.get("summary"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| format!("{} {}", method.to_uppercase(), path));
                                tools.push(ToolDefinition {
                                    name: op_name,
                                    description: desc,
                                    method: method.to_uppercase(),
                                    path: path.clone(),
                                });
                            }
                        }
                    }
                }
                tools
            } else {
                Vec::new()
            };

            Ok(OpenApiImportResult {
                valid: true,
                message: format!("Successfully imported, found {} endpoints", endpoints.len()),
                endpoints,
            })
        }
        Err(e) => Ok(OpenApiImportResult {
            valid: false,
            message: e.to_string(),
            endpoints: vec![],
        }),
    }
}
