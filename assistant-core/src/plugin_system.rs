use crate::{Intent, EngineEvent, EngineError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, error};
use std::path::Path;

/// Plugin metadata and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub entry_point: String,
    pub permissions: Vec<Permission>,
    pub dependencies: Vec<String>,
    pub intent_patterns: Vec<IntentPattern>,
    pub config_schema: Option<serde_json::Value>,
}

/// Plugin permissions for security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    FileSystem { paths: Vec<String> },
    Network { domains: Vec<String> },
    SystemCommands { commands: Vec<String> },
    AudioAccess,
    ConfigAccess,
    AllPermissions, // For trusted plugins
}

/// Intent pattern definition for plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPattern {
    pub name: String,
    pub patterns: Vec<String>,
    pub confidence: f32,
    pub parameters: Vec<ParameterDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub param_type: String, // "string", "number", "boolean", "array"
    pub required: bool,
    pub description: String,
}

/// Plugin execution context
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub plugin_name: String,
    pub config: HashMap<String, serde_json::Value>,
    pub permissions: Vec<Permission>,
    pub data_dir: String,
}

/// Plugin execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
    pub events: Vec<PluginEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginEvent {
    Log { level: String, message: String },
    Notification { title: String, body: String },
    StateChange { key: String, value: serde_json::Value },
    CustomEvent { event_type: String, data: serde_json::Value },
}

/// Plugin trait for custom functionality
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin manifest
    fn manifest(&self) -> &PluginManifest;
    
    /// Initialize plugin with context
    async fn initialize(&mut self, context: PluginContext) -> Result<(), PluginError>;
    
    /// Execute plugin with intent and parameters
    async fn execute(
        &self,
        intent: &str,
        parameters: HashMap<String, serde_json::Value>,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<PluginResult, PluginError>;
    
    /// Cleanup plugin resources
    async fn cleanup(&mut self) -> Result<(), PluginError>;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<(), PluginError>;
}

/// Plugin system errors
#[derive(thiserror::Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Plugin manager for loading and executing plugins
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    plugin_configs: HashMap<String, HashMap<String, serde_json::Value>>,
    plugins_dir: String,
    security_enabled: bool,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugins_dir: P) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_configs: HashMap::new(),
            plugins_dir: plugins_dir.as_ref().to_string_lossy().to_string(),
            security_enabled: true,
        }
    }

    pub fn with_security(mut self, enabled: bool) -> Self {
        self.security_enabled = enabled;
        self
    }

    /// Load plugin from directory
    pub async fn load_plugin<P: Plugin + 'static>(&mut self, plugin: P) -> Result<(), PluginError> {
        let manifest = plugin.manifest().clone();
        let plugin_name = manifest.name.clone();
        
        info!("Loading plugin: {} v{}", plugin_name, manifest.version);
        
        // Validate plugin manifest
        self.validate_manifest(&manifest)?;
        
        // Create plugin context
        let context = PluginContext {
            plugin_name: plugin_name.clone(),
            config: self.plugin_configs.get(&plugin_name).cloned().unwrap_or_default(),
            permissions: manifest.permissions.clone(),
            data_dir: format!("{}/data/{}", self.plugins_dir, plugin_name),
        };
        
        // Initialize plugin
        let mut boxed_plugin = Box::new(plugin);
        boxed_plugin.initialize(context).await?;
        
        // Store plugin
        self.plugins.insert(plugin_name.clone(), boxed_plugin);
        
        info!("Plugin loaded successfully: {}", plugin_name);
        Ok(())
    }

    /// Execute plugin by name
    pub async fn execute_plugin(
        &self,
        plugin_name: &str,
        intent: &str,
        parameters: HashMap<String, serde_json::Value>,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<PluginResult, PluginError> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| PluginError::NotFound(plugin_name.to_string()))?;
        
        debug!("Executing plugin: {} with intent: {}", plugin_name, intent);
        
        // Check permissions if security is enabled
        if self.security_enabled {
            self.check_permissions(plugin_name, intent)?;
        }
        
        // Execute plugin
        let result = plugin.execute(intent, parameters, events).await?;
        
        debug!("Plugin execution completed: {} -> {}", plugin_name, result.success);
        Ok(result)
    }

    /// Find plugin that can handle intent
    pub fn find_plugin_for_intent(&self, intent_text: &str) -> Option<(String, String, HashMap<String, serde_json::Value>)> {
        for (plugin_name, plugin) in &self.plugins {
            let manifest = plugin.manifest();
            
            for pattern in &manifest.intent_patterns {
                if self.matches_pattern(intent_text, pattern) {
                    let parameters = self.extract_parameters(intent_text, pattern);
                    return Some((plugin_name.clone(), pattern.name.clone(), parameters));
                }
            }
        }
        None
    }

    /// List all loaded plugins
    pub fn list_plugins(&self) -> Vec<&PluginManifest> {
        self.plugins.values().map(|p| p.manifest()).collect()
    }

    /// Unload plugin
    pub async fn unload_plugin(&mut self, plugin_name: &str) -> Result<(), PluginError> {
        if let Some(mut plugin) = self.plugins.remove(plugin_name) {
            plugin.cleanup().await?;
            info!("Plugin unloaded: {}", plugin_name);
        }
        Ok(())
    }

    /// Set plugin configuration
    pub fn set_plugin_config(
        &mut self,
        plugin_name: &str,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), PluginError> {
        if let Some(plugin) = self.plugins.get(plugin_name) {
            plugin.validate_config(&config)?;
            self.plugin_configs.insert(plugin_name.to_string(), config);
            Ok(())
        } else {
            Err(PluginError::NotFound(plugin_name.to_string()))
        }
    }

    fn validate_manifest(&self, manifest: &PluginManifest) -> Result<(), PluginError> {
        if manifest.name.is_empty() {
            return Err(PluginError::InvalidConfig("Plugin name cannot be empty".to_string()));
        }
        
        if manifest.version.is_empty() {
            return Err(PluginError::InvalidConfig("Plugin version cannot be empty".to_string()));
        }
        
        // Validate intent patterns
        for pattern in &manifest.intent_patterns {
            if pattern.patterns.is_empty() {
                return Err(PluginError::InvalidConfig(
                    format!("Intent pattern '{}' has no patterns", pattern.name)
                ));
            }
        }
        
        Ok(())
    }

    fn check_permissions(&self, plugin_name: &str, intent: &str) -> Result<(), PluginError> {
        // In a real implementation, this would check specific permissions
        // For now, we'll allow all operations for loaded plugins
        debug!("Permission check passed for plugin: {} intent: {}", plugin_name, intent);
        Ok(())
    }

    fn matches_pattern(&self, text: &str, pattern: &IntentPattern) -> bool {
        let text_lower = text.to_lowercase();
        
        for pattern_str in &pattern.patterns {
            if let Ok(regex) = regex::Regex::new(&pattern_str.to_lowercase()) {
                if regex.is_match(&text_lower) {
                    return true;
                }
            } else {
                // Fallback to simple string matching
                if text_lower.contains(&pattern_str.to_lowercase()) {
                    return true;
                }
            }
        }
        
        false
    }

    fn extract_parameters(&self, text: &str, pattern: &IntentPattern) -> HashMap<String, serde_json::Value> {
        let mut parameters = HashMap::new();
        
        // Simple parameter extraction - in a real implementation this would be more sophisticated
        for param in &pattern.parameters {
            match param.param_type.as_str() {
                "number" => {
                    if let Some(num) = self.extract_number(text) {
                        parameters.insert(param.name.clone(), serde_json::Value::Number(num.into()));
                    }
                }
                "string" => {
                    // Extract quoted strings or specific patterns
                    if let Some(string_val) = self.extract_string(text, &param.name) {
                        parameters.insert(param.name.clone(), serde_json::Value::String(string_val));
                    }
                }
                _ => {}
            }
        }
        
        parameters
    }

    fn extract_number(&self, text: &str) -> Option<i64> {
        for word in text.split_whitespace() {
            if let Ok(num) = word.parse::<i64>() {
                return Some(num);
            }
        }
        None
    }

    fn extract_string(&self, text: &str, param_name: &str) -> Option<String> {
        // Simple extraction - look for words after common prepositions
        let prepositions = ["for", "to", "in", "at", "on", "with"];
        let words: Vec<&str> = text.split_whitespace().collect();
        
        for (i, word) in words.iter().enumerate() {
            if prepositions.contains(&word.to_lowercase().as_str()) && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }
        
        None
    }
}

/// Example weather plugin implementation
pub struct WeatherPlugin {
    manifest: PluginManifest,
    context: Option<PluginContext>,
    api_key: Option<String>,
}

impl WeatherPlugin {
    pub fn new() -> Self {
        let manifest = PluginManifest {
            name: "weather".to_string(),
            version: "1.0.0".to_string(),
            description: "Weather information plugin".to_string(),
            author: "Friday Assistant".to_string(),
            entry_point: "weather_plugin".to_string(),
            permissions: vec![Permission::Network { domains: vec!["api.openweathermap.org".to_string()] }],
            dependencies: vec![],
            intent_patterns: vec![
                IntentPattern {
                    name: "get_weather".to_string(),
                    patterns: vec![
                        r"(?i)weather.*in\s+(\w+)".to_string(),
                        r"(?i)what.*weather.*like".to_string(),
                        r"(?i)temperature.*in\s+(\w+)".to_string(),
                    ],
                    confidence: 0.8,
                    parameters: vec![
                        ParameterDef {
                            name: "location".to_string(),
                            param_type: "string".to_string(),
                            required: false,
                            description: "Location for weather query".to_string(),
                        }
                    ],
                }
            ],
            config_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "api_key": {
                        "type": "string",
                        "description": "OpenWeatherMap API key"
                    },
                    "default_location": {
                        "type": "string",
                        "description": "Default location for weather queries"
                    }
                },
                "required": ["api_key"]
            })),
        };

        Self {
            manifest,
            context: None,
            api_key: None,
        }
    }
}

#[async_trait]
impl Plugin for WeatherPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&mut self, context: PluginContext) -> Result<(), PluginError> {
        self.api_key = context.config.get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        self.context = Some(context);
        
        info!("Weather plugin initialized");
        Ok(())
    }

    async fn execute(
        &self,
        intent: &str,
        parameters: HashMap<String, serde_json::Value>,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<PluginResult, PluginError> {
        match intent {
            "get_weather" => {
                let location = parameters.get("location")
                    .and_then(|v| v.as_str())
                    .unwrap_or("your location");

                // Simulate weather API call
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                let weather_info = if self.api_key.is_some() {
                    format!("The weather in {} is partly cloudy with a temperature of 72°F", location)
                } else {
                    format!("Weather information for {} is not available. Please configure an API key.", location)
                };

                Ok(PluginResult {
                    success: true,
                    message: weather_info,
                    data: Some(serde_json::json!({
                        "location": location,
                        "temperature": 72,
                        "condition": "partly_cloudy"
                    })),
                    events: vec![
                        PluginEvent::Log {
                            level: "info".to_string(),
                            message: format!("Weather query for {}", location),
                        }
                    ],
                })
            }
            _ => Err(PluginError::ExecutionFailed(format!("Unknown intent: {}", intent))),
        }
    }

    async fn cleanup(&mut self) -> Result<(), PluginError> {
        info!("Weather plugin cleanup completed");
        Ok(())
    }

    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<(), PluginError> {
        if !config.contains_key("api_key") {
            return Err(PluginError::InvalidConfig("api_key is required".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_plugin_manager() {
        let mut manager = PluginManager::new("/tmp/plugins");
        let weather_plugin = WeatherPlugin::new();
        
        // Load plugin
        assert!(manager.load_plugin(weather_plugin).await.is_ok());
        
        // Check plugin is loaded
        assert_eq!(manager.list_plugins().len(), 1);
        
        // Test intent matching
        let result = manager.find_plugin_for_intent("what's the weather like in New York");
        assert!(result.is_some());
        
        let (plugin_name, intent, _params) = result.unwrap();
        assert_eq!(plugin_name, "weather");
        assert_eq!(intent, "get_weather");
    }

    #[tokio::test]
    async fn test_weather_plugin_execution() {
        let mut manager = PluginManager::new("/tmp/plugins");
        let weather_plugin = WeatherPlugin::new();
        
        manager.load_plugin(weather_plugin).await.unwrap();
        
        let (tx, _rx) = mpsc::channel(10);
        let mut params = HashMap::new();
        params.insert("location".to_string(), serde_json::Value::String("Boston".to_string()));
        
        let result = manager.execute_plugin("weather", "get_weather", params, tx).await;
        assert!(result.is_ok());
        
        let plugin_result = result.unwrap();
        assert!(plugin_result.success);
        assert!(plugin_result.message.contains("Boston"));
    }

    #[test]
    fn test_intent_pattern_matching() {
        let manager = PluginManager::new("/tmp/plugins");
        let pattern = IntentPattern {
            name: "test".to_string(),
            patterns: vec![r"(?i)weather.*in\s+(\w+)".to_string()],
            confidence: 0.8,
            parameters: vec![],
        };
        
        assert!(manager.matches_pattern("What's the weather in Boston", &pattern));
        assert!(manager.matches_pattern("weather in NYC", &pattern));
        assert!(!manager.matches_pattern("set a timer", &pattern));
    }
}