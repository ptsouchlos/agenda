use anyhow::Result;
use chrono::NaiveDate;

use crate::config::Config;
use crate::models::CalendarEvent;

mod morgen;

pub use morgen::MorgenProvider;

pub trait CalendarProvider: Send {
    fn name(&self) -> &str;
    fn get_events(&self, date: NaiveDate) -> Result<Vec<CalendarEvent>>;
}

pub fn create_provider(name: &str, config: &Config) -> Result<Box<dyn CalendarProvider>> {
    let provider_config = config
        .providers
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("provider '{}' not found in configuration", name))?
        .clone();

    match name {
        "morgen" => Ok(Box::new(MorgenProvider::new(provider_config)?)),
        _ => Err(anyhow::anyhow!("unsupported provider: {}", name)),
    }
}
