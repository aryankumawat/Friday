use crate::{Intent, NluEngine, EngineError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Enhanced intent types beyond basic timer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnhancedIntent {
    Greeting { user_name: Option<String> },
    Timer { duration_secs: u64, label: Option<String> },
    Weather { location: Option<String> },
    AppLaunch { app_name: String },
    Query { question: String },
    SystemControl { action: SystemAction },
    Unknown { text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemAction {
    VolumeUp,
    VolumeDown,
    Mute,
    Unmute,
    Sleep,
    Shutdown,
    Restart,
}

/// Pattern-based intent recognition with confidence scoring
pub struct EnhancedNlu {
    patterns: HashMap<String, Vec<IntentPattern>>,
    confidence_threshold: f32,
}

#[derive(Debug, Clone)]
struct IntentPattern {
    pattern: regex::Regex,
    intent_type: String,
    confidence: f32,
    extractor: Option<fn(&str) -> HashMap<String, String>>,
}

impl EnhancedNlu {
    pub fn new() -> Self {
        let mut nlu = Self {
            patterns: HashMap::new(),
            confidence_threshold: 0.6,
        };
        nlu.initialize_patterns();
        nlu
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold;
        self
    }

    fn initialize_patterns(&mut self) {
        // Greeting patterns - FIRST so they match before others
        self.add_greeting_patterns();
        
        // Timer patterns
        self.add_timer_patterns();
        
        // Weather patterns
        self.add_weather_patterns();
        
        // App launch patterns
        self.add_app_launch_patterns();
        
        // System control patterns
        self.add_system_control_patterns();
        
        // Query patterns
        self.add_query_patterns();
    }

    fn add_greeting_patterns(&mut self) {
        let greeting_patterns = vec![
            (r"(?i)(?:hey|hi|hello|yo)\s+(?:friday|assistant)", 0.95),
            (r"(?i)(?:good\s+)?(?:morning|afternoon|evening)\s+friday", 0.9),
            (r"(?i)what'?s\s+up\s+friday", 0.9),
        ];

        for (pattern_str, confidence) in greeting_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("greeting", pattern, confidence, None);
            }
        }
    }

    fn add_timer_patterns(&mut self) {
        let timer_patterns = vec![
            (r"(?i)set\s+(?:a\s+)?timer\s+for\s+(\d+)\s+(second|minute|hour)s?", 0.9),
            (r"(?i)(?:remind|alert)\s+me\s+in\s+(\d+)\s+(second|minute|hour)s?", 0.8),
            (r"(?i)timer\s+(\d+)\s+(second|minute|hour)s?", 0.7),
            (r"(?i)(\d+)\s+(second|minute|hour)\s+timer", 0.7),
            (r"(?i)set\s+timer\s+(\d+)", 0.6),
        ];

        for (pattern_str, confidence) in timer_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("timer", pattern, confidence, Some(Self::extract_timer_params));
            }
        }
    }

    fn add_weather_patterns(&mut self) {
        let weather_patterns = vec![
            (r"(?i)what'?s\s+the\s+weather\s+(?:like\s+)?(?:in\s+)?(.+)?", 0.9),
            (r"(?i)weather\s+(?:in\s+)?(.+)?", 0.8),
            (r"(?i)how'?s\s+the\s+weather\s+(?:in\s+)?(.+)?", 0.8),
            (r"(?i)is\s+it\s+(?:raining|sunny|cloudy|snowing)\s+(?:in\s+)?(.+)?", 0.7),
            (r"(?i)temperature\s+(?:in\s+)?(.+)?", 0.7),
        ];

        for (pattern_str, confidence) in weather_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("weather", pattern, confidence, Some(Self::extract_weather_params));
            }
        }
    }

    fn add_app_launch_patterns(&mut self) {
        let app_patterns = vec![
            (r"(?i)open\s+(.+)", 0.9),
            (r"(?i)launch\s+(.+)", 0.9),
            (r"(?i)start\s+(.+)", 0.8),
            (r"(?i)run\s+(.+)", 0.7),
        ];

        for (pattern_str, confidence) in app_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("app_launch", pattern, confidence, Some(Self::extract_app_params));
            }
        }
    }

    fn add_system_control_patterns(&mut self) {
        let system_patterns = vec![
            (r"(?i)(?:turn\s+)?volume\s+up", 0.9),
            (r"(?i)(?:turn\s+)?volume\s+down", 0.9),
            (r"(?i)(?:louder|increase\s+volume)", 0.8),
            (r"(?i)(?:quieter|decrease\s+volume|lower\s+volume)", 0.8),
            (r"(?i)mute", 0.9),
            (r"(?i)unmute", 0.9),
            (r"(?i)(?:go\s+to\s+)?sleep", 0.8),
            (r"(?i)shutdown|shut\s+down", 0.9),
            (r"(?i)restart|reboot", 0.9),
        ];

        for (pattern_str, confidence) in system_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("system_control", pattern, confidence, Some(Self::extract_system_params));
            }
        }
    }

    fn add_query_patterns(&mut self) {
        let query_patterns = vec![
            (r"(?i)what\s+is\s+(.+)", 0.7),
            (r"(?i)who\s+is\s+(.+)", 0.7),
            (r"(?i)when\s+(?:is|was|will)\s+(.+)", 0.7),
            (r"(?i)where\s+is\s+(.+)", 0.7),
            (r"(?i)how\s+(?:do\s+(?:i|you)|to)\s+(.+)", 0.7),
            (r"(?i)why\s+(.+)", 0.6),
            (r"(?i)tell\s+me\s+about\s+(.+)", 0.8),
        ];

        for (pattern_str, confidence) in query_patterns {
            if let Ok(pattern) = regex::Regex::new(pattern_str) {
                self.add_pattern("query", pattern, confidence, Some(Self::extract_query_params));
            }
        }
    }

    fn add_pattern(
        &mut self,
        intent_type: &str,
        pattern: regex::Regex,
        confidence: f32,
        extractor: Option<fn(&str) -> HashMap<String, String>>,
    ) {
        let intent_pattern = IntentPattern {
            pattern,
            intent_type: intent_type.to_string(),
            confidence,
            extractor,
        };

        self.patterns
            .entry(intent_type.to_string())
            .or_insert_with(Vec::new)
            .push(intent_pattern);
    }

    fn extract_timer_params(text: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Extract duration and unit
        if let Ok(re) = regex::Regex::new(r"(\d+)\s+(second|minute|hour)s?") {
            if let Some(captures) = re.captures(text) {
                if let (Some(num), Some(unit)) = (captures.get(1), captures.get(2)) {
                    params.insert("number".to_string(), num.as_str().to_string());
                    params.insert("unit".to_string(), unit.as_str().to_string());
                }
            }
        }
        
        // Extract optional label
        if let Ok(re) = regex::Regex::new(r"(?:for|called|named)\s+(.+?)(?:\s+timer)?$") {
            if let Some(captures) = re.captures(text) {
                if let Some(label) = captures.get(1) {
                    params.insert("label".to_string(), label.as_str().to_string());
                }
            }
        }
        
        params
    }

    fn extract_weather_params(text: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Extract location
        if let Ok(re) = regex::Regex::new(r"(?:in|for|at)\s+(.+?)(?:\s+today|\s+tomorrow|$)") {
            if let Some(captures) = re.captures(text) {
                if let Some(location) = captures.get(1) {
                    let loc = location.as_str().trim();
                    if !loc.is_empty() && loc.len() > 1 {
                        params.insert("location".to_string(), loc.to_string());
                    }
                }
            }
        }
        
        params
    }

    fn extract_app_params(text: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Extract app name (everything after the action word)
        if let Ok(re) = regex::Regex::new(r"(?:open|launch|start|run)\s+(.+)") {
            if let Some(captures) = re.captures(text) {
                if let Some(app) = captures.get(1) {
                    params.insert("app_name".to_string(), app.as_str().trim().to_string());
                }
            }
        }
        
        params
    }

    fn extract_system_params(text: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        let text_lower = text.to_lowercase();
        
        let action = if text_lower.contains("volume up") || text_lower.contains("louder") || text_lower.contains("increase") {
            "volume_up"
        } else if text_lower.contains("volume down") || text_lower.contains("quieter") || text_lower.contains("decrease") || text_lower.contains("lower") {
            "volume_down"
        } else if text_lower.contains("mute") && !text_lower.contains("unmute") {
            "mute"
        } else if text_lower.contains("unmute") {
            "unmute"
        } else if text_lower.contains("sleep") {
            "sleep"
        } else if text_lower.contains("shutdown") || text_lower.contains("shut down") {
            "shutdown"
        } else if text_lower.contains("restart") || text_lower.contains("reboot") {
            "restart"
        } else {
            "unknown"
        };
        
        params.insert("action".to_string(), action.to_string());
        params
    }

    fn extract_query_params(text: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        
        // Extract the question part
        if let Ok(re) = regex::Regex::new(r"(?:what|who|when|where|how|why|tell\s+me\s+about)\s+(?:is\s+|was\s+|will\s+|do\s+|to\s+)?(.+)") {
            if let Some(captures) = re.captures(text) {
                if let Some(question) = captures.get(1) {
                    params.insert("question".to_string(), question.as_str().trim().to_string());
                }
            }
        }
        
        params
    }

    fn match_patterns(&self, text: &str) -> Option<(String, f32, HashMap<String, String>)> {
        let mut best_match: Option<(String, f32, HashMap<String, String>)> = None;
        
        for patterns in self.patterns.values() {
            for pattern in patterns {
                if pattern.pattern.is_match(text) {
                    let params = if let Some(extractor) = pattern.extractor {
                        extractor(text)
                    } else {
                        HashMap::new()
                    };
                    
                    if best_match.is_none() || pattern.confidence > best_match.as_ref().unwrap().1 {
                        best_match = Some((pattern.intent_type.clone(), pattern.confidence, params));
                    }
                }
            }
        }
        
        best_match
    }

    fn build_enhanced_intent(&self, intent_type: &str, params: HashMap<String, String>) -> EnhancedIntent {
        match intent_type {
            "greeting" => {
                // For now, hardcode the user name - in a real system this would come from user profile
                EnhancedIntent::Greeting { user_name: Some("Aryan".to_string()) }
            }
            "timer" => {
                let duration_secs = if let (Some(num_str), Some(unit)) = (params.get("number"), params.get("unit")) {
                    if let Ok(num) = num_str.parse::<u64>() {
                        match unit.as_str() {
                            "second" => num,
                            "minute" => num * 60,
                            "hour" => num * 3600,
                            _ => num,
                        }
                    } else {
                        10 // default
                    }
                } else {
                    10 // default
                };
                
                let label = params.get("label").cloned();
                EnhancedIntent::Timer { duration_secs, label }
            }
            "weather" => {
                let location = params.get("location").cloned();
                EnhancedIntent::Weather { location }
            }
            "app_launch" => {
                let app_name = params.get("app_name").cloned().unwrap_or_else(|| "unknown".to_string());
                EnhancedIntent::AppLaunch { app_name }
            }
            "system_control" => {
                let action = match params.get("action").map(|s| s.as_str()).unwrap_or("unknown") {
                    "volume_up" => SystemAction::VolumeUp,
                    "volume_down" => SystemAction::VolumeDown,
                    "mute" => SystemAction::Mute,
                    "unmute" => SystemAction::Unmute,
                    "sleep" => SystemAction::Sleep,
                    "shutdown" => SystemAction::Shutdown,
                    "restart" => SystemAction::Restart,
                    _ => SystemAction::VolumeUp, // default
                };
                EnhancedIntent::SystemControl { action }
            }
            "query" => {
                let question = params.get("question").cloned().unwrap_or_else(|| "unknown".to_string());
                EnhancedIntent::Query { question }
            }
            _ => EnhancedIntent::Unknown { text: "unknown intent".to_string() },
        }
    }
}

#[async_trait]
impl NluEngine for EnhancedNlu {
    async fn parse_intent(&self, text: &str) -> Intent {
        debug!("Parsing intent for: '{}'", text);
        
        if let Some((intent_type, confidence, params)) = self.match_patterns(text) {
            if confidence >= self.confidence_threshold {
                info!("Matched intent '{}' with confidence {:.2}", intent_type, confidence);
                let enhanced_intent = self.build_enhanced_intent(&intent_type, params);
                
                // Convert enhanced intent back to basic Intent for compatibility
                match enhanced_intent {
                    EnhancedIntent::Greeting { user_name } => {
                        Intent::Greeting { user_name }
                    }
                    EnhancedIntent::Timer { duration_secs, .. } => {
                        Intent::Timer { duration_secs }
                    }
                    _ => Intent::Unknown { text: text.to_string() },
                }
            } else {
                debug!("Intent '{}' confidence {:.2} below threshold {:.2}", intent_type, confidence, self.confidence_threshold);
                Intent::Unknown { text: text.to_string() }
            }
        } else {
            debug!("No intent patterns matched");
            Intent::Unknown { text: text.to_string() }
        }
    }
}

impl Default for EnhancedNlu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timer_intent_recognition() {
        let nlu = EnhancedNlu::new();
        
        let test_cases = vec![
            ("set a timer for 5 minutes", 300),
            ("remind me in 30 seconds", 30),
            ("timer 2 hours", 7200),
            ("set timer 10", 10),
        ];
        
        for (text, expected_secs) in test_cases {
            match nlu.parse_intent(text).await {
                Intent::Timer { duration_secs } => {
                    assert_eq!(duration_secs, expected_secs, "Failed for: {}", text);
                }
                _ => panic!("Expected Timer intent for: {}", text),
            }
        }
    }

    #[tokio::test]
    async fn test_unknown_intent() {
        let nlu = EnhancedNlu::new();
        
        let test_cases = vec![
            "hello there",
            "how are you",
            "random text",
        ];
        
        for text in test_cases {
            match nlu.parse_intent(text).await {
                Intent::Unknown { .. } => {
                    // Expected
                }
                _ => panic!("Expected Unknown intent for: {}", text),
            }
        }
    }

    #[test]
    fn test_parameter_extraction() {
        let params = EnhancedNlu::extract_timer_params("set a timer for 5 minutes");
        assert_eq!(params.get("number"), Some(&"5".to_string()));
        assert_eq!(params.get("unit"), Some(&"minute".to_string()));
        
        let params = EnhancedNlu::extract_weather_params("what's the weather in New York");
        assert_eq!(params.get("location"), Some(&"New York".to_string()));
        
        let params = EnhancedNlu::extract_app_params("open Chrome");
        assert_eq!(params.get("app_name"), Some(&"Chrome".to_string()));
    }
}