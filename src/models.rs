use chrono::{DateTime, Local};

pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn can_construct_calendar_event() {
        let start = Local.with_ymd_and_hms(2026, 4, 6, 9, 0, 0).unwrap();
        let end = Local.with_ymd_and_hms(2026, 4, 6, 10, 0, 0).unwrap();
        let event = CalendarEvent {
            id: "abc123".to_string(),
            title: "Standup".to_string(),
            start_time: start,
            end_time: end,
            description: Some("Daily sync".to_string()),
            location: None,
            attendees: vec!["alice@example.com".to_string()],
        };
        assert_eq!(event.title, "Standup");
        assert_eq!(event.attendees.len(), 1);
        assert!(event.location.is_none());
    }
}
