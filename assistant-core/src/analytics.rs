use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Performance metrics for system monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: u64,
    pub session_id: String,
    pub wake_detection_time_ms: u64,
    pub asr_processing_time_ms: u64,
    pub nlu_processing_time_ms: u64,
    pub execution_time_ms: u64,
    pub tts_processing_time_ms: u64,
    pub total_response_time_ms: u64,
    pub memory_usage_mb: f32,
    pub cpu_usage_percent: f32,
    pub success: bool,
    pub error_type: Option<String>,
}

/// Usage analytics for understanding user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAnalytics {
    pub timestamp: u64,
    pub session_id: String,
    pub user_id: Option<String>,
    pub intent_type: String,
    pub confidence_score: f32,
    pub wake_word_used: Option<String>,
    pub response_satisfaction: Option<f32>, // User feedback score
    pub session_duration_ms: u64,
    pub interaction_count: u32,
    pub platform: String,
    pub location: Option<String>,
}

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub timestamp: u64,
    pub uptime_ms: u64,
    pub total_sessions: u64,
    pub active_sessions: u32,
    pub error_rate: f32,
    pub avg_response_time_ms: f32,
    pub memory_usage_mb: f32,
    pub cpu_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub plugin_count: u32,
    pub cache_hit_rate: f32,
}

/// Analytics aggregation for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub period_start: u64,
    pub period_end: u64,
    pub total_interactions: u64,
    pub unique_users: u32,
    pub avg_response_time_ms: f32,
    pub success_rate: f32,
    pub most_used_intents: Vec<(String, u32)>,
    pub peak_usage_hours: Vec<u8>, // Hours 0-23
    pub error_breakdown: HashMap<String, u32>,
    pub performance_trends: Vec<f32>,
}

/// Analytics and monitoring manager
pub struct AnalyticsManager {
    metrics: Vec<PerformanceMetrics>,
    usage_data: Vec<UsageAnalytics>,
    health_history: Vec<SystemHealth>,
    storage_path: String,
    max_entries: usize,
    start_time: SystemTime,
    session_counter: u64,
}

impl AnalyticsManager {
    pub fn new<P: AsRef<Path>>(storage_path: P) -> Self {
        Self {
            metrics: Vec::new(),
            usage_data: Vec::new(),
            health_history: Vec::new(),
            storage_path: storage_path.as_ref().to_string_lossy().to_string(),
            max_entries: 10000,
            start_time: SystemTime::now(),
            session_counter: 0,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Initialize analytics manager
    pub async fn initialize(&mut self) -> Result<(), AnalyticsError> {
        // Create storage directory
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent).await
                .map_err(AnalyticsError::Io)?;
        }

        // Load existing data
        self.load_data().await?;
        
        info!("Analytics manager initialized with {} metrics entries", self.metrics.len());
        Ok(())
    }

    /// Record performance metrics
    pub async fn record_performance(
        &mut self,
        session_id: String,
        wake_detection_time: Duration,
        asr_time: Duration,
        nlu_time: Duration,
        execution_time: Duration,
        tts_time: Duration,
        success: bool,
        error_type: Option<String>,
    ) -> Result<(), AnalyticsError> {
        let total_time = wake_detection_time + asr_time + nlu_time + execution_time + tts_time;
        
        let metrics = PerformanceMetrics {
            timestamp: current_timestamp(),
            session_id,
            wake_detection_time_ms: wake_detection_time.as_millis() as u64,
            asr_processing_time_ms: asr_time.as_millis() as u64,
            nlu_processing_time_ms: nlu_time.as_millis() as u64,
            execution_time_ms: execution_time.as_millis() as u64,
            tts_processing_time_ms: tts_time.as_millis() as u64,
            total_response_time_ms: total_time.as_millis() as u64,
            memory_usage_mb: self.get_memory_usage(),
            cpu_usage_percent: self.get_cpu_usage(),
            success,
            error_type,
        };

        self.metrics.push(metrics);
        self.cleanup_old_entries().await?;
        
        debug!("Recorded performance metrics for session with total time {}ms", total_time.as_millis());
        Ok(())
    }

    /// Record usage analytics
    pub async fn record_usage(
        &mut self,
        session_id: String,
        user_id: Option<String>,
        intent_type: String,
        confidence_score: f32,
        wake_word: Option<String>,
        session_duration: Duration,
        interaction_count: u32,
        platform: String,
    ) -> Result<(), AnalyticsError> {
        let usage = UsageAnalytics {
            timestamp: current_timestamp(),
            session_id,
            user_id,
            intent_type,
            confidence_score,
            wake_word_used: wake_word,
            response_satisfaction: None,
            session_duration_ms: session_duration.as_millis() as u64,
            interaction_count,
            platform,
            location: None,
        };

        self.usage_data.push(usage);
        self.cleanup_old_entries().await?;
        
        debug!("Recorded usage analytics for intent: {}", intent_type);
        Ok(())
    }

    /// Record user satisfaction feedback
    pub async fn record_satisfaction(
        &mut self,
        session_id: &str,
        satisfaction_score: f32,
    ) -> Result<(), AnalyticsError> {
        // Find the most recent usage entry for this session and update satisfaction
        if let Some(usage) = self.usage_data.iter_mut()
            .rev()
            .find(|u| u.session_id == session_id) {
            usage.response_satisfaction = Some(satisfaction_score);
            debug!("Updated satisfaction score for session {}: {}", session_id, satisfaction_score);
        }
        
        Ok(())
    }

    /// Record system health snapshot
    pub async fn record_system_health(
        &mut self,
        active_sessions: u32,
        plugin_count: u32,
        cache_hit_rate: f32,
    ) -> Result<(), AnalyticsError> {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        let error_rate = self.calculate_error_rate();
        let avg_response_time = self.calculate_avg_response_time();

        let health = SystemHealth {
            timestamp: current_timestamp(),
            uptime_ms: uptime.as_millis() as u64,
            total_sessions: self.session_counter,
            active_sessions,
            error_rate,
            avg_response_time_ms: avg_response_time,
            memory_usage_mb: self.get_memory_usage(),
            cpu_usage_percent: self.get_cpu_usage(),
            disk_usage_percent: self.get_disk_usage(),
            plugin_count,
            cache_hit_rate,
        };

        self.health_history.push(health);
        self.cleanup_old_entries().await?;
        
        debug!("Recorded system health snapshot");
        Ok(())
    }

    /// Generate analytics report for a time period
    pub fn generate_report(&self, hours_back: u64) -> AnalyticsReport {
        let period_start = current_timestamp() - (hours_back * 60 * 60 * 1000);
        let period_end = current_timestamp();

        let recent_usage: Vec<&UsageAnalytics> = self.usage_data
            .iter()
            .filter(|u| u.timestamp >= period_start)
            .collect();

        let recent_metrics: Vec<&PerformanceMetrics> = self.metrics
            .iter()
            .filter(|m| m.timestamp >= period_start)
            .collect();

        let total_interactions = recent_usage.len() as u64;
        let unique_users = recent_usage
            .iter()
            .filter_map(|u| u.user_id.as_ref())
            .collect::<std::collections::HashSet<_>>()
            .len() as u32;

        let avg_response_time = if !recent_metrics.is_empty() {
            recent_metrics.iter().map(|m| m.total_response_time_ms as f32).sum::<f32>() / recent_metrics.len() as f32
        } else {
            0.0
        };

        let success_rate = if !recent_metrics.is_empty() {
            recent_metrics.iter().filter(|m| m.success).count() as f32 / recent_metrics.len() as f32
        } else {
            0.0
        };

        let mut intent_counts: HashMap<String, u32> = HashMap::new();
        for usage in &recent_usage {
            *intent_counts.entry(usage.intent_type.clone()).or_insert(0) += 1;
        }

        let mut most_used_intents: Vec<(String, u32)> = intent_counts.into_iter().collect();
        most_used_intents.sort_by(|a, b| b.1.cmp(&a.1));
        most_used_intents.truncate(10);

        let peak_usage_hours = self.calculate_peak_usage_hours(&recent_usage);

        let mut error_breakdown: HashMap<String, u32> = HashMap::new();
        for metric in &recent_metrics {
            if let Some(ref error) = metric.error_type {
                *error_breakdown.entry(error.clone()).or_insert(0) += 1;
            }
        }

        let performance_trends = self.calculate_performance_trends(&recent_metrics);

        AnalyticsReport {
            period_start,
            period_end,
            total_interactions,
            unique_users,
            avg_response_time_ms: avg_response_time,
            success_rate,
            most_used_intents,
            peak_usage_hours,
            error_breakdown,
            performance_trends,
        }
    }

    /// Get current system health
    pub fn get_current_health(&self) -> SystemHealth {
        let uptime = self.start_time.elapsed().unwrap_or_default();
        
        SystemHealth {
            timestamp: current_timestamp(),
            uptime_ms: uptime.as_millis() as u64,
            total_sessions: self.session_counter,
            active_sessions: 0, // Would be provided by caller
            error_rate: self.calculate_error_rate(),
            avg_response_time_ms: self.calculate_avg_response_time(),
            memory_usage_mb: self.get_memory_usage(),
            cpu_usage_percent: self.get_cpu_usage(),
            disk_usage_percent: self.get_disk_usage(),
            plugin_count: 0, // Would be provided by caller
            cache_hit_rate: 0.0, // Would be provided by caller
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        if self.metrics.is_empty() {
            return PerformanceStats::default();
        }

        let total_requests = self.metrics.len();
        let successful_requests = self.metrics.iter().filter(|m| m.success).count();
        let success_rate = successful_requests as f32 / total_requests as f32;

        let response_times: Vec<u64> = self.metrics.iter().map(|m| m.total_response_time_ms).collect();
        let avg_response_time = response_times.iter().sum::<u64>() as f32 / response_times.len() as f32;
        
        let mut sorted_times = response_times.clone();
        sorted_times.sort();
        let p95_response_time = sorted_times.get(sorted_times.len() * 95 / 100).copied().unwrap_or(0) as f32;
        let p99_response_time = sorted_times.get(sorted_times.len() * 99 / 100).copied().unwrap_or(0) as f32;

        let avg_memory_usage = self.metrics.iter().map(|m| m.memory_usage_mb).sum::<f32>() / self.metrics.len() as f32;
        let avg_cpu_usage = self.metrics.iter().map(|m| m.cpu_usage_percent).sum::<f32>() / self.metrics.len() as f32;

        PerformanceStats {
            total_requests,
            successful_requests,
            success_rate,
            avg_response_time_ms: avg_response_time,
            p95_response_time_ms: p95_response_time,
            p99_response_time_ms: p99_response_time,
            avg_memory_usage_mb: avg_memory_usage,
            avg_cpu_usage_percent: avg_cpu_usage,
        }
    }

    /// Start new session
    pub fn start_session(&mut self) -> String {
        self.session_counter += 1;
        format!("session_{}", self.session_counter)
    }

    /// Export analytics data
    pub async fn export_data(&self, format: ExportFormat) -> Result<String, AnalyticsError> {
        match format {
            ExportFormat::Json => {
                let data = ExportData {
                    metrics: self.metrics.clone(),
                    usage_data: self.usage_data.clone(),
                    health_history: self.health_history.clone(),
                };
                serde_json::to_string_pretty(&data).map_err(AnalyticsError::Serialization)
            }
            ExportFormat::Csv => {
                // Simple CSV export for metrics
                let mut csv = String::from("timestamp,session_id,total_response_time_ms,success,error_type\n");
                for metric in &self.metrics {
                    csv.push_str(&format!(
                        "{},{},{},{},{}\n",
                        metric.timestamp,
                        metric.session_id,
                        metric.total_response_time_ms,
                        metric.success,
                        metric.error_type.as_deref().unwrap_or("")
                    ));
                }
                Ok(csv)
            }
        }
    }

    fn calculate_error_rate(&self) -> f32 {
        if self.metrics.is_empty() {
            return 0.0;
        }
        
        let errors = self.metrics.iter().filter(|m| !m.success).count();
        errors as f32 / self.metrics.len() as f32
    }

    fn calculate_avg_response_time(&self) -> f32 {
        if self.metrics.is_empty() {
            return 0.0;
        }
        
        let total_time: u64 = self.metrics.iter().map(|m| m.total_response_time_ms).sum();
        total_time as f32 / self.metrics.len() as f32
    }

    fn calculate_peak_usage_hours(&self, usage_data: &[&UsageAnalytics]) -> Vec<u8> {
        let mut hour_counts = vec![0u32; 24];
        
        for usage in usage_data {
            let hour = ((usage.timestamp / 1000 / 3600) % 24) as usize;
            if hour < 24 {
                hour_counts[hour] += 1;
            }
        }
        
        // Convert to percentages
        let max_count = hour_counts.iter().max().copied().unwrap_or(1);
        hour_counts.into_iter()
            .map(|count| ((count as f32 / max_count as f32) * 100.0) as u8)
            .collect()
    }

    fn calculate_performance_trends(&self, metrics: &[&PerformanceMetrics]) -> Vec<f32> {
        if metrics.len() < 2 {
            return vec![0.0];
        }
        
        // Calculate moving average of response times
        let window_size = 10;
        let mut trends = Vec::new();
        
        for i in 0..metrics.len() {
            let start = if i >= window_size { i - window_size } else { 0 };
            let window = &metrics[start..=i];
            let avg = window.iter().map(|m| m.total_response_time_ms as f32).sum::<f32>() / window.len() as f32;
            trends.push(avg);
        }
        
        trends
    }

    fn get_memory_usage(&self) -> f32 {
        // Simplified memory usage - in real implementation would use system APIs
        #[cfg(target_os = "linux")]
        {
            if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
                for line in contents.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<f32>() {
                                return kb / 1024.0; // Convert to MB
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback estimation
        50.0
    }

    fn get_cpu_usage(&self) -> f32 {
        // Simplified CPU usage - in real implementation would use system APIs
        // This is a placeholder that returns a random-ish value
        (current_timestamp() % 100) as f32 / 4.0
    }

    fn get_disk_usage(&self) -> f32 {
        // Simplified disk usage - in real implementation would check actual disk space
        75.0
    }

    async fn cleanup_old_entries(&mut self) -> Result<(), AnalyticsError> {
        // Keep only the most recent entries
        if self.metrics.len() > self.max_entries {
            let excess = self.metrics.len() - self.max_entries;
            self.metrics.drain(0..excess);
        }
        
        if self.usage_data.len() > self.max_entries {
            let excess = self.usage_data.len() - self.max_entries;
            self.usage_data.drain(0..excess);
        }
        
        if self.health_history.len() > self.max_entries {
            let excess = self.health_history.len() - self.max_entries;
            self.health_history.drain(0..excess);
        }
        
        Ok(())
    }

    async fn save_data(&self) -> Result<(), AnalyticsError> {
        let data = ExportData {
            metrics: self.metrics.clone(),
            usage_data: self.usage_data.clone(),
            health_history: self.health_history.clone(),
        };
        
        let json = serde_json::to_string_pretty(&data)
            .map_err(AnalyticsError::Serialization)?;
        
        fs::write(&self.storage_path, json).await
            .map_err(AnalyticsError::Io)?;
        
        debug!("Saved analytics data to {}", self.storage_path);
        Ok(())
    }

    async fn load_data(&mut self) -> Result<(), AnalyticsError> {
        if !Path::new(&self.storage_path).exists() {
            debug!("No existing analytics data found");
            return Ok(());
        }
        
        let content = fs::read_to_string(&self.storage_path).await
            .map_err(AnalyticsError::Io)?;
        
        let data: ExportData = serde_json::from_str(&content)
            .map_err(AnalyticsError::Serialization)?;
        
        self.metrics = data.metrics;
        self.usage_data = data.usage_data;
        self.health_history = data.health_history;
        
        info!("Loaded analytics data: {} metrics, {} usage entries", 
              self.metrics.len(), self.usage_data.len());
        Ok(())
    }
}

/// Performance statistics summary
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub success_rate: f32,
    pub avg_response_time_ms: f32,
    pub p95_response_time_ms: f32,
    pub p99_response_time_ms: f32,
    pub avg_memory_usage_mb: f32,
    pub avg_cpu_usage_percent: f32,
}

/// Export formats for analytics data
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Data structure for export
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportData {
    metrics: Vec<PerformanceMetrics>,
    usage_data: Vec<UsageAnalytics>,
    health_history: Vec<SystemHealth>,
}

/// Analytics system errors
#[derive(thiserror::Error, Debug)]
pub enum AnalyticsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Invalid time period: {0}")]
    InvalidTimePeriod(String),
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
    async fn test_analytics_manager_initialization() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        let mut manager = AnalyticsManager::new(storage_path);
        
        assert!(manager.initialize().await.is_ok());
        assert_eq!(manager.metrics.len(), 0);
    }

    #[tokio::test]
    async fn test_performance_recording() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        let mut manager = AnalyticsManager::new(storage_path);
        manager.initialize().await.unwrap();
        
        let session_id = manager.start_session();
        
        manager.record_performance(
            session_id,
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(50),
            Duration::from_millis(300),
            Duration::from_millis(150),
            true,
            None,
        ).await.unwrap();
        
        assert_eq!(manager.metrics.len(), 1);
        assert_eq!(manager.metrics[0].total_response_time_ms, 800);
        assert!(manager.metrics[0].success);
    }

    #[tokio::test]
    async fn test_usage_analytics() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        let mut manager = AnalyticsManager::new(storage_path);
        manager.initialize().await.unwrap();
        
        let session_id = manager.start_session();
        
        manager.record_usage(
            session_id.clone(),
            Some("user123".to_string()),
            "weather_query".to_string(),
            0.95,
            Some("friday".to_string()),
            Duration::from_secs(30),
            3,
            "desktop".to_string(),
        ).await.unwrap();
        
        assert_eq!(manager.usage_data.len(), 1);
        assert_eq!(manager.usage_data[0].intent_type, "weather_query");
        assert_eq!(manager.usage_data[0].confidence_score, 0.95);
        
        // Test satisfaction recording
        manager.record_satisfaction(&session_id, 4.5).await.unwrap();
        assert_eq!(manager.usage_data[0].response_satisfaction, Some(4.5));
    }

    #[tokio::test]
    async fn test_analytics_report_generation() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        let mut manager = AnalyticsManager::new(storage_path);
        manager.initialize().await.unwrap();
        
        // Add some test data
        let session_id = manager.start_session();
        
        manager.record_usage(
            session_id.clone(),
            Some("user1".to_string()),
            "weather".to_string(),
            0.9,
            Some("friday".to_string()),
            Duration::from_secs(10),
            1,
            "mobile".to_string(),
        ).await.unwrap();
        
        manager.record_performance(
            session_id,
            Duration::from_millis(50),
            Duration::from_millis(100),
            Duration::from_millis(25),
            Duration::from_millis(200),
            Duration::from_millis(75),
            true,
            None,
        ).await.unwrap();
        
        let report = manager.generate_report(24);
        assert_eq!(report.total_interactions, 1);
        assert_eq!(report.unique_users, 1);
        assert!(report.success_rate > 0.0);
        assert!(report.avg_response_time_ms > 0.0);
    }

    #[test]
    fn test_performance_stats() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        let mut manager = AnalyticsManager::new(storage_path);
        
        // Add test metrics
        manager.metrics.push(PerformanceMetrics {
            timestamp: current_timestamp(),
            session_id: "test1".to_string(),
            wake_detection_time_ms: 50,
            asr_processing_time_ms: 100,
            nlu_processing_time_ms: 25,
            execution_time_ms: 200,
            tts_processing_time_ms: 75,
            total_response_time_ms: 450,
            memory_usage_mb: 64.0,
            cpu_usage_percent: 25.0,
            success: true,
            error_type: None,
        });
        
        let stats = manager.get_performance_stats();
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.success_rate, 1.0);
        assert_eq!(stats.avg_response_time_ms, 450.0);
    }

    #[tokio::test]
    async fn test_data_persistence() {
        let dir = tempdir().unwrap();
        let storage_path = dir.path().join("analytics.json");
        
        // Create manager and add data
        {
            let mut manager = AnalyticsManager::new(&storage_path);
            manager.initialize().await.unwrap();
            
            let session_id = manager.start_session();
            manager.record_usage(
                session_id,
                Some("user1".to_string()),
                "test_intent".to_string(),
                0.8,
                None,
                Duration::from_secs(5),
                1,
                "test".to_string(),
            ).await.unwrap();
            
            manager.save_data().await.unwrap();
        }
        
        // Create new manager and load data
        {
            let mut manager = AnalyticsManager::new(&storage_path);
            manager.initialize().await.unwrap();
            
            assert_eq!(manager.usage_data.len(), 1);
            assert_eq!(manager.usage_data[0].intent_type, "test_intent");
        }
    }
}