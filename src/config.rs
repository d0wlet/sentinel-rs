use anyhow::Result;

use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct LogRule {
    pub name: String,
    pub pattern: String, // Regex string from config
    pub threshold: u64,  // Simple threshold (e.g. notify after X occurrences)
                         // In a real app we might have time_window, etc.
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub rules: Vec<LogRule>,
    pub polling_interval_ms: u64,
    pub webhook_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                LogRule {
                    name: "Error".to_string(),
                    pattern: "(?i)error".to_string(),
                    threshold: 1,
                },
                LogRule {
                    name: "Panic".to_string(),
                    pattern: "(?i)panic".to_string(),
                    threshold: 1,
                },
            ],
            polling_interval_ms: 100,
            webhook_url: None,
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<AppConfig> {
    if !path.as_ref().exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}
