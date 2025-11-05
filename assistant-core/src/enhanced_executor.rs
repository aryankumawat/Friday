use crate::{Intent, Executor, EngineEvent, EngineError};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use tokio::process::Command;
use tracing::{debug, info, warn, error};
use std::collections::HashMap;

/// Enhanced executor that handles multiple intent types
pub struct EnhancedExecutor {
    weather_api_key: Option<String>,
    app_aliases: HashMap<String, String>,
}

impl EnhancedExecutor {
    pub fn new() -> Self {
        let mut executor = Self {
            weather_api_key: None,
            app_aliases: HashMap::new(),
        };
        executor.initialize_app_aliases();
        executor
    }

    pub fn with_weather_api_key(mut self, api_key: String) -> Self {
        self.weather_api_key = Some(api_key);
        self
    }

    fn initialize_app_aliases(&mut self) {
        // Common app aliases for different platforms
        let aliases = vec![
            // Browsers
            ("chrome", "Google Chrome"),
            ("firefox", "Firefox"),
            ("safari", "Safari"),
            ("edge", "Microsoft Edge"),
            
            // Development
            ("vscode", "Visual Studio Code"),
            ("code", "Visual Studio Code"),
            ("terminal", "Terminal"),
            ("iterm", "iTerm"),
            
            // Communication
            ("slack", "Slack"),
            ("discord", "Discord"),
            ("zoom", "Zoom"),
            ("teams", "Microsoft Teams"),
            
            // Productivity
            ("notes", "Notes"),
            ("calendar", "Calendar"),
            ("mail", "Mail"),
            ("finder", "Finder"),
            
            // Media
            ("spotify", "Spotify"),
            ("music", "Music"),
            ("photos", "Photos"),
            ("vlc", "VLC"),
        ];

        for (alias, app_name) in aliases {
            self.app_aliases.insert(alias.to_string(), app_name.to_string());
        }
    }

    async fn execute_timer(&self, duration_secs: u64, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        let msg = format!("Timer set for {} seconds", duration_secs);
        events.send(EngineEvent::ExecutionStarted("timer".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;
        
        // Fire-and-forget notification after duration
        let events_clone = events.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(duration_secs)).await;
            let _ = events_clone.send(EngineEvent::Notification("Timer done!".into())).await;
            let _ = events_clone.send(EngineEvent::ExecutionFinished("timer".into())).await;
        });
        
        Ok(msg)
    }

    async fn execute_weather(&self, location: Option<String>, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        events.send(EngineEvent::ExecutionStarted("weather".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        let location_str = location.unwrap_or_else(|| "your location".to_string());
        
        // Simulate weather API call
        info!("Fetching weather for: {}", location_str);
        sleep(Duration::from_millis(500)).await; // Simulate API delay
        
        let response = if self.weather_api_key.is_some() {
            // In a real implementation, this would call a weather API
            format!("The weather in {} is partly cloudy with a temperature of 72 degrees Fahrenheit", location_str)
        } else {
            format!("Weather information for {} is not available. Please configure a weather API key.", location_str)
        };

        events.send(EngineEvent::ExecutionFinished("weather".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        Ok(response)
    }

    async fn execute_app_launch(&self, app_name: String, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        events.send(EngineEvent::ExecutionStarted("app_launch".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        // Resolve app alias
        let resolved_app = self.app_aliases.get(&app_name.to_lowercase())
            .unwrap_or(&app_name)
            .clone();

        info!("Launching app: {} (resolved from: {})", resolved_app, app_name);

        let result = self.launch_app_by_platform(&resolved_app).await;
        
        let response = match result {
            Ok(_) => format!("Launched {}", resolved_app),
            Err(e) => {
                warn!("Failed to launch {}: {}", resolved_app, e);
                format!("Sorry, I couldn't launch {}. {}", resolved_app, e)
            }
        };

        events.send(EngineEvent::ExecutionFinished("app_launch".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        Ok(response)
    }

    async fn launch_app_by_platform(&self, app_name: &str) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("open")
                .arg("-a")
                .arg(app_name)
                .output()
                .await
                .map_err(|e| format!("Failed to execute open command: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(format!("App '{}' not found or failed to launch", app_name))
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = Command::new("start")
                .arg("")
                .arg(app_name)
                .output()
                .await
                .map_err(|e| format!("Failed to execute start command: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(format!("App '{}' not found or failed to launch", app_name))
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = Command::new("xdg-open")
                .arg(app_name)
                .output()
                .await
                .map_err(|e| format!("Failed to execute xdg-open command: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(format!("App '{}' not found or failed to launch", app_name))
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err("App launching not supported on this platform".to_string())
        }
    }

    async fn execute_system_control(&self, action: &str, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        events.send(EngineEvent::ExecutionStarted("system_control".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        info!("Executing system control action: {}", action);

        let result = self.execute_system_action(action).await;
        
        let response = match result {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Failed to execute system action {}: {}", action, e);
                format!("Sorry, I couldn't {}. {}", action, e)
            }
        };

        events.send(EngineEvent::ExecutionFinished("system_control".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        Ok(response)
    }

    async fn execute_system_action(&self, action: &str) -> Result<String, String> {
        match action {
            "volume_up" => {
                #[cfg(target_os = "macos")]
                {
                    let output = Command::new("osascript")
                        .arg("-e")
                        .arg("set volume output volume (output volume of (get volume settings) + 10)")
                        .output()
                        .await
                        .map_err(|e| format!("Failed to increase volume: {}", e))?;

                    if output.status.success() {
                        Ok("Volume increased".to_string())
                    } else {
                        Err("Failed to increase volume".to_string())
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok("Volume up (simulated)".to_string())
                }
            }
            "volume_down" => {
                #[cfg(target_os = "macos")]
                {
                    let output = Command::new("osascript")
                        .arg("-e")
                        .arg("set volume output volume (output volume of (get volume settings) - 10)")
                        .output()
                        .await
                        .map_err(|e| format!("Failed to decrease volume: {}", e))?;

                    if output.status.success() {
                        Ok("Volume decreased".to_string())
                    } else {
                        Err("Failed to decrease volume".to_string())
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok("Volume down (simulated)".to_string())
                }
            }
            "mute" => {
                #[cfg(target_os = "macos")]
                {
                    let output = Command::new("osascript")
                        .arg("-e")
                        .arg("set volume with output muted")
                        .output()
                        .await
                        .map_err(|e| format!("Failed to mute: {}", e))?;

                    if output.status.success() {
                        Ok("Audio muted".to_string())
                    } else {
                        Err("Failed to mute audio".to_string())
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok("Audio muted (simulated)".to_string())
                }
            }
            "unmute" => {
                #[cfg(target_os = "macos")]
                {
                    let output = Command::new("osascript")
                        .arg("-e")
                        .arg("set volume without output muted")
                        .output()
                        .await
                        .map_err(|e| format!("Failed to unmute: {}", e))?;

                    if output.status.success() {
                        Ok("Audio unmuted".to_string())
                    } else {
                        Err("Failed to unmute audio".to_string())
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Ok("Audio unmuted (simulated)".to_string())
                }
            }
            _ => Ok(format!("System action '{}' executed (simulated)", action)),
        }
    }

    async fn execute_query(&self, question: String, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        events.send(EngineEvent::ExecutionStarted("query".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        info!("Processing query: {}", question);
        
        // Simulate processing time
        sleep(Duration::from_millis(300)).await;
        
        // Simple knowledge base responses
        let response = self.get_knowledge_response(&question);

        events.send(EngineEvent::ExecutionFinished("query".into())).await
            .map_err(|e| EngineError::Audio(e.to_string()))?;

        Ok(response)
    }

    fn get_knowledge_response(&self, question: &str) -> String {
        let question_lower = question.to_lowercase();
        
        // Simple pattern matching for common questions
        if question_lower.contains("time") {
            let now = chrono::Local::now();
            format!("The current time is {}", now.format("%I:%M %p"))
        } else if question_lower.contains("date") {
            let now = chrono::Local::now();
            format!("Today is {}", now.format("%A, %B %d, %Y"))
        } else if question_lower.contains("friday") && question_lower.contains("assistant") {
            "I'm Friday, your voice assistant. I can help you with timers, weather, launching apps, and answering questions.".to_string()
        } else if question_lower.contains("help") {
            "I can help you with setting timers, checking weather, launching applications, controlling system volume, and answering basic questions. Just ask me!".to_string()
        } else {
            format!("I'm not sure about '{}'. You could try asking about the time, date, weather, or ask me to launch an app or set a timer.", question)
        }
    }
}

#[async_trait]
impl Executor for EnhancedExecutor {
    async fn execute(&self, intent: &Intent, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        debug!("Executing intent: {:?}", intent);
        
        match intent {
            Intent::Timer { duration_secs } => {
                self.execute_timer(*duration_secs, events).await
            }
            Intent::Unknown { text } => {
                // Try to handle as different intent types based on text content
                let text_lower = text.to_lowercase();
                
                if text_lower.contains("weather") {
                    // Extract location if present
                    let location = if let Some(pos) = text_lower.find(" in ") {
                        Some(text[pos + 4..].trim().to_string())
                    } else {
                        None
                    };
                    self.execute_weather(location, events).await
                } else if text_lower.contains("open ") || text_lower.contains("launch ") || text_lower.contains("start ") {
                    // Extract app name
                    let app_name = if let Some(pos) = text_lower.find("open ") {
                        text[pos + 5..].trim().to_string()
                    } else if let Some(pos) = text_lower.find("launch ") {
                        text[pos + 7..].trim().to_string()
                    } else if let Some(pos) = text_lower.find("start ") {
                        text[pos + 6..].trim().to_string()
                    } else {
                        "unknown".to_string()
                    };
                    self.execute_app_launch(app_name, events).await
                } else if text_lower.contains("volume") || text_lower.contains("mute") {
                    let action = if text_lower.contains("volume up") || text_lower.contains("louder") {
                        "volume_up"
                    } else if text_lower.contains("volume down") || text_lower.contains("quieter") {
                        "volume_down"
                    } else if text_lower.contains("mute") && !text_lower.contains("unmute") {
                        "mute"
                    } else if text_lower.contains("unmute") {
                        "unmute"
                    } else {
                        "volume_up"
                    };
                    self.execute_system_control(action, events).await
                } else if text_lower.starts_with("what") || text_lower.starts_with("who") || 
                         text_lower.starts_with("when") || text_lower.starts_with("where") ||
                         text_lower.starts_with("how") || text_lower.starts_with("why") {
                    self.execute_query(text.clone(), events).await
                } else {
                    Ok("I'm not sure how to help with that. Try asking about the weather, setting a timer, or launching an app.".to_string())
                }
            }
        }
    }
}

impl Default for EnhancedExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_timer_execution() {
        let executor = EnhancedExecutor::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let intent = Intent::Timer { duration_secs: 5 };
        let result = executor.execute(&intent, tx).await;
        
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Timer set for 5 seconds"));
    }

    #[tokio::test]
    async fn test_weather_execution() {
        let executor = EnhancedExecutor::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let intent = Intent::Unknown { text: "what's the weather in New York".to_string() };
        let result = executor.execute(&intent, tx).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("weather") || response.contains("New York"));
    }

    #[tokio::test]
    async fn test_app_launch_execution() {
        let executor = EnhancedExecutor::new();
        let (tx, _rx) = mpsc::channel(10);
        
        let intent = Intent::Unknown { text: "open chrome".to_string() };
        let result = executor.execute(&intent, tx).await;
        
        assert!(result.is_ok());
        // Result may vary based on platform and whether Chrome is installed
    }

    #[test]
    fn test_app_aliases() {
        let executor = EnhancedExecutor::new();
        
        assert_eq!(executor.app_aliases.get("chrome"), Some(&"Google Chrome".to_string()));
        assert_eq!(executor.app_aliases.get("vscode"), Some(&"Visual Studio Code".to_string()));
        assert_eq!(executor.app_aliases.get("terminal"), Some(&"Terminal".to_string()));
    }
}