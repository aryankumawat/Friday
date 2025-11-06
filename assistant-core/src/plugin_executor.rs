use crate::{Intent, Executor, EngineEvent, EngineError};
use crate::plugin_system::{PluginManager, WeatherPlugin};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use std::collections::HashMap;

/// Executor that integrates with the plugin system
pub struct PluginExecutor {
    plugin_manager: PluginManager,
    fallback_executor: Box<dyn Executor + Send + Sync>,
}

impl PluginExecutor {
    pub fn new<E: Executor + Send + Sync + 'static>(fallback_executor: E) -> Self {
        Self {
            plugin_manager: PluginManager::new("~/.friday/plugins"),
            fallback_executor: Box::new(fallback_executor),
        }
    }

    pub async fn initialize(&mut self) -> Result<(), EngineError> {
        info!("Initializing plugin executor");
        
        // Load built-in plugins
        self.load_builtin_plugins().await?;
        
        // TODO: Load external plugins from directory
        // self.load_external_plugins().await?;
        
        info!("Plugin executor initialized with {} plugins", 
              self.plugin_manager.list_plugins().len());
        Ok(())
    }

    async fn load_builtin_plugins(&mut self) -> Result<(), EngineError> {
        // Load weather plugin
        let weather_plugin = WeatherPlugin::new();
        self.plugin_manager.load_plugin(weather_plugin).await
            .map_err(|e| EngineError::Audio(format!("Failed to load weather plugin: {}", e)))?;
        
        info!("Built-in plugins loaded successfully");
        Ok(())
    }

    /// Try to handle intent with plugins first, fallback to default executor
    async fn try_plugin_execution(&self, text: &str, events: mpsc::Sender<EngineEvent>) -> Option<String> {
        // Check if any plugin can handle this intent
        if let Some((plugin_name, intent, parameters)) = self.plugin_manager.find_plugin_for_intent(text) {
            info!("Found plugin '{}' for intent '{}' in text: '{}'", plugin_name, intent, text);
            
            match self.plugin_manager.execute_plugin(&plugin_name, &intent, parameters, events.clone()).await {
                Ok(result) => {
                    if result.success {
                        // Process plugin events
                        for event in result.events {
                            match event {
                                crate::plugin_system::PluginEvent::Log { level, message } => {
                                    match level.as_str() {
                                        "info" => info!("Plugin {}: {}", plugin_name, message),
                                        "warn" => warn!("Plugin {}: {}", plugin_name, message),
                                        "error" => warn!("Plugin {}: {}", plugin_name, message),
                                        _ => debug!("Plugin {}: {}", plugin_name, message),
                                    }
                                }
                                crate::plugin_system::PluginEvent::Notification { title, body } => {
                                    let _ = events.send(EngineEvent::Notification(format!("{}: {}", title, body))).await;
                                }
                                _ => {
                                    debug!("Plugin event: {:?}", event);
                                }
                            }
                        }
                        
                        return Some(result.message);
                    } else {
                        warn!("Plugin execution failed: {}", result.message);
                    }
                }
                Err(e) => {
                    warn!("Plugin execution error: {}", e);
                }
            }
        }
        
        None
    }

    /// Get list of available plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugin_manager.list_plugins()
            .iter()
            .map(|manifest| format!("{} v{} - {}", manifest.name, manifest.version, manifest.description))
            .collect()
    }

    /// Configure plugin
    pub fn configure_plugin(&mut self, plugin_name: &str, config: HashMap<String, serde_json::Value>) -> Result<(), EngineError> {
        self.plugin_manager.set_plugin_config(plugin_name, config)
            .map_err(|e| EngineError::Audio(format!("Failed to configure plugin: {}", e)))
    }
}

#[async_trait]
impl Executor for PluginExecutor {
    async fn execute(&self, intent: &Intent, events: mpsc::Sender<EngineEvent>) -> Result<String, EngineError> {
        debug!("Plugin executor processing intent: {:?}", intent);
        
        match intent {
            Intent::Unknown { text } => {
                // Try plugin execution first
                if let Some(response) = self.try_plugin_execution(text, events.clone()).await {
                    return Ok(response);
                }
                
                // Fallback to default executor
                debug!("No plugin found for intent, using fallback executor");
                self.fallback_executor.execute(intent, events).await
            }
            _ => {
                // For known intents, use fallback executor directly
                self.fallback_executor.execute(intent, events).await
            }
        }
    }
}

/// Factory for creating plugin executor with different configurations
pub struct PluginExecutorBuilder {
    plugins_dir: Option<String>,
    security_enabled: bool,
    builtin_plugins: bool,
}

impl PluginExecutorBuilder {
    pub fn new() -> Self {
        Self {
            plugins_dir: None,
            security_enabled: true,
            builtin_plugins: true,
        }
    }

    pub fn with_plugins_dir<S: Into<String>>(mut self, dir: S) -> Self {
        self.plugins_dir = Some(dir.into());
        self
    }

    pub fn with_security(mut self, enabled: bool) -> Self {
        self.security_enabled = enabled;
        self
    }

    pub fn with_builtin_plugins(mut self, enabled: bool) -> Self {
        self.builtin_plugins = enabled;
        self
    }

    pub async fn build<E: Executor + Send + Sync + 'static>(self, fallback_executor: E) -> Result<PluginExecutor, EngineError> {
        let plugins_dir = self.plugins_dir.unwrap_or_else(|| "~/.friday/plugins".to_string());
        
        let mut plugin_manager = PluginManager::new(plugins_dir)
            .with_security(self.security_enabled);
        
        if self.builtin_plugins {
            // Load built-in plugins
            let weather_plugin = WeatherPlugin::new();
            plugin_manager.load_plugin(weather_plugin).await
                .map_err(|e| EngineError::Audio(format!("Failed to load weather plugin: {}", e)))?;
        }
        
        Ok(PluginExecutor {
            plugin_manager,
            fallback_executor: Box::new(fallback_executor),
        })
    }
}

impl Default for PluginExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SimpleExecutor;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_plugin_executor_weather() {
        let mut executor = PluginExecutor::new(SimpleExecutor);
        executor.initialize().await.unwrap();
        
        let (tx, _rx) = mpsc::channel(10);
        let intent = Intent::Unknown { text: "what's the weather like in Boston".to_string() };
        
        let result = executor.execute(&intent, tx).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.contains("Boston") || response.contains("weather"));
    }

    #[tokio::test]
    async fn test_plugin_executor_fallback() {
        let mut executor = PluginExecutor::new(SimpleExecutor);
        executor.initialize().await.unwrap();
        
        let (tx, _rx) = mpsc::channel(10);
        let intent = Intent::Timer { duration_secs: 60 };
        
        let result = executor.execute(&intent, tx).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(response.contains("Timer set"));
    }

    #[tokio::test]
    async fn test_plugin_executor_builder() {
        let executor = PluginExecutorBuilder::new()
            .with_security(false)
            .with_builtin_plugins(true)
            .build(SimpleExecutor)
            .await;
        
        assert!(executor.is_ok());
        
        let plugin_executor = executor.unwrap();
        let plugins = plugin_executor.list_plugins();
        assert!(!plugins.is_empty());
        assert!(plugins.iter().any(|p| p.contains("weather")));
    }

    #[tokio::test]
    async fn test_plugin_list() {
        let mut executor = PluginExecutor::new(SimpleExecutor);
        executor.initialize().await.unwrap();
        
        let plugins = executor.list_plugins();
        assert!(!plugins.is_empty());
        
        // Should have weather plugin
        assert!(plugins.iter().any(|p| p.contains("weather")));
    }
}