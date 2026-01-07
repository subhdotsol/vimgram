use grammers_client::{Client, Config, InitParams};
use grammers_session::Session;
use std::path::PathBuf;
use directories::ProjectDirs;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub api_id: i32,
    pub api_hash: String,
}

fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "vimgram").map(|p| p.config_dir().to_path_buf())
}

fn get_session_path() -> PathBuf {
    get_config_dir()
        .map(|d| d.join("session.dat"))
        .unwrap_or_else(|| PathBuf::from(".bifrost_session"))
}

fn get_credentials_path() -> PathBuf {
    get_config_dir()
        .map(|d| d.join("credentials.json"))
        .unwrap_or_else(|| PathBuf::from("credentials.json"))
}

impl Credentials {
    pub fn load() -> Option<Self> {
        let path = get_credentials_path();
        if path.exists() {
            let file = fs::File::open(path).ok()?;
            serde_json::from_reader(file).ok()
        } else {
            None
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_credentials_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

pub struct TelegramClient {
    pub client: Client,
    pub account_id: Option<String>,
}

impl TelegramClient {
    /// Connect with legacy session (for backward compatibility)
    pub async fn connect(api_id: i32, api_hash: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let session_path = get_session_path();
        let session = if session_path.exists() {
            Session::load_file(&session_path)?
        } else {
            Session::new()
        };

        let client = Client::connect(Config {
            session,
            api_id,
            api_hash: api_hash.to_string(),
            params: InitParams {
                ..Default::default()
            },
        })
        .await?;

        Ok(Self { client, account_id: None })
    }
    
    /// Connect with a specific account
    pub async fn connect_with_account(api_id: i32, api_hash: &str, account_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        use super::accounts::get_session_path_for_account;
        
        let session_path = get_session_path_for_account(account_id);
        
        // Ensure sessions directory exists
        if let Some(parent) = session_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let session = if session_path.exists() {
            Session::load_file(&session_path)?
        } else {
            Session::new()
        };

        let client = Client::connect(Config {
            session,
            api_id,
            api_hash: api_hash.to_string(),
            params: InitParams {
                ..Default::default()
            },
        })
        .await?;

        Ok(Self { client, account_id: Some(account_id.to_string()) })
    }

    /// Save session (uses account_id if set)
    pub fn save_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        use super::accounts::get_session_path_for_account;
        
        let data = self.client.session().save();
        let session_path = if let Some(ref account_id) = self.account_id {
            get_session_path_for_account(account_id)
        } else {
            get_session_path()
        };
        
        // Ensure parent exists
        if let Some(parent) = session_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(session_path, data)?;
        Ok(())
    }

    pub async fn is_authorized(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.client.is_authorized().await?)
    }
}

/// Delete the session file for the active account
pub fn delete_session() -> Result<bool, Box<dyn std::error::Error>> {
    let session_path = get_session_path();
    if session_path.exists() {
        fs::remove_file(&session_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Delete session for a specific account
pub fn delete_session_for_account(account_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use super::accounts::get_session_path_for_account;
    
    let session_path = get_session_path_for_account(account_id);
    if session_path.exists() {
        fs::remove_file(&session_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Delete credentials file
pub fn delete_credentials() -> Result<bool, Box<dyn std::error::Error>> {
    let creds_path = get_credentials_path();
    if creds_path.exists() {
        fs::remove_file(&creds_path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
