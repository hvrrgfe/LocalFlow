use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::ProviderType;

/// Configuration for a model/API provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: Uuid,
    pub provider_type: ProviderType,
    pub name: String,
    pub base_url: String,
    /// Reference to the API key in the secret vault (e.g., "secret://provider/deepseek").
    pub api_key_secret_ref: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating or updating a ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigInput {
    pub provider_type: ProviderType,
    pub name: String,
    pub base_url: String,
    pub api_key_secret_ref: String,
}
