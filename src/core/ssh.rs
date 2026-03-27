use vte4 as vte;
use vte::prelude::*;
use thiserror::Error;
use crate::models::SshProfile;

#[derive(Error, Debug)]
pub enum SshError {
    #[error("Invalid hostname: {0}")]
    InvalidHostname(String),
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
    #[error("Failed to build arguments")]
    ArgumentError,
}

pub struct SshSession {
    pub profile: SshProfile,
}

impl SshSession {
    pub fn new(profile: SshProfile) -> Self {
        Self { profile }
    }

    pub fn is_valid_hostname(host: &str) -> bool {
        if host.is_empty() || host.starts_with('-') || host.len() > 255 {
            return false;
        }
        // RFC 1123 allowed characters: letters, digits, hyphen, dot. 
        // We also allow ':' for IPv6 addresses.
        host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':')
    }

    pub fn is_valid_username(user: &str) -> bool {
        if user.is_empty() || user.starts_with('-') || user.len() > 32 {
            return false;
        }
        // POSIX username standards: lowercase, digits, underscore, dot, hyphen.
        user.chars().all(|c| c.is_lowercase() || c.is_uppercase() || c.is_numeric() || c == '.' || c == '-' || c == '_')
    }

    pub fn get_arguments(&self) -> Result<Vec<String>, SshError> {
        if !Self::is_valid_hostname(&self.profile.host) {
            return Err(SshError::InvalidHostname(self.profile.host.clone()));
        }
        if !Self::is_valid_username(&self.profile.username) {
            return Err(SshError::InvalidUsername(self.profile.username.clone()));
        }

        let mut args = vec![
            "ssh".to_string(),
            "-p".to_string(),
            self.profile.port.to_string(),
        ];

        if let Some(ref id_file) = self.profile.identity_file {
            args.push("-i".to_string());
            // SAFETY: In this app, identity_file usually comes from a file picker, 
            // but we'll convert it to a string lossily. 
            args.push(id_file.to_string_lossy().to_string());
        }

        let host_spec = format!("{}@{}", self.profile.username, self.profile.host);
        args.push(host_spec);

        Ok(args)
    }
}

pub fn spawn_ssh_in_terminal(terminal: &vte::Terminal, profile: &SshProfile) {
    let session = SshSession::new(profile.clone());
    let args = match session.get_arguments() {
        Ok(a) => a,
        Err(e) => {
            log::error!("Security error: {}", e);
            return;
        }
    };
    
    let argv_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    terminal.spawn_async(
        vte::PtyFlags::DEFAULT,
        None::<&str>,
        &argv_refs,
        &[],
        glib::SpawnFlags::SEARCH_PATH,
        || {},
        -1,
        None::<&gio::Cancellable>,
        move |result| {
            if let Err(e) = result {
                log::error!("Failed to spawn SSH process: {}", e);
            }
        }
    );
}
