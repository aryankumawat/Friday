use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: u64,
}

/// Simple memory manager
pub struct MemoryManager {
    memories: HashMap<String, MemoryEntry>,
    storage_path: String,
}

impl MemoryManager {
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Self {
        Self {
            memories: HashMap::new(),
            storage_path: storage_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent).await?;
        }
        info!("Memory manager initialized");
        Ok(())
    }

    pub async fn store_memory(&mut self, key: String, value: serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        let entry = MemoryEntry {
            key: key.clone(),
            value,
            created_at: current_timestamp(),
        };
        self.memories.insert(key, entry);
        debug!("Stored memory entry");
        Ok(())
    }

    pub fn get_memory(&self, key: &str) -> Option<&MemoryEntry> {
        self.memories.get(key)
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}