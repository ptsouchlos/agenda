use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;

use crate::models::CalendarEvent;

pub struct EventFormatter {
    time_format: String,
    handlebars: Handlebars<'static>,
}

#[derive(Serialize)]
struct TemplateData {
    #[serde(rename = "Title")]
    title: String,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Location")]
    location: String,
    #[serde(rename = "StartTimeFormatted")]
    start_time_formatted: String,
    #[serde(rename = "EndTimeFormatted")]
    end_time_formatted: String,
    #[serde(rename = "Duration")]
    duration: String,
    #[serde(rename = "Attendees")]
    attendees: Vec<String>,
}

impl EventFormatter {
    pub fn new(time_format: &str, event_template: &str) -> Result<Self> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("event", event_template)
            .context("failed to compile event template")?;
        Ok(EventFormatter {
            time_format: time_format.to_string(),
            handlebars,
        })
    }

    pub fn format_event(&self, event: &CalendarEvent) -> Result<String> {
        let duration_secs = (event.end_time - event.start_time).num_seconds();
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let duration = if hours > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        };

        let data = TemplateData {
            title: event.title.clone(),
            description: event.description.clone().unwrap_or_default(),
            location: event.location.clone().unwrap_or_default(),
            start_time_formatted: event.start_time.format(&self.time_format).to_string(),
            end_time_formatted: event.end_time.format(&self.time_format).to_string(),
            duration,
            attendees: event.attendees.clone(),
        };

        let rendered = self
            .handlebars
            .render("event", &data)
            .context("failed to render event template")?;
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use chrono::TimeZone;

    fn make_event(title: &str, start_h: u32, end_h: u32) -> CalendarEvent {
        let start = Local.with_ymd_and_hms(2026, 4, 6, start_h, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 4, 6, end_h, 0, 0).unwrap();
        CalendarEvent {
            id: "1".to_string(),
            title: title.to_string(),
            start_time: start,
            end_time: end,
            description: None,
            location: None,
            attendees: Vec::new(),
        }
    }

    #[test]
    fn formats_event_with_default_template() {
        let formatter = EventFormatter::new(
            "%H:%M",
            "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}",
        )
        .unwrap();
        let event = make_event("Standup", 9, 10);
        let result = formatter.format_event(&event).unwrap();
        assert_eq!(result, "- 09:00-10:00: Standup");
    }

    #[test]
    fn formats_event_with_duration_template() {
        let formatter = EventFormatter::new(
            "%H:%M",
            "- {{StartTimeFormatted}} ({{Duration}}): {{Title}}",
        )
        .unwrap();
        let event = make_event("Meeting", 14, 15);
        let result = formatter.format_event(&event).unwrap();
        assert_eq!(result, "- 14:00 (1h0m): Meeting");
    }

    #[test]
    fn invalid_template_returns_error() {
        let result = EventFormatter::new("%H:%M", "{{#if}}unclosed");
        assert!(result.is_err());
    }
}
