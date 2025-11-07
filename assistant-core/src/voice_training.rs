use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple voice profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    pub user_id: String,
    pub name: String,
    pub created_at: u64,
}

/// Simple voice training manager
pub struct VoiceTrainingManager {
    profiles: HashMap<String, VoiceProfile>,
    storage_path: String,
}

impl VoiceTrainingManager {
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Self {
        Self {
            profiles: HashMap::new(),
            storage_path: storage_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent).await?;
        }
        info!("Voice training manager initialized");
        Ok(())
    }

    pub fn get_profile(&self, user_id: &str) -> Option<&VoiceProfile> {
        self.profiles.get(user_id)
    }

    pub async fn create_profile(&mut self, user_id: String, name: String) -> Result<(), Box<dyn std::error::Error>> {
        let profile = VoiceProfile {
            user_id: user_id.clone(),
            name,
            created_at: current_timestamp(),
        };
        self.profiles.insert(user_id, profile);
        debug!("Created voice profile");
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}