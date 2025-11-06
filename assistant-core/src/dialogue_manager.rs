use crate::{Intent, EngineEvent, EngineError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};

/// Dialogue state for multi-turn conversations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueState {
    pub session_id: String,
    pub context: DialogueContext,
    pub active_intent: Option<ActiveIntent>,
    pub slot_filling: HashMap<String, SlotValue>,
    pub conversation_history: Vec<Turn>,
    pub created_at: u64,
    pub last_activity: u64,
}

/// Dialogue context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueContext {
    pub user_preferences: HashMap<String, String>,
    pub location: Option<String>,
    pub timezone: Option<String>,
    pub language: String,
    pub previous_topics: Vec<String>,
}

/// Active intent being processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveIntent {
    pub intent_type: String,
    pub confidence: f32,
    pub required_slots: Vec<String>,
    pub optional_slots: Vec<String>,
    pub filled_slots: HashMap<String, SlotValue>,
    pub missing_slots: Vec<String>,
}

/// Slot value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotValue {
    pub value: String,
    pub confidence: f32,
    pub source: SlotSource,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlotSource {
    UserInput,
    Context,
    Default,
    Inferred,
}

/// Conversation turn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub turn_id: u64,
    pub user_input: String,
    pub intent: Option<String>,
    pub response: String,
    pub timestamp: u64,
    pub success: bool,
}

/// Dialogue manager for multi-turn conversations
pub struct DialogueManager {
    sessions: HashMap<String, DialogueState>,
    session_timeout_ms: u64,
    max_sessions: usize,
    slot_definitions: HashMap<String, SlotDefinition>,
}

#[derive(Debug, Clone)]
pub struct SlotDefinition {
    pub name: String,
    pub slot_type: SlotType,
    pub required: bool,
    pub validation_pattern: Option<String>,
    pub default_value: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub enum SlotType {
    String,
    Number,
    Duration,
    Location,
    DateTime,
    Boolean,
    Choice(Vec<String>),
}

impl DialogueManager {
    pub fn new() -> Self {
        let mut manager = Self {
            sessions: HashMap::new(),
            session_timeout_ms: 30 * 60 * 1000, // 30 minutes
            max_sessions: 100,
            slot_definitions: HashMap::new(),
        };
        
        manager.initialize_slot_definitions();
        manager
    }

    pub fn with_session_timeout(mut self, timeout_ms: u64) -> Self {
        self.session_timeout_ms = timeout_ms;
        self
    }

    pub fn with_max_sessions(mut self, max_sessions: usize) -> Self {
        self.max_sessions = max_sessions;
        self
    }

    fn initialize_slot_definitions(&mut self) {
        // Timer intent slots
        self.add_slot_definition(SlotDefinition {
            name: "duration".to_string(),
            slot_type: SlotType::Duration,
            required: true,
            validation_pattern: Some(r"\d+\s*(second|minute|hour)s?".to_string()),
            default_value: Some("5 minutes".to_string()),
            prompt: "How long should the timer be?".to_string(),
        });

        self.add_slot_definition(SlotDefinition {
            name: "timer_label".to_string(),
            slot_type: SlotType::String,
            required: false,
            validation_pattern: None,
            default_value: None,
            prompt: "What would you like to call this timer?".to_string(),
        });

        // Weather intent slots
        self.add_slot_definition(SlotDefinition {
            name: "location".to_string(),
            slot_type: SlotType::Location,
            required: false,
            validation_pattern: None,
            default_value: None,
            prompt: "Which location would you like the weather for?".to_string(),
        });

        // App launch slots
        self.add_slot_definition(SlotDefinition {
            name: "app_name".to_string(),
            slot_type: SlotType::String,
            required: true,
            validation_pattern: None,
            default_value: None,
            prompt: "Which application would you like to open?".to_string(),
        });
    }

    fn add_slot_definition(&mut self, slot_def: SlotDefinition) {
        self.slot_definitions.insert(slot_def.name.clone(), slot_def);
    }

    /// Get or create dialogue session
    pub fn get_or_create_session(&mut self, session_id: &str) -> &mut DialogueState {
        let now = current_timestamp();
        
        // Clean up expired sessions
        self.cleanup_expired_sessions(now);
        
        // Create new session if it doesn't exist
        if !self.sessions.contains_key(session_id) {
            let session = DialogueState {
                session_id: session_id.to_string(),
                context: DialogueContext {
                    user_preferences: HashMap::new(),
                    location: None,
                    timezone: None,
                    language: "en".to_string(),
                    previous_topics: Vec::new(),
                },
                active_intent: None,
                slot_filling: HashMap::new(),
                conversation_history: Vec::new(),
                created_at: now,
                last_activity: now,
            };
            
            self.sessions.insert(session_id.to_string(), session);
            info!("Created new dialogue session: {}", session_id);
        }
        
        // Update last activity
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.last_activity = now;
        }
        
        self.sessions.get_mut(session_id).unwrap()
    }

    /// Process user input and manage dialogue flow
    pub async fn process_turn(
        &mut self,
        session_id: &str,
        user_input: &str,
        intent: &Intent,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<DialogueResponse, EngineError> {
        let session = self.get_or_create_session(session_id);
        let turn_id = session.conversation_history.len() as u64 + 1;
        
        debug!("Processing dialogue turn {} for session {}", turn_id, session_id);
        
        // Check if we're in the middle of slot filling
        if let Some(ref mut active_intent) = session.active_intent {
            return self.handle_slot_filling(session, user_input, events).await;
        }
        
        // Start new intent processing
        match intent {
            Intent::Timer { duration_secs } => {
                self.handle_timer_intent(session, user_input, *duration_secs, events).await
            }
            Intent::Unknown { text } => {
                self.handle_unknown_intent(session, user_input, text, events).await
            }
        }
    }

    async fn handle_timer_intent(
        &mut self,
        session: &mut DialogueState,
        user_input: &str,
        duration_secs: u64,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<DialogueResponse, EngineError> {
        // Extract timer label if present
        let timer_label = self.extract_timer_label(user_input);
        
        // Create active intent for potential slot filling
        let mut active_intent = ActiveIntent {
            intent_type: "timer".to_string(),
            confidence: 0.9,
            required_slots: vec!["duration".to_string()],
            optional_slots: vec!["timer_label".to_string()],
            filled_slots: HashMap::new(),
            missing_slots: Vec::new(),
        };
        
        // Fill duration slot
        active_intent.filled_slots.insert("duration".to_string(), SlotValue {
            value: format!("{} seconds", duration_secs),
            confidence: 0.9,
            source: SlotSource::UserInput,
            timestamp: current_timestamp(),
        });
        
        // Fill label slot if present
        if let Some(label) = timer_label {
            active_intent.filled_slots.insert("timer_label".to_string(), SlotValue {
                value: label,
                confidence: 0.8,
                source: SlotSource::UserInput,
                timestamp: current_timestamp(),
            });
        }
        
        // Check if all required slots are filled
        let missing_slots = self.check_missing_slots(&active_intent);
        
        if missing_slots.is_empty() {
            // All slots filled, execute intent
            session.active_intent = None;
            let response = format!("Timer set for {} seconds", duration_secs);
            
            self.add_turn_to_history(session, user_input, Some("timer"), &response, true);
            
            Ok(DialogueResponse {
                response,
                needs_more_input: false,
                prompt: None,
                session_complete: false,
            })
        } else {
            // Need more information
            active_intent.missing_slots = missing_slots.clone();
            session.active_intent = Some(active_intent);
            
            let prompt = self.generate_slot_prompt(&missing_slots[0]);
            
            Ok(DialogueResponse {
                response: prompt.clone(),
                needs_more_input: true,
                prompt: Some(prompt),
                session_complete: false,
            })
        }
    }

    async fn handle_unknown_intent(
        &mut self,
        session: &mut DialogueState,
        user_input: &str,
        text: &str,
        _events: mpsc::Sender<EngineEvent>,
    ) -> Result<DialogueResponse, EngineError> {
        // Check if this might be a follow-up question
        if self.is_followup_question(text) {
            let response = self.handle_followup(session, text);
            self.add_turn_to_history(session, user_input, None, &response, true);
            
            return Ok(DialogueResponse {
                response,
                needs_more_input: false,
                prompt: None,
                session_complete: false,
            });
        }
        
        // Check conversation history for context
        let response = if session.conversation_history.is_empty() {
            "Hello! I'm Friday, your voice assistant. I can help you with timers, weather, opening apps, and more. What would you like to do?".to_string()
        } else {
            "I'm not sure how to help with that. You can ask me to set timers, check the weather, open applications, or control system volume.".to_string()
        };
        
        self.add_turn_to_history(session, user_input, None, &response, true);
        
        Ok(DialogueResponse {
            response,
            needs_more_input: false,
            prompt: None,
            session_complete: false,
        })
    }

    async fn handle_slot_filling(
        &mut self,
        session: &mut DialogueState,
        user_input: &str,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<DialogueResponse, EngineError> {
        let active_intent = session.active_intent.as_mut().unwrap();
        let missing_slot = &active_intent.missing_slots[0].clone();
        
        // Extract slot value from user input
        if let Some(slot_value) = self.extract_slot_value(missing_slot, user_input) {
            active_intent.filled_slots.insert(missing_slot.clone(), slot_value);
            active_intent.missing_slots.remove(0);
            
            // Check if more slots are needed
            if active_intent.missing_slots.is_empty() {
                // All slots filled, execute intent
                let response = self.execute_filled_intent(session, events).await?;
                session.active_intent = None;
                
                self.add_turn_to_history(session, user_input, Some(&active_intent.intent_type), &response, true);
                
                Ok(DialogueResponse {
                    response,
                    needs_more_input: false,
                    prompt: None,
                    session_complete: false,
                })
            } else {
                // Need more slots
                let next_slot = &active_intent.missing_slots[0];
                let prompt = self.generate_slot_prompt(next_slot);
                
                Ok(DialogueResponse {
                    response: prompt.clone(),
                    needs_more_input: true,
                    prompt: Some(prompt),
                    session_complete: false,
                })
            }
        } else {
            // Couldn't extract slot value
            let prompt = format!("I didn't understand that. {}", self.generate_slot_prompt(missing_slot));
            
            Ok(DialogueResponse {
                response: prompt.clone(),
                needs_more_input: true,
                prompt: Some(prompt),
                session_complete: false,
            })
        }
    }

    fn extract_timer_label(&self, text: &str) -> Option<String> {
        // Simple extraction - look for "for X" or "called X"
        let patterns = [
            r"(?i)for\s+(.+?)(?:\s+timer)?$",
            r"(?i)called\s+(.+?)(?:\s+timer)?$",
            r"(?i)named\s+(.+?)(?:\s+timer)?$",
        ];
        
        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(text) {
                    if let Some(label) = captures.get(1) {
                        return Some(label.as_str().trim().to_string());
                    }
                }
            }
        }
        
        None
    }

    fn check_missing_slots(&self, active_intent: &ActiveIntent) -> Vec<String> {
        let mut missing = Vec::new();
        
        for required_slot in &active_intent.required_slots {
            if !active_intent.filled_slots.contains_key(required_slot) {
                missing.push(required_slot.clone());
            }
        }
        
        missing
    }

    fn generate_slot_prompt(&self, slot_name: &str) -> String {
        if let Some(slot_def) = self.slot_definitions.get(slot_name) {
            slot_def.prompt.clone()
        } else {
            format!("Please provide a value for {}", slot_name)
        }
    }

    fn extract_slot_value(&self, slot_name: &str, text: &str) -> Option<SlotValue> {
        match slot_name {
            "duration" => self.extract_duration(text),
            "timer_label" => Some(SlotValue {
                value: text.trim().to_string(),
                confidence: 0.8,
                source: SlotSource::UserInput,
                timestamp: current_timestamp(),
            }),
            "location" => Some(SlotValue {
                value: text.trim().to_string(),
                confidence: 0.8,
                source: SlotSource::UserInput,
                timestamp: current_timestamp(),
            }),
            _ => None,
        }
    }

    fn extract_duration(&self, text: &str) -> Option<SlotValue> {
        // Extract duration from text like "5 minutes", "30 seconds", etc.
        if let Ok(re) = regex::Regex::new(r"(\d+)\s*(second|minute|hour)s?") {
            if let Some(captures) = re.captures(text) {
                if let (Some(num), Some(unit)) = (captures.get(1), captures.get(2)) {
                    let duration = format!("{} {}", num.as_str(), unit.as_str());
                    return Some(SlotValue {
                        value: duration,
                        confidence: 0.9,
                        source: SlotSource::UserInput,
                        timestamp: current_timestamp(),
                    });
                }
            }
        }
        None
    }

    async fn execute_filled_intent(
        &self,
        session: &DialogueState,
        _events: mpsc::Sender<EngineEvent>,
    ) -> Result<String, EngineError> {
        let active_intent = session.active_intent.as_ref().unwrap();
        
        match active_intent.intent_type.as_str() {
            "timer" => {
                let duration = active_intent.filled_slots.get("duration")
                    .map(|v| &v.value)
                    .unwrap_or(&"5 minutes".to_string());
                
                let label = active_intent.filled_slots.get("timer_label")
                    .map(|v| format!(" called '{}'", v.value))
                    .unwrap_or_default();
                
                Ok(format!("Timer set for {}{}", duration, label))
            }
            _ => Ok("Intent executed successfully".to_string()),
        }
    }

    fn is_followup_question(&self, text: &str) -> bool {
        let followup_patterns = [
            r"(?i)^(what|how|when|where|why|who)",
            r"(?i)^(can you|could you|will you)",
            r"(?i)^(tell me|show me)",
            r"(?i)(more|again|repeat)",
        ];
        
        for pattern in &followup_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if re.is_match(text) {
                    return true;
                }
            }
        }
        
        false
    }

    fn handle_followup(&self, session: &DialogueState, text: &str) -> String {
        // Simple followup handling based on conversation history
        if text.to_lowercase().contains("what") && text.to_lowercase().contains("can") {
            return "I can help you with setting timers, checking weather, opening applications, controlling system volume, and answering basic questions.".to_string();
        }
        
        if !session.conversation_history.is_empty() {
            let last_turn = &session.conversation_history[session.conversation_history.len() - 1];
            if let Some(ref intent) = last_turn.intent {
                return format!("That was about {}. Is there anything else I can help you with?", intent);
            }
        }
        
        "I'm not sure what you're referring to. Could you be more specific?".to_string()
    }

    fn add_turn_to_history(
        &self,
        session: &mut DialogueState,
        user_input: &str,
        intent: Option<&str>,
        response: &str,
        success: bool,
    ) {
        let turn = Turn {
            turn_id: session.conversation_history.len() as u64 + 1,
            user_input: user_input.to_string(),
            intent: intent.map(|s| s.to_string()),
            response: response.to_string(),
            timestamp: current_timestamp(),
            success,
        };
        
        session.conversation_history.push(turn);
        
        // Keep only last 10 turns to prevent memory bloat
        if session.conversation_history.len() > 10 {
            session.conversation_history.remove(0);
        }
    }

    fn cleanup_expired_sessions(&mut self, now: u64) {
        let expired_sessions: Vec<String> = self.sessions
            .iter()
            .filter(|(_, session)| now - session.last_activity > self.session_timeout_ms)
            .map(|(id, _)| id.clone())
            .collect();
        
        for session_id in expired_sessions {
            self.sessions.remove(&session_id);
            debug!("Removed expired session: {}", session_id);
        }
        
        // Enforce max sessions limit
        while self.sessions.len() > self.max_sessions {
            if let Some(oldest_session) = self.sessions
                .iter()
                .min_by_key(|(_, session)| session.last_activity)
                .map(|(id, _)| id.clone())
            {
                self.sessions.remove(&oldest_session);
                debug!("Removed oldest session to enforce limit: {}", oldest_session);
            }
        }
    }

    /// Get session statistics
    pub fn get_session_stats(&self) -> SessionStats {
        SessionStats {
            active_sessions: self.sessions.len(),
            total_turns: self.sessions.values().map(|s| s.conversation_history.len()).sum(),
            sessions_with_active_intent: self.sessions.values().filter(|s| s.active_intent.is_some()).count(),
        }
    }
}

/// Response from dialogue processing
#[derive(Debug, Clone)]
pub struct DialogueResponse {
    pub response: String,
    pub needs_more_input: bool,
    pub prompt: Option<String>,
    pub session_complete: bool,
}

/// Session statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub active_sessions: usize,
    pub total_turns: usize,
    pub sessions_with_active_intent: usize,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for DialogueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_dialogue_session_creation() {
        let mut manager = DialogueManager::new();
        let session = manager.get_or_create_session("test_session");
        
        assert_eq!(session.session_id, "test_session");
        assert!(session.conversation_history.is_empty());
        assert!(session.active_intent.is_none());
    }

    #[tokio::test]
    async fn test_timer_intent_processing() {
        let mut manager = DialogueManager::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let intent = Intent::Timer { duration_secs: 300 };
        let response = manager.process_turn("test", "set a timer for 5 minutes", &intent, tx).await.unwrap();
        
        assert!(!response.needs_more_input);
        assert!(response.response.contains("Timer set"));
    }

    #[tokio::test]
    async fn test_slot_filling() {
        let mut manager = DialogueManager::new();
        let (tx, _rx) = mpsc::channel(10);
        
        // Start with incomplete timer request
        let intent = Intent::Unknown { text: "set a timer".to_string() };
        let response = manager.process_turn("test", "set a timer", &intent, tx.clone()).await.unwrap();
        
        // Should ask for duration
        assert!(response.needs_more_input);
        assert!(response.prompt.is_some());
        
        // Provide duration
        let intent2 = Intent::Unknown { text: "5 minutes".to_string() };
        let response2 = manager.process_turn("test", "5 minutes", &intent2, tx).await.unwrap();
        
        // Should complete the intent
        assert!(!response2.needs_more_input);
    }

    #[test]
    fn test_duration_extraction() {
        let manager = DialogueManager::new();
        
        let duration = manager.extract_duration("5 minutes");
        assert!(duration.is_some());
        assert_eq!(duration.unwrap().value, "5 minute");
        
        let duration2 = manager.extract_duration("30 seconds");
        assert!(duration2.is_some());
        assert_eq!(duration2.unwrap().value, "30 second");
    }

    #[test]
    fn test_followup_detection() {
        let manager = DialogueManager::new();
        
        assert!(manager.is_followup_question("What can you do?"));
        assert!(manager.is_followup_question("Can you help me?"));
        assert!(manager.is_followup_question("Tell me more"));
        assert!(!manager.is_followup_question("Set a timer"));
    }
}