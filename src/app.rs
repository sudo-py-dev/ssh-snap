use crate::core::storage::Storage;
use crate::models::{AppSettings, SshProfile};
use anyhow::{Context, Result};
use std::sync::{Arc, RwLock};
use std::path::PathBuf;

pub struct AppData {
    pub profiles: Vec<SshProfile>,
    pub settings: AppSettings,
}

pub struct AppState {
    pub data: Arc<RwLock<AppData>>,
    pub storage: Arc<Storage>,
}

impl AppState {
    pub fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let storage = Arc::new(Storage::new(custom_path).context("Failed to initialize storage")?);
        
        // Load data, but don't silently ignore corruption errors unless they are "not found"
        let profiles = storage.load_profiles().context("Failed to load profiles")?;
        let settings = storage.load_settings().context("Failed to load settings")?;

        let data = Arc::new(RwLock::new(AppData { profiles, settings }));

        Ok(Self { data, storage })
    }

    pub fn save_profiles(&self) -> Result<()> {
        let profiles = self.data.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?.profiles.clone();
        self.storage.save_profiles(&profiles).context("Failed to persist profiles to disk")
    }

    pub fn save_settings(&self) -> Result<()> {
        let settings = self.data.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?.settings.clone();
        self.storage.save_settings(&settings).context("Failed to persist settings")
    }

    pub fn add_profile(&self, profile: SshProfile) -> Result<()> {
        {
            let mut data = self.data.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            if data.profiles.iter().any(|p| p.id == profile.id) {
                return Err(anyhow::anyhow!("Profile with ID {} already exists", profile.id));
            }
            data.profiles.push(profile);
        }
        self.save_profiles()
    }

    pub fn update_profile(&self, profile: SshProfile) -> Result<()> {
        {
            let mut data = self.data.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            if let Some(pos) = data.profiles.iter().position(|p| p.id == profile.id) {
                data.profiles[pos] = profile;
            } else {
                return Err(anyhow::anyhow!("Profile not found: {}", profile.id));
            }
        }
        self.save_profiles()
    }

    pub fn delete_profile(&self, id: &uuid::Uuid) -> Result<()> {
        {
            let mut data = self.data.write().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
            data.profiles.retain(|p| p.id != *id);
        }
        // Handle deletion errors explicitly
        if let Err(e) = self.storage.delete_password(&id.to_string()) {
            log::warn!("Failed to delete password for profile {}: {}", id, e);
        }
        self.save_profiles()
    }

    pub fn get_profile_at(&self, index: usize) -> Option<SshProfile> {
        self.data.read().ok()?.profiles.get(index).cloned()
    }
}