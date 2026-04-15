use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_FOLDER: &str = "agenda";
const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub provider: String,
    pub time_format: String,
    pub event_template: String,
    pub config_version: u32,
    pub providers: HashMap<String, ProviderConfig>,
}

fn default_cache_ttl_seconds() -> u64 {
    86400
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProviderConfig {
    pub base_url: String,
    pub headers: HashMap<String, String>,
    pub env_api_key: String,
    pub calendars_to_ignore: Vec<String>,
    #[serde(default = "default_cache_ttl_seconds")]
    pub calendar_cache_ttl_seconds: u64,
}

pub fn default_config() -> Config {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "ApiKey {API_KEY}".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let mut providers = HashMap::new();
    providers.insert(
        "morgen".to_string(),
        ProviderConfig {
            base_url: "https://api.morgen.so/v3".to_string(),
            headers,
            env_api_key: "MORGEN_API_KEY".to_string(),
            calendars_to_ignore: vec!["ignore_this_calendar".to_string()],
            calendar_cache_ttl_seconds: default_cache_ttl_seconds(),
        },
    );

    Config {
        provider: "morgen".to_string(),
        time_format: "%H:%M".to_string(),
        event_template: "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}".to_string(),
        config_version: CURRENT_CONFIG_VERSION,
        providers,
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("could not determine config directory")
        .join(CONFIG_FOLDER)
        .join(CONFIG_FILE_NAME)
}

pub fn write_config(config: &Config, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }
    let toml_str = toml::to_string_pretty(config).context("failed to serialize config")?;
    fs::write(path, toml_str).context("failed to write config file")?;
    Ok(())
}

pub fn read_config(path: &PathBuf) -> Result<Config> {
    if !path.exists() {
        anyhow::bail!("config file not found at {}", path.display());
    }
    let contents = fs::read_to_string(path).context("failed to read config file")?;
    let config: Config = toml::from_str(&contents).context("failed to parse config file")?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_morgen_provider() {
        let config = default_config();
        assert_eq!(config.provider, "morgen");
        assert!(config.providers.contains_key("morgen"));
        assert_eq!(config.config_version, CURRENT_CONFIG_VERSION);
    }

    #[test]
    fn default_config_uses_strftime_time_format() {
        let config = default_config();
        assert_eq!(config.time_format, "%H:%M");
    }

    #[test]
    fn config_roundtrips_through_toml() {
        let original = default_config();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.provider, original.provider);
        assert_eq!(parsed.time_format, original.time_format);
        assert_eq!(parsed.event_template, original.event_template);
        assert_eq!(parsed.config_version, original.config_version);
        assert!(parsed.providers.contains_key("morgen"));
    }

    #[test]
    fn write_and_read_config_roundtrip() {
        let mut tmp = std::env::temp_dir();
        tmp.push("agenda_test_config.toml");

        let original = default_config();
        write_config(&original, &tmp).unwrap();
        let loaded = read_config(&tmp).unwrap();

        assert_eq!(loaded.provider, original.provider);
        assert_eq!(loaded.time_format, original.time_format);
        fs::remove_file(&tmp).ok();
    }
}
