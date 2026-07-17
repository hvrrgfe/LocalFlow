use localflow_core::error::{CoreError, CoreResult};

/// Trait for secret storage backends.
pub trait SecretVault: Send + Sync {
    /// Store a secret value.
    fn store(&self, key: &str, value: &str) -> CoreResult<()>;

    /// Retrieve a secret value.
    fn get(&self, key: &str) -> CoreResult<String>;

    /// Delete a secret.
    fn delete(&self, key: &str) -> CoreResult<()>;

    /// Check if a secret exists.
    fn exists(&self, key: &str) -> CoreResult<bool>;
}

/// An in-memory secret vault for testing and development.
/// **Do not use in production.** Secrets are stored in plaintext in memory.
pub struct InMemoryVault {
    secrets: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl InMemoryVault {
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Create a vault that is populated with some test secrets.
    pub fn with_test_secrets() -> Self {
        let vault = Self::new();
        vault
            .store("test/provider/openai", "sk-test-key-12345")
            .ok();
        vault
            .store("test/provider/deepseek", "sk-deepseek-test")
            .ok();
        vault
    }
}

impl Default for InMemoryVault {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretVault for InMemoryVault {
    fn store(&self, key: &str, value: &str) -> CoreResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire vault lock: {e}")))?;
        secrets.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> CoreResult<String> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire vault lock: {e}")))?;
        secrets
            .get(key)
            .cloned()
            .ok_or_else(|| CoreError::not_found("secret", key))
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let mut secrets = self
            .secrets
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire vault lock: {e}")))?;
        secrets.remove(key);
        Ok(())
    }

    fn exists(&self, key: &str) -> CoreResult<bool> {
        let secrets = self
            .secrets
            .lock()
            .map_err(|e| CoreError::internal(format!("Failed to acquire vault lock: {e}")))?;
        Ok(secrets.contains_key(key))
    }
}

/// Resolve a secret reference URI to an actual secret value.
/// Format: "secret://{vault_key}"
pub fn resolve_secret_ref(
    vault: &dyn SecretVault,
    secret_ref: &localflow_core::models::SecretRefUri,
) -> CoreResult<String> {
    let vault_key = format!("{}/{}", secret_ref.owner_type, secret_ref.owner_name);
    vault.get(&vault_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use localflow_core::models::SecretRefUri;

    #[test]
    fn test_in_memory_vault_store_and_get() {
        let vault = InMemoryVault::new();
        vault.store("test/key", "secret-value").unwrap();
        assert_eq!(vault.get("test/key").unwrap(), "secret-value");
    }

    #[test]
    fn test_in_memory_vault_get_nonexistent() {
        let vault = InMemoryVault::new();
        let result = vault.get("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CoreError::NotFound { .. }));
    }

    #[test]
    fn test_in_memory_vault_delete() {
        let vault = InMemoryVault::new();
        vault.store("test/key", "value").unwrap();
        assert!(vault.exists("test/key").unwrap());
        vault.delete("test/key").unwrap();
        assert!(!vault.exists("test/key").unwrap());
    }

    #[test]
    fn test_resolve_secret_ref() {
        let vault = InMemoryVault::with_test_secrets();
        let ref_uri = SecretRefUri::parse("secret://test/provider/openai").unwrap();
        let value = resolve_secret_ref(&vault, &ref_uri).unwrap();
        assert_eq!(value, "sk-test-key-12345");
    }

    #[test]
    fn test_parse_secret_ref() {
        let uri = SecretRefUri::parse("secret://provider/deepseek").unwrap();
        assert_eq!(uri.owner_type, "provider");
        assert_eq!(uri.owner_name, "deepseek");
    }

    #[test]
    fn test_parse_invalid_secret_ref() {
        assert!(SecretRefUri::parse("invalid").is_err());
        assert!(SecretRefUri::parse("secret://").is_err());
        assert!(SecretRefUri::parse("secret://provider").is_err());
    }
}
