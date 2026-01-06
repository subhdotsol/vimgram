use grammers_client::{Client, Config, InitParams};
use grammers_session::Session;
use std::path::PathBuf;
use directories::ProjectDirs;
use std::fs;

fn get_session_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "vimgram") {
        let config_dir = proj_dirs.config_dir();
        // Ensure directory exists
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        config_dir.join("session.dat")
    } else {
        PathBuf::from(".bifrost_session") // Fallback
    }
}

pub struct TelegramClient {
    pub client: Client,
}

impl TelegramClient {
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

        Ok(Self { client })
    }

    pub fn save_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.client.session().save();
        let session_path = get_session_path();
        // Ensure parent exists again just in case
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
