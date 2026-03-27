use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SshProfile {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AppSettings {
    pub theme: String,
    pub layout_mode: String,
    pub lock_enabled: bool,
    pub terminal_bg_color: Option<String>,
    #[serde(default)]
    pub terminal_fg_color: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "default".to_string(),
            layout_mode: "sidebar".to_string(),
            lock_enabled: false,
            terminal_bg_color: None,
            terminal_fg_color: None,
        }
    }
}

