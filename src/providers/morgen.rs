use anyhow::Result;
use chrono::NaiveDate;

use crate::config::ProviderConfig;
use crate::models::CalendarEvent;
use super::CalendarProvider;

pub struct MorgenProvider {
    config: ProviderConfig,
    api_key: String,
}

impl MorgenProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let api_key = std::env::var(&config.env_api_key)
            .map_err(|_| anyhow::anyhow!(
                "API key not set. Please set the {} environment variable.",
                config.env_api_key
            ))?;
        Ok(MorgenProvider { config, api_key })
    }
}

impl CalendarProvider for MorgenProvider {
    fn name(&self) -> &str {
        "morgen"
    }

    fn get_events(&self, _date: NaiveDate) -> Result<Vec<CalendarEvent>> {
        unimplemented!("Morgen provider not yet implemented")
    }
}
