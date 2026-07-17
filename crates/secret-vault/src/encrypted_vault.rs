use localflow_core::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand::RngCore;

/// An AES-256-GCM encrypted file vault for storing secrets on disk.
///
/// Secrets are encrypted at rest using a randomly generated master key.
/// The key file and secret file are stored in the designated data directory.
///
/// **Security note**: This is a fallback when OS keychain is unavailable.
/// The master key itself is stored on disk (in a separate file) — this protects
/// against accidental disclosure but not against an attacker who has full filesystem access.
pub struct EncryptedFileVault {
    secrets_file: PathBuf,
    key_file: PathBuf,
    secrets: Mutex<std::collections::HashMap<String, String>>,
    dirty: Mutex<bool>,
}

#[derive(Serialize, Deserialize)]
struct EncryptedPayload {
    nonce: String,     // base64
    ciphertext: String, // base64
}

impl EncryptedFileVault {
    /// Create or open an encrypted vault at the given directory.
    ///
    /// If the key file doesn't exist, a new master key is generated.
    /// Secrets are loaded (decrypted) on creation.
    pub fn new(data_dir: &std::path::Path) -> CoreResult<Self> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            CoreError::internal(format!(
                "Failed to create vault data directory '{}': {e}",
                data_dir.display()
            ))
        })?;

        let key_file = data_dir.join(".vault_key");
        let secrets_file = data_dir.join("secrets.enc");

        // Load or generate master key
        let key_bytes = if key_file.exists() {
            let encoded = std::fs::read_to_string(&key_file).map_err(|e| {
                CoreError::internal(format!("Failed to read vault key file: {e}"))
            })?;
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                encoded.trim(),
            )
            .map_err(|e| CoreError::internal(format!("Failed to decode vault key: {e}")))?
        } else {
            let mut key = [0u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut key);
            let encoded = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                key,
            );
            std::fs::write(&key_file, encoded).map_err(|e| {
                CoreError::internal(format!("Failed to write vault key file: {e}"))
            })?;
            key.to_vec()
        };

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| CoreError::internal(format!("Invalid AES key: {e}")))?;

        // Load existing secrets
        let secrets = if secrets_file.exists() {
            let data = std::fs::read_to_string(&secrets_file).map_err(|e| {
                CoreError::internal(format!("Failed to read secrets file: {e}"))
            })?;
            let payload: EncryptedPayload = serde_json::from_str(&data).map_err(|e| {
                CoreError::internal(format!("Failed to parse encrypted secrets: {e}"))
            })?;
            let nonce_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &payload.nonce,
            )
            .map_err(|e| CoreError::internal(format!("Failed to decode nonce: {e}")))?;
            let nonce = Nonce::from_slice(&nonce_bytes);
            let plaintext = cipher
                .decrypt(nonce, base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &payload.ciphertext,
                ).map_err(|e| CoreError::internal(format!("Failed to decode ciphertext: {e}")))?.as_ref())
                .map_err(|_| CoreError::internal("Failed to decrypt secrets file (key may have changed or file is corrupted)"))?;
            let map: std::collections::HashMap<String, String> =
                serde_json::from_slice(&plaintext)
                    .map_err(|e| CoreError::internal(format!("Failed to parse decrypted secrets: {e}")))?;
            map
        } else {
            std::collections::HashMap::new()
        };

        Ok(Self {
            secrets_file,
            key_file,
            secrets: Mutex::new(secrets),
            dirty: Mutex::new(false),
        })
    }

    fn flush(&self) -> CoreResult<()> {
        let mut dirty = self.dirty.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire dirty lock: {e}"))
        })?;
        if !*dirty {
            return Ok(());
        }

        let key_data = std::fs::read_to_string(&self.key_file).map_err(|e| {
            CoreError::internal(format!("Failed to read vault key file: {e}"))
        })?;
        let key_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            key_data.trim(),
        )
        .map_err(|e| CoreError::internal(format!("Failed to decode vault key: {e}")))?;
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| CoreError::internal(format!("Invalid AES key: {e}")))?;

        let secrets = self.secrets.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire secrets lock: {e}"))
        })?;
        let plaintext = serde_json::to_vec(&*secrets)
            .map_err(|e| CoreError::internal(format!("Failed to serialize secrets: {e}")))?;

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| CoreError::internal(format!("Failed to encrypt secrets: {e}")))?;

        let payload = EncryptedPayload {
            nonce: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                nonce_bytes,
            ),
            ciphertext: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                ciphertext,
            ),
        };

        let data = serde_json::to_string_pretty(&payload)
            .map_err(|e| CoreError::internal(format!("Failed to serialize payload: {e}")))?;
        std::fs::write(&self.secrets_file, data).map_err(|e| {
            CoreError::internal(format!("Failed to write secrets file: {e}"))
        })?;

        *dirty = false;
        Ok(())
    }
}

impl super::SecretVault for EncryptedFileVault {
    fn store(&self, key: &str, value: &str) -> CoreResult<()> {
        let mut secrets = self.secrets.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire secrets lock: {e}"))
        })?;
        secrets.insert(key.to_string(), value.to_string());
        drop(secrets);

        let mut dirty = self.dirty.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire dirty lock: {e}"))
        })?;
        *dirty = true;
        self.flush()
    }

    fn get(&self, key: &str) -> CoreResult<String> {
        let secrets = self.secrets.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire secrets lock: {e}"))
        })?;
        secrets
            .get(key)
            .cloned()
            .ok_or_else(|| CoreError::not_found("secret", key))
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let mut secrets = self.secrets.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire secrets lock: {e}"))
        })?;
        secrets.remove(key);
        drop(secrets);

        let mut dirty = self.dirty.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire dirty lock: {e}"))
        })?;
        *dirty = true;
        self.flush()
    }

    fn exists(&self, key: &str) -> CoreResult<bool> {
        let secrets = self.secrets.lock().map_err(|e| {
            CoreError::internal(format!("Failed to acquire secrets lock: {e}"))
        })?;
        Ok(secrets.contains_key(key))
    }
}

impl Drop for EncryptedFileVault {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecretVault;
    use tempfile::TempDir;

    #[test]
    fn test_encrypted_vault_store_and_get() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        vault.store("test/key", "super-secret-value").unwrap();
        assert_eq!(vault.get("test/key").unwrap(), "super-secret-value");
    }

    #[test]
    fn test_encrypted_vault_get_nonexistent() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        let result = vault.get("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_vault_delete() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        vault.store("test/key", "value").unwrap();
        assert!(vault.exists("test/key").unwrap());
        vault.delete("test/key").unwrap();
        assert!(!vault.exists("test/key").unwrap());
    }

    #[test]
    fn test_encrypted_vault_persistence_across_reload() {
        let dir = TempDir::new().unwrap();
        {
            let vault = EncryptedFileVault::new(dir.path()).unwrap();
            vault.store("persist/key", "persistent-value").unwrap();
        }
        // Reload vault
        {
            let vault = EncryptedFileVault::new(dir.path()).unwrap();
            assert_eq!(vault.get("persist/key").unwrap(), "persistent-value");
        }
    }

    #[test]
    fn test_encrypted_vault_empty_after_creation() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        assert!(!vault.exists("anything").unwrap());
    }

    #[test]
    fn test_encrypted_vault_overwrite() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        vault.store("test/key", "original").unwrap();
        vault.store("test/key", "updated").unwrap();
        assert_eq!(vault.get("test/key").unwrap(), "updated");
    }

    #[test]
    fn test_encrypted_vault_multiple_keys() {
        let dir = TempDir::new().unwrap();
        let vault = EncryptedFileVault::new(dir.path()).unwrap();
        vault.store("a", "1").unwrap();
        vault.store("b", "2").unwrap();
        vault.store("c", "3").unwrap();
        assert_eq!(vault.get("a").unwrap(), "1");
        assert_eq!(vault.get("b").unwrap(), "2");
        assert_eq!(vault.get("c").unwrap(), "3");
    }
}

