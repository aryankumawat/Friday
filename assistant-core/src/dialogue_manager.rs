use crate::EngineEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple dialogue session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueSession {
    pub session_id: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub turn_count: u32,
}

/// Simple dialogue intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogueIntent {
    Timer { duration: Option<u64>, label: Option<String> },
    Unknown,
}

/// Simple dialogue manager
pub struct DialogueManager {
    sessions: HashMap<String, DialogueSession>,
    session_timeout_ms: u64,
}

impl DialogueManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_timeout_ms: 300_000, // 5 minutes
        }
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Dialogue manager initialized");
        Ok(())
    }

    pub async fn process_user_input(
        &mut self,
        session_id: String,
        user_input: String,
        _events: mpsc::Sender<EngineEvent>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let session = self.get_or_create_session(session_id);
        session.turn_count += 1;
        session.last_activity = current_timestamp();
        
        let intent = self.parse_intent(&user_input).await;
        
        match intent {
            DialogueIntent::Timer { duration, label } => {
                let duration = duration.unwrap_or(300); // Default 5 minutes
                let default_label = "Timer".to_string();
                let label = label.as_ref().unwrap_or(&default_label);
                Ok(format!("Timer set for {} seconds: {}", duration, label))
            }
            DialogueIntent::Unknown => {
                Ok("I didn't understand that. Can you try again?".to_string())
            }
        }
    }

    fn get_or_create_session(&mut self, session_id: String) -> &mut DialogueSession {
        self.sessions.entry(session_id.clone()).or_insert_with(|| {
            DialogueSession {
                session_id,
                created_at: current_timestamp(),
                last_activity: current_timestamp(),
                turn_count: 0,
            }
        })
    }

    async fn parse_intent(&self, text: &str) -> DialogueIntent {
        let lower = text.to_lowercase();
        
        if lower.contains("timer") {
            // Simple duration extraction
            let duration = if lower.contains("minute") {
                Some(300) // 5 minutes default
            } else if lower.contains("second") {
                Some(10) // 10 seconds default
            } else {
                None
            };
            
            DialogueIntent::Timer { duration, label: None }
        } else {
            DialogueIntent::Unknown
        }
    }

    pub async fn cleanup_expired_sessions(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = current_timestamp();
        let expired_sessions: Vec<String> = self.sessions
            .iter()
            .filter(|(_, session)| now - session.last_activity > self.session_timeout_ms)
            .map(|(id, _)| id.clone())
            .collect();
        
        for session_id in expired_sessions {
            self.sessions.remove(&session_id);
            debug!("Cleaned up expired session: {}", session_id);
        }
        
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}