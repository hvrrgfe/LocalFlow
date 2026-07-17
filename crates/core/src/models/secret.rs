use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::SecretType;

/// A reference to a stored secret in the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReference {
    pub id: Uuid,
    /// The provider or entity that owns this secret.
    pub owner_id: Uuid,
    pub secret_type: SecretType,
    /// Human-readable key name.
    pub secret_key: String,
    /// The key used to store/retrieve from the system credential store.
    pub vault_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Parsed secret reference URI.
/// Format: "secret://{owner_type}/{owner_name}"
#[derive(Debug, Clone)]
pub struct SecretRefUri {
    pub owner_type: String,
    pub owner_name: String,
}

impl SecretRefUri {
    /// Parse a secret reference URI string.
    pub fn parse(uri: &str) -> Result<Self, String> {
        let stripped = uri.strip_prefix("secret://").ok_or_else(|| {
            format!("Invalid secret reference: '{uri}': must start with 'secret://'")
        })?;

        let parts: Vec<&str> = stripped.splitn(2, '/').collect();
        if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(format!(
                "Invalid secret reference: '{uri}': expected format 'secret://owner_type/owner_name'"
            ));
        }

        Ok(Self {
            owner_type: parts[0].to_string(),
            owner_name: parts[1].to_string(),
        })
    }
}
