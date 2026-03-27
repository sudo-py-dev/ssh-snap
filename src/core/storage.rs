use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce
};
use argon2::{
    Argon2, 
    password_hash::SaltString
};
use rand::rngs::OsRng;
use rand::RngCore;
use secrecy::{Secret, SecretString, ExposeSecret};
use thiserror::Error;
use crate::models::{SshProfile, AppSettings};
use keyring::Entry;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("Security error: {0}")]
    Security(String),
    #[error("Lock poisoned")]
    Poisoned,
}

pub struct Storage {
    pub config_dir: PathBuf,
    pub encryption_key: Arc<RwLock<Option<Secret<[u8; 32]>>>>,
}

impl Storage {
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self, StorageError> {
        let config_dir = custom_path.unwrap_or_else(|| {
            directories::ProjectDirs::from("com", "sudopydev", "ssh-snap")
                .map(|p| p.config_dir().to_path_buf())
                .unwrap_or_else(|| PathBuf::from(".config/ssh-snap"))
        });

        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(Self {
            config_dir,
            encryption_key: Arc::new(RwLock::new(None)),
        })
    }

    fn get_salt(&self) -> Result<SaltString, StorageError> {
        let salt_path = self.config_dir.join(".salt");
        if salt_path.exists() {
            let s = fs::read_to_string(&salt_path)?;
            SaltString::from_b64(&s).map_err(|e| StorageError::Security(e.to_string()))
        } else {
            let salt = SaltString::generate(&mut OsRng);
            fs::write(&salt_path, salt.as_str())?;
            Ok(salt)
        }
    }

    pub fn setup_encryption(&self, password: &SecretString) -> Result<(), StorageError> {
        let salt = self.get_salt()?;
        let argon2 = Argon2::default();
        let mut key_bytes = [0u8; 32];
        argon2.hash_password_into(password.expose_secret().as_bytes(), salt.as_str().as_bytes(), &mut key_bytes)
            .map_err(|e| StorageError::Security(format!("KDF error: {}", e)))?;
        
        let mut k_lock = self.encryption_key.write().map_err(|_| StorageError::Poisoned)?;
        *k_lock = Some(Secret::new(key_bytes));
        Ok(())
    }

    pub fn write_secure_file(&self, path: &Path, content: &str) -> Result<(), StorageError> {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config_dir.join(path)
        };

        fs::write(&absolute_path, content)?;
        
        #[cfg(unix)]
        {
            if let Ok(metadata) = fs::metadata(&absolute_path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(&absolute_path, perms);
            }
        }
        Ok(())
    }

    pub fn save_profiles(&self, profiles: &[SshProfile]) -> Result<(), StorageError> {
        let path = self.config_dir.join("profiles.json");
        let json = serde_json::to_string_pretty(profiles)?;
        
        let settings = self.load_settings().unwrap_or_default();
        if settings.lock_enabled {
            let key_opt = self.encryption_key.read().map_err(|_| StorageError::Poisoned)?;
            if let Some(key) = key_opt.as_ref() {
                let cipher = {
                    let k: &[u8; 32] = key.expose_secret();
                    Aes256Gcm::new_from_slice(k).map_err(|e| StorageError::Encryption(e.to_string()))?
                };
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                let ciphertext = cipher.encrypt(nonce, json.as_bytes())
                    .map_err(|e| StorageError::Encryption(e.to_string()))?;
                
                let mut combined = nonce_bytes.to_vec();
                combined.extend_from_slice(&ciphertext);
                fs::write(&path, combined)?;
                return Ok(());
            }
        }

        self.write_secure_file(&path, &json)
    }

    pub fn load_profiles(&self) -> Result<Vec<SshProfile>, StorageError> {
        let path = self.config_dir.join("profiles.json");
        if !path.exists() { return Ok(vec![]); }
        
        let data = fs::read(&path)?;
        
        // Attempt plain JSON first
        if let Ok(p) = serde_json::from_slice::<Vec<SshProfile>>(&data) {
            return Ok(p);
        }

        // Attempt decryption
        let key_opt = self.encryption_key.read().map_err(|_| StorageError::Poisoned)?;
        if let Some(key) = key_opt.as_ref() {
            if data.len() < 12 { return Err(StorageError::Decryption("Encrypted data too short".to_string())); }
            let (nonce_bytes, ciphertext) = data.split_at(12);
            let nonce = Nonce::from_slice(nonce_bytes);
            let cipher = {
                let k: &[u8; 32] = key.expose_secret();
                Aes256Gcm::new_from_slice(k).map_err(|e| StorageError::Decryption(e.to_string()))?
            };
            
            let decrypted = cipher.decrypt(nonce, ciphertext)
                .map_err(|e| StorageError::Decryption(format!("Decryption failure: {}", e)))?;
            
            let profiles = serde_json::from_slice(&decrypted)?;
            Ok(profiles)
        } else {
            Ok(vec![])
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let path = self.config_dir.join("settings.json");
        let json = serde_json::to_string_pretty(settings)?;
        self.write_secure_file(&path, &json)
    }

    pub fn load_settings(&self) -> Result<AppSettings, StorageError> {
        let path = self.config_dir.join("settings.json");
        if !path.exists() { return Ok(AppSettings::default()); }

        let json = fs::read_to_string(&path)?;
        match serde_json::from_str::<AppSettings>(&json) {
            Ok(s) => Ok(s),
            Err(e) => {
                log::error!("Corrupt settings.json: {}. Returning defaults.", e);
                Ok(AppSettings::default())
            }
        }
    }

    pub fn save_password(&self, profile_id: &str, password: &SecretString) -> Result<(), StorageError> {
        let entry = Entry::new("ssh-snap", profile_id)?;
        entry.set_password(password.expose_secret())?;
        Ok(())
    }

    pub fn get_password(&self, profile_id: &str) -> Result<Option<SecretString>, StorageError> {
        let entry = Entry::new("ssh-snap", profile_id)?;
        match entry.get_password() {
            Ok(p) => Ok(Some(SecretString::new(p))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(StorageError::Keyring(e)),
        }
    }

    pub fn delete_password(&self, profile_id: &str) -> Result<(), StorageError> {
        let entry = Entry::new("ssh-snap", profile_id)?;
        match entry.delete_credential() {
            Ok(_) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(StorageError::Keyring(e)),
        }
    }

    pub fn verify_system_password(&self, user: &str, password: &SecretString) -> bool {
        let mut auth = match pam::Authenticator::with_password("login") {
            Ok(a) => a,
            Err(_) => return false,
        };
        auth.get_handler().set_credentials(user, password.expose_secret());
        auth.authenticate().is_ok()
    }
}
