use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Configuration validation error: {0}")]
    Validation(String),
}

pub type ConfigResult<T> = Result<T, ConfigError>;

/// Complete Friday Assistant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FridayConfig {
    pub version: String,
    pub audio: AudioConfig,
    pub wake: WakeConfig,
    pub asr: AsrConfig,
    pub tts: TtsConfig,
    pub nlu: NluConfig,
    pub executor: ExecutorConfig,
    pub profiles: HashMap<String, Profile>,
    pub active_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeConfig {
    pub engine: String, // "mock", "porcupine", "realtime", "energy"
    pub porcupine: Option<PorcupineConfig>,
    pub realtime: Option<RealtimeConfig>,
    pub energy: Option<EnergyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PorcupineConfig {
    pub binary_path: String,
    pub keyword_path: String,
    pub device_index: Option<i32>,
    pub sensitivity: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    pub energy_threshold: f32,
    pub wake_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyConfig {
    pub threshold: f32,
    pub trigger_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsrConfig {
    pub engine: String, // "mock", "whisper"
    pub whisper: Option<WhisperConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperConfig {
    pub binary_path: String,
    pub model_path: String,
    pub audio_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub engine: String, // "mock", "piper"
    pub piper: Option<PiperConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperConfig {
    pub binary_path: String,
    pub model_path: String,
    pub output_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NluConfig {
    pub engine: String, // "simple", "enhanced"
    pub enhanced: Option<EnhancedNluConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedNluConfig {
    pub confidence_threshold: f32,
    pub custom_patterns: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub engine: String, // "simple", "enhanced"
    pub enhanced: Option<EnhancedExecutorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedExecutorConfig {
    pub weather_api_key: Option<String>,
    pub app_aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub wake_engine: String,
    pub asr_engine: String,
    pub tts_engine: String,
    pub nlu_engine: String,
    pub executor_engine: String,
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for FridayConfig {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            audio: AudioConfig::default(),
            wake: WakeConfig::default(),
            asr: AsrConfig::default(),
            tts: TtsConfig::default(),
            nlu: NluConfig::default(),
            executor: ExecutorConfig::default(),
            profiles: Self::default_profiles(),
            active_profile: Some("development".to_string()),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            buffer_size: 1024,
            device_name: None,
        }
    }
}

impl Default for WakeConfig {
    fn default() -> Self {
        Self {
            engine: "mock".to_string(),
            porcupine: None,
            realtime: Some(RealtimeConfig {
                energy_threshold: 0.02,
                wake_phrases: vec![
                    "friday".to_string(),
                    "hey friday".to_string(),
                    "hello friday".to_string(),
                ],
            }),
            energy: Some(EnergyConfig {
                threshold: 0.05,
                trigger_duration_ms: 500,
            }),
        }
    }
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            engine: "mock".to_string(),
            whisper: None,
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: "mock".to_string(),
            piper: None,
        }
    }
}

impl Default for NluConfig {
    fn default() -> Self {
        Self {
            engine: "enhanced".to_string(),
            enhanced: Some(EnhancedNluConfig {
                confidence_threshold: 0.6,
                custom_patterns: HashMap::new(),
            }),
        }
    }
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            engine: "enhanced".to_string(),
            enhanced: Some(EnhancedExecutorConfig {
                weather_api_key: None,
                app_aliases: HashMap::new(),
            }),
        }
    }
}

impl FridayConfig {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: FridayConfig = serde_json::from_str(&content)?;
        config.validate()?;
        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        self.validate()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        info!("Configuration saved successfully");
        Ok(())
    }

    /// Load configuration from default location
    pub fn load_default() -> ConfigResult<Self> {
        let config_path = Self::default_config_path()?;
        if config_path.exists() {
            Self::load_from_file(config_path)
        } else {
            info!("No configuration file found, using defaults");
            Ok(Self::default())
        }
    }

    /// Save configuration to default location
    pub fn save_default(&self) -> ConfigResult<()> {
        let config_path = Self::default_config_path()?;
        
        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        self.save_to_file(config_path)
    }

    /// Get default configuration file path
    pub fn default_config_path() -> ConfigResult<std::path::PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| ConfigError::Validation("Could not find home directory".to_string()))?;
        
        Ok(home_dir.join(".friday").join("config.json"))
    }

    /// Validate configuration
    pub fn validate(&self) -> ConfigResult<()> {
        // Validate audio config
        if self.audio.sample_rate == 0 {
            return Err(ConfigError::Validation("Sample rate must be greater than 0".to_string()));
        }
        
        if self.audio.channels == 0 {
            return Err(ConfigError::Validation("Channels must be greater than 0".to_string()));
        }

        // Validate wake config
        match self.wake.engine.as_str() {
            "mock" | "porcupine" | "realtime" | "energy" => {}
            _ => return Err(ConfigError::Validation(format!("Invalid wake engine: {}", self.wake.engine))),
        }

        // Validate ASR config
        match self.asr.engine.as_str() {
            "mock" | "whisper" => {}
            _ => return Err(ConfigError::Validation(format!("Invalid ASR engine: {}", self.asr.engine))),
        }

        // Validate TTS config
        match self.tts.engine.as_str() {
            "mock" | "piper" => {}
            _ => return Err(ConfigError::Validation(format!("Invalid TTS engine: {}", self.tts.engine))),
        }

        // Validate NLU config
        match self.nlu.engine.as_str() {
            "simple" | "enhanced" => {}
            _ => return Err(ConfigError::Validation(format!("Invalid NLU engine: {}", self.nlu.engine))),
        }

        // Validate Executor config
        match self.executor.engine.as_str() {
            "simple" | "enhanced" => {}
            _ => return Err(ConfigError::Validation(format!("Invalid executor engine: {}", self.executor.engine))),
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Get active profile
    pub fn get_active_profile(&self) -> Option<&Profile> {
        if let Some(profile_name) = &self.active_profile {
            self.profiles.get(profile_name)
        } else {
            None
        }
    }

    /// Set active profile
    pub fn set_active_profile(&mut self, profile_name: String) -> ConfigResult<()> {
        if !self.profiles.contains_key(&profile_name) {
            return Err(ConfigError::Validation(format!("Profile '{}' does not exist", profile_name)));
        }
        self.active_profile = Some(profile_name);
        Ok(())
    }

    /// Add or update profile
    pub fn add_profile(&mut self, profile: Profile) {
        self.profiles.insert(profile.name.clone(), profile);
    }

    /// Remove profile
    pub fn remove_profile(&mut self, profile_name: &str) -> ConfigResult<()> {
        if !self.profiles.contains_key(profile_name) {
            return Err(ConfigError::Validation(format!("Profile '{}' does not exist", profile_name)));
        }
        
        // Don't allow removing the active profile
        if self.active_profile.as_ref() == Some(&profile_name.to_string()) {
            return Err(ConfigError::Validation("Cannot remove the active profile".to_string()));
        }
        
        self.profiles.remove(profile_name);
        Ok(())
    }

    /// Create default profiles
    fn default_profiles() -> HashMap<String, Profile> {
        let mut profiles = HashMap::new();
        
        // Development profile (all mock)
        profiles.insert("development".to_string(), Profile {
            name: "development".to_string(),
            description: "Development profile with mock engines for testing".to_string(),
            wake_engine: "mock".to_string(),
            asr_engine: "mock".to_string(),
            tts_engine: "mock".to_string(),
            nlu_engine: "enhanced".to_string(),
            executor_engine: "enhanced".to_string(),
            settings: HashMap::new(),
        });

        // Production profile (real engines)
        profiles.insert("production".to_string(), Profile {
            name: "production".to_string(),
            description: "Production profile with real engines".to_string(),
            wake_engine: "realtime".to_string(),
            asr_engine: "whisper".to_string(),
            tts_engine: "piper".to_string(),
            nlu_engine: "enhanced".to_string(),
            executor_engine: "enhanced".to_string(),
            settings: HashMap::new(),
        });

        // Demo profile (energy wake + enhanced processing)
        profiles.insert("demo".to_string(), Profile {
            name: "demo".to_string(),
            description: "Demo profile with energy-based wake detection".to_string(),
            wake_engine: "energy".to_string(),
            asr_engine: "mock".to_string(),
            tts_engine: "mock".to_string(),
            nlu_engine: "enhanced".to_string(),
            executor_engine: "enhanced".to_string(),
            settings: HashMap::new(),
        });

        profiles
    }

    /// Apply profile settings to current config
    pub fn apply_profile(&mut self, profile_name: &str) -> ConfigResult<()> {
        let profile = self.profiles.get(profile_name)
            .ok_or_else(|| ConfigError::Validation(format!("Profile '{}' not found", profile_name)))?
            .clone();

        self.wake.engine = profile.wake_engine;
        self.asr.engine = profile.asr_engine;
        self.tts.engine = profile.tts_engine;
        self.nlu.engine = profile.nlu_engine;
        self.executor.engine = profile.executor_engine;
        self.active_profile = Some(profile_name.to_string());

        info!("Applied profile: {}", profile_name);
        Ok(())
    }

    /// List available profiles
    pub fn list_profiles(&self) -> Vec<&Profile> {
        self.profiles.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = FridayConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.version, "0.1.0");
        assert_eq!(config.audio.sample_rate, 16000);
        assert_eq!(config.wake.engine, "mock");
    }

    #[test]
    fn test_config_serialization() {
        let config = FridayConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: FridayConfig = serde_json::from_str(&json).unwrap();
        assert!(deserialized.validate().is_ok());
    }

    #[test]
    fn test_config_file_operations() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("test_config.json");
        
        let config = FridayConfig::default();
        config.save_to_file(&config_path).unwrap();
        
        let loaded_config = FridayConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.version, loaded_config.version);
    }

    #[test]
    fn test_profile_management() {
        let mut config = FridayConfig::default();
        
        // Test getting active profile
        let active = config.get_active_profile().unwrap();
        assert_eq!(active.name, "development");
        
        // Test switching profiles
        config.apply_profile("production").unwrap();
        assert_eq!(config.wake.engine, "realtime");
        assert_eq!(config.asr.engine, "whisper");
        
        // Test invalid profile
        assert!(config.apply_profile("nonexistent").is_err());
    }

    #[test]
    fn test_config_validation() {
        let mut config = FridayConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid sample rate should fail
        config.audio.sample_rate = 0;
        assert!(config.validate().is_err());
        
        // Invalid engine should fail
        config.audio.sample_rate = 16000;
        config.wake.engine = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}