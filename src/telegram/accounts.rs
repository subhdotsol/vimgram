use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Represents a single Telegram account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,      // Unique ID like "account_1"
    pub phone: String,   // Phone number for display
    pub name: String,    // User-given name like "Personal"
}

/// Registry of all accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRegistry {
    pub active: String,           // Currently active account ID
    pub accounts: Vec<Account>,   // All accounts
}

impl Default for AccountRegistry {
    fn default() -> Self {
        Self {
            active: String::new(),
            accounts: Vec::new(),
        }
    }
}

fn get_config_dir() -> Option<PathBuf> {
    ProjectDirs::from("", "", "vimgram").map(|p| p.config_dir().to_path_buf())
}

fn get_accounts_path() -> PathBuf {
    get_config_dir()
        .map(|d| d.join("accounts.json"))
        .unwrap_or_else(|| PathBuf::from("accounts.json"))
}

fn get_sessions_dir() -> PathBuf {
    get_config_dir()
        .map(|d| d.join("sessions"))
        .unwrap_or_else(|| PathBuf::from("sessions"))
}

/// Get the session file path for a specific account
pub fn get_session_path_for_account(account_id: &str) -> PathBuf {
    get_sessions_dir().join(format!("{}.dat", account_id))
}

impl AccountRegistry {
    /// Load the account registry from disk
    pub fn load() -> Self {
        let path = get_accounts_path();
        if path.exists() {
            if let Ok(file) = fs::File::open(&path) {
                if let Ok(registry) = serde_json::from_reader(file) {
                    return registry;
                }
            }
        }
        
        // Check for legacy session migration
        Self::migrate_legacy_session()
    }
    
    /// Migrate legacy single-session setup to multi-account
    fn migrate_legacy_session() -> Self {
        let legacy_session = get_config_dir()
            .map(|d| d.join("session.dat"))
            .unwrap_or_else(|| PathBuf::from("session.dat"));
        
        if legacy_session.exists() {
            // Create sessions directory
            let sessions_dir = get_sessions_dir();
            let _ = fs::create_dir_all(&sessions_dir);
            
            // Move legacy session to default account
            let new_session_path = sessions_dir.join("default.dat");
            if fs::rename(&legacy_session, &new_session_path).is_ok() {
                // Create registry with migrated account
                let registry = AccountRegistry {
                    active: "default".to_string(),
                    accounts: vec![Account {
                        id: "default".to_string(),
                        phone: "Migrated".to_string(),
                        name: "Default".to_string(),
                    }],
                };
                let _ = registry.save();
                return registry;
            }
        }
        
        // No accounts yet
        AccountRegistry::default()
    }
    
    /// Save the account registry to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = get_accounts_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
    
    /// Get the currently active account
    pub fn get_active_account(&self) -> Option<&Account> {
        self.accounts.iter().find(|a| a.id == self.active)
    }
    
    /// Add a new account
    pub fn add_account(&mut self, phone: String, name: String) -> String {
        let id = format!("account_{}", self.accounts.len() + 1);
        self.accounts.push(Account {
            id: id.clone(),
            phone,
            name,
        });
        
        // If this is the first account, make it active
        if self.active.is_empty() {
            self.active = id.clone();
        }
        
        id
    }
    
    /// Set the active account
    pub fn set_active(&mut self, account_id: &str) {
        if self.accounts.iter().any(|a| a.id == account_id) {
            self.active = account_id.to_string();
        }
    }
    
    /// Check if there are any accounts
    pub fn has_accounts(&self) -> bool {
        !self.accounts.is_empty()
    }
    
    /// Get account by index
    pub fn get_account_by_index(&self, index: usize) -> Option<&Account> {
        self.accounts.get(index)
    }
    
    /// Delete an account's session file
    pub fn delete_account_session(account_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let session_path = get_session_path_for_account(account_id);
        if session_path.exists() {
            fs::remove_file(&session_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
