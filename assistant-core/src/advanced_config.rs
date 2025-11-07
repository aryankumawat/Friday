use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration profile for different environments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigProfile {
    pub name: String,
    pub description: String,
    pub audio_config: AudioConfiguration,
    pub wake_config: WakeConfiguration,
    pub asr_config: AsrConfiguration,
    pub nlu_config: NluConfiguration,
    pub tts_config: TtsConfiguration,
    pub plugin_config: PluginConfiguration,
    pub performance_config: PerformanceConfiguration,
    pub security_config: SecurityConfiguration,
    pub created_at: u64,
    pub last_modified: u64,
}

/// Audio system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfiguration {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub noise_suppression: bool,
    pub echo_cancellation: bool,
    pub auto_gain_control: bool,
    pub volume_level: f32,
    pub quality_preset: AudioQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioQuality {
    Low,      // 16kHz, mono
    Medium,   // 22kHz, mono
    High,     // 44kHz, stereo
    Studio,   // 48kHz, stereo
}

/// Wake word detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeConfiguration {
    pub enabled: bool,
    pub wake_words: Vec<WakeWordConfig>,
    pub sensitivity: f32,
    pub timeout_ms: u64,
    pub continuous_listening: bool,
    pub voice_training_enabled: bool,
    pub multi_user_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordConfig {
    pub word: String,
    pub model_path: Option<String>,
    pub sensitivity: f32,
    pub enabled: bool,
}

/// ASR (Automatic Speech Recognition) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfiguration {
    pub engine: AsrEngine,
    pub model_path: String,
    pub language: String,
    pub streaming_enabled: bool,
    pub vad_enabled: bool,
    pub vad_threshold: f32,
    pub silence_timeout_ms: u64,
    pub max_recording_duration_ms: u64,
    pub beam_size: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AsrEngine {
    Whisper,
    Vosk,
    DeepSpeech,
    Custom(String),
}

/// NLU (Natural Language Understanding) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NluConfiguration {
    pub engine: NluEngine,
    pub model_path: Option<String>,
    pub confidence_threshold: f32,
    pub intent_timeout_ms: u64,
    pub context_window_size: usize,
    pub entity_extraction_enabled: bool,
    pub sentiment_analysis_enabled: bool,
    pub language_detection_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NluEngine {
    Simple,
    Rasa,
    Spacy,
    Transformers,
    Custom(String),
}

/// TTS (Text-to-Speech) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfiguration {
    pub engine: TtsEngine,
    pub voice: String,
    pub speed: f32,
    pub pitch: f32,
    pub volume: f32,
    pub model_path: Option<String>,
    pub quality: TtsQuality,
    pub streaming_enabled: bool,
    pub emotion_enabled: bool,
    pub ssml_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TtsEngine {
    Piper,
    Festival,
    Espeak,
    Azure,
    Google,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TtsQuality {
    Fast,
    Balanced,
    HighQuality,
}

/// Plugin system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfiguration {
    pub enabled_plugins: Vec<String>,
    pub plugin_directories: Vec<String>,
    pub auto_load: bool,
    pub sandbox_enabled: bool,
    pub max_execution_time_ms: u64,
    pub memory_limit_mb: u32,
    pub network_access: bool,
    pub file_system_access: bool,
}

/// Performance and resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfiguration {
    pub max_concurrent_sessions: u32,
    pub thread_pool_size: Option<usize>,
    pub memory_limit_mb: Option<u32>,
    pub cpu_limit_percent: Option<f32>,
    pub cache_size_mb: u32,
    pub log_level: LogLevel,
    pub metrics_enabled: bool,
    pub profiling_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfiguration {
    pub encryption_enabled: bool,
    pub api_key_required: bool,
    pub rate_limiting: RateLimitConfig,
    pub allowed_origins: Vec<String>,
    pub data_retention_days: u32,
    pub privacy_mode: bool,
    pub audit_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_minute: u32,
    pub burst_size: u32,
}

/// Configuration validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Advanced configuration manager
pub struct AdvancedConfigManager {
    profiles: HashMap<String, ConfigProfile>,
    active_profile: Option<String>,
    config_directory: PathBuf,
    auto_save: bool,
    validation_enabled: bool,
}

impl AdvancedConfigManager {
    pub fn new<P: AsRef<Path>>(config_directory: P) -> Self {
        Self {
            profiles: HashMap::new(),
            active_profile: None,
            config_directory: config_directory.as_ref().to_path_buf(),
            auto_save: true,
            validation_enabled: true,
        }
    }

    pub fn with_auto_save(mut self, enabled: bool) -> Self {
        self.auto_save = enabled;
        self
    }

    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validation_enabled = enabled;
        self
    }

    /// Initialize configuration manager
    pub async fn initialize(&mut self) -> Result<(), ConfigError> {
        // Create config directory if it doesn't exist
        fs::create_dir_all(&self.config_directory).await
            .map_err(ConfigError::Io)?;

        // Load existing profiles
        self.load_profiles().await?;

        // Create default profile if none exist
        if self.profiles.is_empty() {
            let default_profile = self.create_default_profile();
            self.add_profile(default_profile).await?;
            self.set_active_profile("default").await?;
        }

        info!("Configuration manager initialized with {} profiles", self.profiles.len());
        Ok(())
    }

    /// Create a new configuration profile
    pub async fn create_profile(
        &mut self,
        name: String,
        description: String,
        base_profile: Option<&str>,
    ) -> Result<(), ConfigError> {
        if self.profiles.contains_key(&name) {
            return Err(ConfigError::ProfileExists(name));
        }

        let profile = if let Some(base_name) = base_profile {
            // Clone from existing profile
            let base = self.profiles.get(base_name)
                .ok_or_else(|| ConfigError::ProfileNotFound(base_name.to_string()))?;
            
            let mut new_profile = base.clone();
            new_profile.name = name.clone();
            new_profile.description = description;
            new_profile.created_at = current_timestamp();
            new_profile.last_modified = current_timestamp();
            new_profile
        } else {
            // Create from default
            let mut default = self.create_default_profile();
            default.name = name.clone();
            default.description = description;
            default
        };

        self.add_profile(profile).await?;
        info!("Created new configuration profile: {}", name);
        Ok(())
    }

    /// Add a configuration profile
    pub async fn add_profile(&mut self, profile: ConfigProfile) -> Result<(), ConfigError> {
        if self.validation_enabled {
            let validation = self.validate_profile(&profile);
            if !validation.is_valid {
                return Err(ConfigError::ValidationFailed(validation.errors));
            }
        }

        let name = profile.name.clone();
        self.profiles.insert(name.clone(), profile);

        if self.auto_save {
            self.save_profile(&name).await?;
        }

        Ok(())
    }

    /// Update an existing profile
    pub async fn update_profile(
        &mut self,
        name: &str,
        updater: impl FnOnce(&mut ConfigProfile),
    ) -> Result<(), ConfigError> {
        {
            let profile = self.profiles.get_mut(name)
                .ok_or_else(|| ConfigError::ProfileNotFound(name.to_string()))?;

            updater(profile);
            profile.last_modified = current_timestamp();
        }

        if self.validation_enabled {
            let profile = self.profiles.get(name).unwrap();
            let validation = self.validate_profile(profile);
            if !validation.is_valid {
                return Err(ConfigError::ValidationFailed(validation.errors));
            }
        }

        if self.auto_save {
            self.save_profile(name).await?;
        }

        info!("Updated configuration profile: {}", name);
        Ok(())
    }

    /// Delete a configuration profile
    pub async fn delete_profile(&mut self, name: &str) -> Result<(), ConfigError> {
        if !self.profiles.contains_key(name) {
            return Err(ConfigError::ProfileNotFound(name.to_string()));
        }

        // Don't allow deleting the active profile
        if self.active_profile.as_ref() == Some(&name.to_string()) {
            return Err(ConfigError::CannotDeleteActiveProfile(name.to_string()));
        }

        self.profiles.remove(name);

        // Remove profile file
        let profile_path = self.get_profile_path(name);
        if profile_path.exists() {
            fs::remove_file(profile_path).await.map_err(ConfigError::Io)?;
        }

        info!("Deleted configuration profile: {}", name);
        Ok(())
    }

    /// Set the active configuration profile
    pub async fn set_active_profile(&mut self, name: &str) -> Result<(), ConfigError> {
        if !self.profiles.contains_key(name) {
            return Err(ConfigError::ProfileNotFound(name.to_string()));
        }

        self.active_profile = Some(name.to_string());

        // Save active profile reference
        let active_file = self.config_directory.join("active_profile.txt");
        fs::write(active_file, name).await.map_err(ConfigError::Io)?;

        info!("Set active configuration profile: {}", name);
        Ok(())
    }

    /// Get the active configuration profile
    pub fn get_active_profile(&self) -> Option<&ConfigProfile> {
        self.active_profile.as_ref()
            .and_then(|name| self.profiles.get(name))
    }

    /// Get a specific configuration profile
    pub fn get_profile(&self, name: &str) -> Option<&ConfigProfile> {
        self.profiles.get(name)
    }

    /// List all configuration profiles
    pub fn list_profiles(&self) -> Vec<&ConfigProfile> {
        self.profiles.values().collect()
    }

    /// Export configuration profile
    pub async fn export_profile(&self, name: &str, format: ExportFormat) -> Result<String, ConfigError> {
        let profile = self.profiles.get(name)
            .ok_or_else(|| ConfigError::ProfileNotFound(name.to_string()))?;

        match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(profile).map_err(ConfigError::Serialization)
            }
            ExportFormat::Yaml => {
                // Simplified - just return JSON for now
                serde_json::to_string_pretty(profile).map_err(ConfigError::Serialization)
            }
            ExportFormat::Toml => {
                // Simplified - just return JSON for now
                serde_json::to_string_pretty(profile).map_err(ConfigError::Serialization)
            }
        }
    }

    /// Import configuration profile
    pub async fn import_profile(&mut self, data: &str, format: ExportFormat) -> Result<String, ConfigError> {
        let profile: ConfigProfile = match format {
            ExportFormat::Json => {
                serde_json::from_str(data).map_err(ConfigError::Serialization)?
            }
            ExportFormat::Yaml => {
                // Simplified - assume JSON for now
                serde_json::from_str(data).map_err(ConfigError::Serialization)?
            }
            ExportFormat::Toml => {
                // Simplified - assume JSON for now
                serde_json::from_str(data).map_err(ConfigError::Serialization)?
            }
        };

        let name = profile.name.clone();
        self.add_profile(profile).await?;
        
        info!("Imported configuration profile: {}", name);
        Ok(name)
    }

    /// Validate a configuration profile
    pub fn validate_profile(&self, profile: &ConfigProfile) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate audio configuration
        if profile.audio_config.sample_rate < 8000 || profile.audio_config.sample_rate > 48000 {
            errors.push("Sample rate must be between 8000 and 48000 Hz".to_string());
        }

        if profile.audio_config.volume_level < 0.0 || profile.audio_config.volume_level > 1.0 {
            errors.push("Volume level must be between 0.0 and 1.0".to_string());
        }

        // Validate wake configuration
        if profile.wake_config.sensitivity < 0.0 || profile.wake_config.sensitivity > 1.0 {
            errors.push("Wake sensitivity must be between 0.0 and 1.0".to_string());
        }

        if profile.wake_config.wake_words.is_empty() && profile.wake_config.enabled {
            warnings.push("Wake detection is enabled but no wake words are configured".to_string());
        }

        // Validate ASR configuration
        if profile.asr_config.vad_threshold < 0.0 || profile.asr_config.vad_threshold > 1.0 {
            errors.push("VAD threshold must be between 0.0 and 1.0".to_string());
        }

        if profile.asr_config.max_recording_duration_ms < 1000 {
            warnings.push("Very short max recording duration may cause issues".to_string());
        }

        // Validate NLU configuration
        if profile.nlu_config.confidence_threshold < 0.0 || profile.nlu_config.confidence_threshold > 1.0 {
            errors.push("NLU confidence threshold must be between 0.0 and 1.0".to_string());
        }

        // Validate TTS configuration
        if profile.tts_config.speed < 0.1 || profile.tts_config.speed > 3.0 {
            warnings.push("TTS speed outside normal range (0.1-3.0)".to_string());
        }

        if profile.tts_config.volume < 0.0 || profile.tts_config.volume > 1.0 {
            errors.push("TTS volume must be between 0.0 and 1.0".to_string());
        }

        // Validate performance configuration
        if profile.performance_config.max_concurrent_sessions == 0 {
            errors.push("Max concurrent sessions must be greater than 0".to_string());
        }

        if let Some(memory_limit) = profile.performance_config.memory_limit_mb {
            if memory_limit < 64 {
                warnings.push("Very low memory limit may cause performance issues".to_string());
            }
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Get configuration value by path (dot notation)
    pub fn get_config_value(&self, profile_name: &str, path: &str) -> Result<serde_json::Value, ConfigError> {
        let profile = self.profiles.get(profile_name)
            .ok_or_else(|| ConfigError::ProfileNotFound(profile_name.to_string()))?;

        let json_value = serde_json::to_value(profile).map_err(ConfigError::Serialization)?;
        
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &json_value;

        for part in parts {
            current = current.get(part)
                .ok_or_else(|| ConfigError::ConfigPathNotFound(path.to_string()))?;
        }

        Ok(current.clone())
    }

    /// Set configuration value by path (dot notation)
    pub async fn set_config_value(
        &mut self,
        profile_name: &str,
        path: &str,
        value: serde_json::Value,
    ) -> Result<(), ConfigError> {
        let profile = self.profiles.get_mut(profile_name)
            .ok_or_else(|| ConfigError::ProfileNotFound(profile_name.to_string()))?;

        let mut json_value = serde_json::to_value(&*profile).map_err(ConfigError::Serialization)?;
        
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = &mut json_value;

        // Navigate to the parent of the target field
        for part in &parts[..parts.len() - 1] {
            current = current.get_mut(part)
                .ok_or_else(|| ConfigError::ConfigPathNotFound(path.to_string()))?;
        }

        // Set the final value
        if let Some(last_part) = parts.last() {
            current[last_part] = value;
        }

        // Deserialize back to profile
        *profile = serde_json::from_value(json_value).map_err(ConfigError::Serialization)?;
        profile.last_modified = current_timestamp();

        if self.auto_save {
            self.save_profile(profile_name).await?;
        }

        Ok(())
    }

    fn create_default_profile(&self) -> ConfigProfile {
        ConfigProfile {
            name: "default".to_string(),
            description: "Default configuration profile".to_string(),
            audio_config: AudioConfiguration {
                sample_rate: 16000,
                channels: 1,
                buffer_size: 1024,
                input_device: None,
                output_device: None,
                noise_suppression: true,
                echo_cancellation: true,
                auto_gain_control: true,
                volume_level: 0.8,
                quality_preset: AudioQuality::Medium,
            },
            wake_config: WakeConfiguration {
                enabled: true,
                wake_words: vec![
                    WakeWordConfig {
                        word: "friday".to_string(),
                        model_path: None,
                        sensitivity: 0.7,
                        enabled: true,
                    }
                ],
                sensitivity: 0.7,
                timeout_ms: 5000,
                continuous_listening: true,
                voice_training_enabled: false,
                multi_user_support: false,
            },
            asr_config: AsrConfiguration {
                engine: AsrEngine::Whisper,
                model_path: "models/whisper-base.bin".to_string(),
                language: "en".to_string(),
                streaming_enabled: true,
                vad_enabled: true,
                vad_threshold: 0.3,
                silence_timeout_ms: 2000,
                max_recording_duration_ms: 30000,
                beam_size: Some(5),
                temperature: Some(0.0),
            },
            nlu_config: NluConfiguration {
                engine: NluEngine::Simple,
                model_path: None,
                confidence_threshold: 0.7,
                intent_timeout_ms: 5000,
                context_window_size: 5,
                entity_extraction_enabled: true,
                sentiment_analysis_enabled: false,
                language_detection_enabled: false,
            },
            tts_config: TtsConfiguration {
                engine: TtsEngine::Piper,
                voice: "en_US-lessac-medium".to_string(),
                speed: 1.0,
                pitch: 1.0,
                volume: 0.8,
                model_path: Some("models/en_US-lessac-medium.onnx".to_string()),
                quality: TtsQuality::Balanced,
                streaming_enabled: false,
                emotion_enabled: false,
                ssml_enabled: false,
            },
            plugin_config: PluginConfiguration {
                enabled_plugins: vec!["timer".to_string(), "weather".to_string()],
                plugin_directories: vec!["plugins/".to_string()],
                auto_load: true,
                sandbox_enabled: true,
                max_execution_time_ms: 30000,
                memory_limit_mb: 128,
                network_access: true,
                file_system_access: false,
            },
            performance_config: PerformanceConfiguration {
                max_concurrent_sessions: 10,
                thread_pool_size: None,
                memory_limit_mb: None,
                cpu_limit_percent: None,
                cache_size_mb: 256,
                log_level: LogLevel::Info,
                metrics_enabled: true,
                profiling_enabled: false,
            },
            security_config: SecurityConfiguration {
                encryption_enabled: false,
                api_key_required: false,
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    requests_per_minute: 60,
                    burst_size: 10,
                },
                allowed_origins: vec!["*".to_string()],
                data_retention_days: 30,
                privacy_mode: false,
                audit_logging: false,
            },
            created_at: current_timestamp(),
            last_modified: current_timestamp(),
        }
    }

    fn get_profile_path(&self, name: &str) -> PathBuf {
        self.config_directory.join(format!("{}.json", name))
    }

    async fn save_profile(&self, name: &str) -> Result<(), ConfigError> {
        let profile = self.profiles.get(name)
            .ok_or_else(|| ConfigError::ProfileNotFound(name.to_string()))?;

        let json = serde_json::to_string_pretty(profile)
            .map_err(ConfigError::Serialization)?;

        let profile_path = self.get_profile_path(name);
        fs::write(profile_path, json).await.map_err(ConfigError::Io)?;

        debug!("Saved configuration profile: {}", name);
        Ok(())
    }

    async fn load_profiles(&mut self) -> Result<(), ConfigError> {
        let mut entries = fs::read_dir(&self.config_directory).await
            .map_err(ConfigError::Io)?;

        while let Some(entry) = entries.next_entry().await.map_err(ConfigError::Io)? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_profile(stem).await {
                        Ok(_) => debug!("Loaded profile: {}", stem),
                        Err(e) => warn!("Failed to load profile {}: {}", stem, e),
                    }
                }
            }
        }

        // Load active profile reference
        let active_file = self.config_directory.join("active_profile.txt");
        if active_file.exists() {
            if let Ok(active_name) = fs::read_to_string(active_file).await {
                let active_name = active_name.trim();
                if self.profiles.contains_key(active_name) {
                    self.active_profile = Some(active_name.to_string());
                }
            }
        }

        Ok(())
    }

    async fn load_profile(&mut self, name: &str) -> Result<(), ConfigError> {
        let profile_path = self.get_profile_path(name);
        let content = fs::read_to_string(profile_path).await.map_err(ConfigError::Io)?;
        let profile: ConfigProfile = serde_json::from_str(&content)
            .map_err(ConfigError::Serialization)?;

        self.profiles.insert(name.to_string(), profile);
        Ok(())
    }
}

/// Export formats for configuration
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Yaml,
    Toml,
}

/// Configuration management errors
#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),
    #[error("Profile already exists: {0}")]
    ProfileExists(String),
    #[error("Cannot delete active profile: {0}")]
    CannotDeleteActiveProfile(String),
    #[error("Validation failed: {0:?}")]
    ValidationFailed(Vec<String>),
    #[error("Configuration path not found: {0}")]
    ConfigPathNotFound(String),
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_config_manager_initialization() {
        let dir = tempdir().unwrap();
        let mut manager = AdvancedConfigManager::new(dir.path());
        
        assert!(manager.initialize().await.is_ok());
        assert_eq!(manager.profiles.len(), 1); // Default profile created
        assert!(manager.get_active_profile().is_some());
    }

    #[tokio::test]
    async fn test_profile_creation() {
        let dir = tempdir().unwrap();
        let mut manager = AdvancedConfigManager::new(dir.path());
        manager.initialize().await.unwrap();
        
        manager.create_profile(
            "test_profile".to_string(),
            "Test profile".to_string(),
            Some("default"),
        ).await.unwrap();
        
        assert_eq!(manager.profiles.len(), 2);
        assert!(manager.get_profile("test_profile").is_some());
    }

    #[tokio::test]
    async fn test_profile_validation() {
        let dir = tempdir().unwrap();
        let manager = AdvancedConfigManager::new(dir.path());
        
        let mut profile = manager.create_default_profile();
        profile.audio_config.sample_rate = 100; // Invalid sample rate
        
        let validation = manager.validate_profile(&profile);
        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_value_access() {
        let dir = tempdir().unwrap();
        let mut manager = AdvancedConfigManager::new(dir.path());
        manager.initialize().await.unwrap();
        
        // Get a config value
        let sample_rate = manager.get_config_value("default", "audio_config.sample_rate").unwrap();
        assert_eq!(sample_rate, serde_json::Value::Number(16000.into()));
        
        // Set a config value
        manager.set_config_value(
            "default",
            "audio_config.sample_rate",
            serde_json::Value::Number(22050.into()),
        ).await.unwrap();
        
        let updated_rate = manager.get_config_value("default", "audio_config.sample_rate").unwrap();
        assert_eq!(updated_rate, serde_json::Value::Number(22050.into()));
    }

    #[tokio::test]
    async fn test_profile_export_import() {
        let dir = tempdir().unwrap();
        let mut manager = AdvancedConfigManager::new(dir.path());
        manager.initialize().await.unwrap();
        
        // Export profile
        let exported = manager.export_profile("default", ExportFormat::Json).await.unwrap();
        assert!(!exported.is_empty());
        
        // Import as new profile
        let imported_name = manager.import_profile(&exported, ExportFormat::Json).await.unwrap();
        assert_eq!(imported_name, "default");
    }

    #[test]
    fn test_default_profile_validation() {
        let manager = AdvancedConfigManager::new("/tmp");
        let profile = manager.create_default_profile();
        
        let validation = manager.validate_profile(&profile);
        assert!(validation.is_valid, "Default profile should be valid: {:?}", validation.errors);
    }

    #[tokio::test]
    async fn test_active_profile_management() {
        let dir = tempdir().unwrap();
        let mut manager = AdvancedConfigManager::new(dir.path());
        manager.initialize().await.unwrap();
        
        // Create another profile
        manager.create_profile(
            "production".to_string(),
            "Production profile".to_string(),
            Some("default"),
        ).await.unwrap();
        
        // Switch active profile
        manager.set_active_profile("production").await.unwrap();
        assert_eq!(manager.get_active_profile().unwrap().name, "production");
        
        // Try to delete active profile (should fail)
        let result = manager.delete_profile("production").await;
        assert!(result.is_err());
    }
}