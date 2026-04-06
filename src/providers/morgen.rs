use anyhow::{Context, Result};
use chrono::{Duration, Local, NaiveDate, TimeZone};
use iso8601_duration::Duration as IsoDuration;
use serde::Deserialize;
use std::collections::HashMap;

use crate::config::ProviderConfig;
use crate::models::CalendarEvent;
use super::CalendarProvider;

pub struct MorgenProvider {
    config: ProviderConfig,
    api_key: String,
}

#[derive(Deserialize)]
struct MorgenCalendarRights {
    #[serde(rename = "mayReadItems")]
    may_read_items: bool,
}

#[derive(Deserialize)]
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
    time_zone: String,
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

impl MorgenProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let api_key = std::env::var(&config.env_api_key)
            .map_err(|_| anyhow::anyhow!(
                "API key not set. Please set the {} environment variable.",
                config.env_api_key
            ))?;
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

    fn get_calendars(&self) -> Result<Vec<MorgenCalendar>> {
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
}

impl CalendarProvider for MorgenProvider {
    fn name(&self) -> &str {
        "morgen"
    }

    fn get_events(&self, date: NaiveDate) -> Result<Vec<CalendarEvent>> {
        let calendars = self.get_calendars()?;

        // Group readable, non-ignored calendar IDs by account
        let mut account_calendar_map: HashMap<String, Vec<String>> = HashMap::new();
        for cal in &calendars {
            if cal.my_rights.may_read_items
                && !self.config.calendars_to_ignore.contains(&cal.name)
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
            let calendar_ids_str = calendar_ids.join(",");
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
                .parse()
                .with_context(|| format!("unknown timezone: {}", me.time_zone))?;

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
                title: me.title,
                start_time,
                end_time,
                description: me.description,
                location: me.location,
                attendees: Vec::new(),
            });
        }

        Ok(events)
    }
}
