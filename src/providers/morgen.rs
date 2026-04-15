use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use iso8601_duration::Duration as IsoDuration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::CalendarProvider;
use crate::config::ProviderConfig;
use crate::models::CalendarEvent;

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

pub struct MorgenProvider {
    config: ProviderConfig,
    api_key: String,
}

#[derive(Deserialize, Serialize)]
struct MorgenCalendarRights {
    #[serde(rename = "mayReadItems")]
    may_read_items: bool,
}

#[derive(Deserialize, Serialize)]
struct MorgenCalendar {
    id: String,
    name: String,
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "myRights")]
    my_rights: MorgenCalendarRights,
}

#[derive(Deserialize)]
struct MorgenCalendarsResponseData {
    calendars: Vec<MorgenCalendar>,
}

#[derive(Deserialize)]
struct MorgenCalendarsResponse {
    data: MorgenCalendarsResponseData,
}

#[derive(Deserialize)]
struct MorgenEvent {
    id: String,
    title: String,
    start: String,
    duration: String,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
    description: Option<String>,
    location: Option<String>,
}

#[derive(Deserialize)]
struct MorgenEventsResponseData {
    events: Vec<MorgenEvent>,
}

#[derive(Deserialize)]
struct MorgenEventsResponse {
    data: MorgenEventsResponseData,
}

#[derive(Serialize, Deserialize)]
struct CalendarCache {
    cached_at: DateTime<Utc>,
    calendars: Vec<MorgenCalendar>,
}

fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .expect("could not determine cache directory")
        .join("agenda")
        .join("morgen_calendars.json")
}

impl MorgenProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let api_key = std::env::var(&config.env_api_key).map_err(|_| {
            anyhow::anyhow!(
                "API key not set. Please set the {} environment variable.",
                config.env_api_key
            )
        })?;
        Ok(MorgenProvider { config, api_key })
    }

    fn build_request(&self, url: &str) -> ureq::Request {
        let mut req = ureq::get(url);
        for (key, value) in &self.config.headers {
            let header_value = if key == "Authorization" {
                value.replace("{API_KEY}", &self.api_key)
            } else {
                value.clone()
            };
            req = req.set(key, &header_value);
        }
        req
    }

    fn fetch_calendars(&self) -> Result<Vec<MorgenCalendar>> {
        let url = format!("{}/calendars/list", self.config.base_url);
        let response = self
            .build_request(&url)
            .call()
            .context("failed to fetch calendars")?;
        let data: MorgenCalendarsResponse = response
            .into_json()
            .context("failed to deserialize calendars response")?;
        Ok(data.data.calendars)
    }

    fn load_cache() -> Option<CalendarCache> {
        let path = cache_path();
        let contents = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn save_cache(calendars: &[MorgenCalendar]) -> Result<()> {
        let path = cache_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create cache directory")?;
        }
        let cache = CalendarCache {
            cached_at: Utc::now(),
            calendars: calendars
                .iter()
                .map(|c| MorgenCalendar {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    account_id: c.account_id.clone(),
                    my_rights: MorgenCalendarRights {
                        may_read_items: c.my_rights.may_read_items,
                    },
                })
                .collect(),
        };
        let json = serde_json::to_string_pretty(&cache).context("failed to serialize cache")?;
        fs::write(&path, json).context("failed to write calendar cache")?;
        Ok(())
    }

    fn get_calendars_cached(&self, force_refresh: bool) -> Result<Vec<MorgenCalendar>> {
        let ttl = Duration::seconds(self.config.calendar_cache_ttl_seconds as i64);

        if !force_refresh {
            if let Some(cache) = Self::load_cache() {
                let age = Utc::now().signed_duration_since(cache.cached_at);
                if age < ttl {
                    return Ok(cache.calendars);
                }
            }
        }

        let calendars = self.fetch_calendars()?;
        if let Err(e) = Self::save_cache(&calendars) {
            eprintln!("Warning: failed to write calendar cache: {:#}", e);
        }
        Ok(calendars)
    }
}

impl CalendarProvider for MorgenProvider {
    fn get_events(&self, date: NaiveDate, force_refresh: bool) -> Result<Vec<CalendarEvent>> {
        let calendars = self.get_calendars_cached(force_refresh)?;

        // Group readable, non-ignored calendar IDs by account
        let mut account_calendar_map: HashMap<String, Vec<String>> = HashMap::new();
        for cal in &calendars {
            if cal.my_rights.may_read_items
                && !(self.config.calendars_to_ignore.contains(&cal.name)
                    || self.config.calendars_to_ignore.contains(&cal.id))
            {
                account_calendar_map
                    .entry(cal.account_id.clone())
                    .or_default()
                    .push(cal.id.clone());
            }
        }

        let start_of_day = date
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
            .and_local_timezone(Local)
            .single()
            .expect("valid local datetime");
        let end_of_day = start_of_day + Duration::hours(24);
        let start_str = start_of_day.to_rfc3339();
        let end_str = end_of_day.to_rfc3339();

        let url = format!("{}/events/list", self.config.base_url);
        let mut raw_events: Vec<MorgenEvent> = Vec::new();

        for (account_id, calendar_ids) in &account_calendar_map {
            let mut unique_ids = calendar_ids.clone();
            unique_ids.sort();
            unique_ids.dedup();
            let calendar_ids_str = unique_ids.join(",");
            let response = self
                .build_request(&url)
                .query("start", &start_str)
                .query("end", &end_str)
                .query("accountId", account_id)
                .query("calendarIds", &calendar_ids_str)
                .call()
                .with_context(|| format!("failed to fetch events for account {}", account_id))?;
            let data: MorgenEventsResponse = response
                .into_json()
                .context("failed to deserialize events response")?;
            raw_events.extend(data.data.events);
        }

        let mut events: Vec<CalendarEvent> = Vec::new();
        for me in raw_events {
            let tz: chrono_tz::Tz = me
                .time_zone
                .as_deref()
                .unwrap_or("UTC")
                .parse()
                .with_context(|| {
                    format!(
                        "unknown timezone: {}",
                        me.time_zone.as_deref().unwrap_or("UTC")
                    )
                })?;

            let naive = chrono::NaiveDateTime::parse_from_str(&me.start, "%Y-%m-%dT%H:%M:%S")
                .with_context(|| format!("failed to parse start time: {}", me.start))?;

            let start_time = tz
                .from_local_datetime(&naive)
                .single()
                .with_context(|| format!("ambiguous local time: {}", me.start))?
                .with_timezone(&Local);

            let iso_dur: IsoDuration = me
                .duration
                .parse()
                .map_err(|_| anyhow::anyhow!("failed to parse duration: {}", me.duration))?;
            let total_secs = (iso_dur.day * 86400.0
                + iso_dur.hour * 3600.0
                + iso_dur.minute * 60.0
                + iso_dur.second) as i64;
            let end_time = start_time + Duration::seconds(total_secs);

            events.push(CalendarEvent {
                id: me.id,
                title: decode_html_entities(&me.title),
                start_time,
                end_time,
                description: me.description.as_deref().map(decode_html_entities),
                location: me.location.as_deref().map(decode_html_entities),
                attendees: Vec::new(),
            });
        }

        Ok(events)
    }
}
