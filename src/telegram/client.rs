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
