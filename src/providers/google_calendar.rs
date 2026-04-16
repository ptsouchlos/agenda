use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, Utc};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use super::CalendarProvider;
use crate::config::ProviderConfig;
use crate::models::CalendarEvent;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
const DEFAULT_TOKEN_FILENAME: &str = "google_calendar_tokens.json";
const CALENDAR_CACHE_FILENAME: &str = "google_calendar_calendars.json";

pub struct GoogleCalendarProvider {
    config: ProviderConfig,
    client_id: String,
    client_secret: String,
    tokens: RefCell<StoredTokens>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StoredTokens {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default = "default_token_type")]
    token_type: String,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListEntry {
    id: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    primary: Option<bool>,
    #[serde(default)]
    access_role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarListResponse {
    #[serde(default)]
    items: Vec<GoogleCalendarListEntry>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GoogleEventDateTime {
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    date_time: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    time_zone: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GoogleAttendee {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    response_status: Option<String>,
    #[serde(default, rename = "self")]
    is_self: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GoogleEvent {
    id: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    start: Option<GoogleEventDateTime>,
    #[serde(default)]
    end: Option<GoogleEventDateTime>,
    #[serde(default)]
    attendees: Option<Vec<GoogleAttendee>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleEventsResponse {
    #[serde(default)]
    items: Vec<GoogleEvent>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct CalendarCache {
    cached_at: DateTime<Utc>,
    calendars: Vec<GoogleCalendarListEntry>,
}

fn token_cache_path(filename: &str) -> PathBuf {
    dirs::cache_dir()
        .expect("could not determine cache directory")
        .join("agenda")
        .join(filename)
}

fn calendar_cache_path() -> PathBuf {
    dirs::cache_dir()
        .expect("could not determine cache directory")
        .join("agenda")
        .join(CALENDAR_CACHE_FILENAME)
}

fn is_expired(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now + chrono::Duration::seconds(60) >= expires_at
}

fn load_tokens(path: &Path) -> Result<StoredTokens> {
    let contents = fs::read_to_string(path).with_context(|| {
        format!(
            "could not read token cache at {}; run `agenda auth google_calendar` first",
            path.display()
        )
    })?;
    serde_json::from_str(&contents).context("failed to parse token cache")
}

fn save_tokens(path: &Path, tokens: &StoredTokens) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create cache directory")?;
    }
    let json = serde_json::to_string_pretty(tokens).context("failed to serialize tokens")?;
    fs::write(path, json).context("failed to write token cache")?;
    Ok(())
}

fn read_env_var(var_name: &str, purpose: &str) -> Result<String> {
    std::env::var(var_name).map_err(|_| {
        anyhow::anyhow!(
            "environment variable {} is not set (required for {})",
            var_name,
            purpose
        )
    })
}

fn resolve_token_filename(config: &ProviderConfig) -> String {
    config
        .token_cache_filename
        .clone()
        .unwrap_or_else(|| DEFAULT_TOKEN_FILENAME.to_string())
}

impl GoogleCalendarProvider {
    pub fn new(config: ProviderConfig) -> Result<Self> {
        let client_id_var = config.env_oauth_client_id.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "google_calendar provider requires `env_oauth_client_id` in the config"
            )
        })?;
        let client_secret_var = config.env_oauth_client_secret.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "google_calendar provider requires `env_oauth_client_secret` in the config"
            )
        })?;
        let client_id = read_env_var(client_id_var, "OAuth client ID")?;
        let client_secret = read_env_var(client_secret_var, "OAuth client secret")?;

        let tokens_path = token_cache_path(&resolve_token_filename(&config));
        let tokens = load_tokens(&tokens_path)?;

        Ok(GoogleCalendarProvider {
            config,
            client_id,
            client_secret,
            tokens: RefCell::new(tokens),
        })
    }

    fn tokens_path(&self) -> PathBuf {
        token_cache_path(&resolve_token_filename(&self.config))
    }

    fn ensure_valid_access_token(&self) -> Result<String> {
        {
            let tokens = self.tokens.borrow();
            if !is_expired(tokens.expires_at, Utc::now()) {
                return Ok(tokens.access_token.clone());
            }
        }

        // Refresh
        let refresh_token = self.tokens.borrow().refresh_token.clone();
        let http_client = build_http_client()?;
        let client = build_oauth_client(&self.client_id, &self.client_secret, None)?;
        let token_response = client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request(&http_client)
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to refresh access token: {}. Run `agenda auth google_calendar` if the refresh token has been revoked.",
                    e
                )
            })?;

        let new_access = token_response.access_token().secret().clone();
        let new_refresh = token_response
            .refresh_token()
            .map(|t| t.secret().clone())
            .unwrap_or(refresh_token);
        let expires_in = token_response
            .expires_in()
            .map(|d| d.as_secs() as i64)
            .unwrap_or(3600);
        let expires_at = Utc::now() + chrono::Duration::seconds(expires_in);
        let scope = token_response
            .scopes()
            .map(|scopes| {
                scopes
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            });

        let new_tokens = StoredTokens {
            access_token: new_access.clone(),
            refresh_token: new_refresh,
            expires_at,
            scope,
            token_type: "Bearer".to_string(),
        };
        *self.tokens.borrow_mut() = new_tokens.clone();
        save_tokens(&self.tokens_path(), &new_tokens)?;
        Ok(new_access)
    }

    fn build_get(&self, url: &str) -> Result<ureq::Request> {
        let token = self.ensure_valid_access_token()?;
        Ok(ureq::get(url).set("Authorization", &format!("Bearer {}", token)))
    }

    fn fetch_calendar_list(&self) -> Result<Vec<GoogleCalendarListEntry>> {
        let url = format!("{}/users/me/calendarList", self.config.base_url);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut req = self.build_get(&url)?;
            if let Some(tok) = &page_token {
                req = req.query("pageToken", tok);
            }
            let resp = req.call().context("failed to fetch calendar list")?;
            let data: GoogleCalendarListResponse = resp
                .into_json()
                .context("failed to deserialize calendar list response")?;
            out.extend(data.items);
            match data.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }
        Ok(out)
    }

    fn get_calendars_cached(
        &self,
        force_refresh: bool,
    ) -> Result<Vec<GoogleCalendarListEntry>> {
        let ttl = chrono::Duration::seconds(self.config.calendar_cache_ttl_seconds as i64);
        let path = calendar_cache_path();

        if !force_refresh
            && let Ok(contents) = fs::read_to_string(&path)
            && let Ok(cache) = serde_json::from_str::<CalendarCache>(&contents)
        {
            let age = Utc::now().signed_duration_since(cache.cached_at);
            if age < ttl {
                return Ok(cache.calendars);
            }
        }

        let calendars = self.fetch_calendar_list()?;
        let cache = CalendarCache {
            cached_at: Utc::now(),
            calendars: calendars.clone(),
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&cache) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("Warning: failed to write calendar cache: {:#}", e);
                }
            }
            Err(e) => eprintln!("Warning: failed to serialize calendar cache: {:#}", e),
        }
        Ok(calendars)
    }

    fn fetch_events_for_calendar(
        &self,
        calendar_id: &str,
        time_min: &str,
        time_max: &str,
    ) -> Result<Vec<GoogleEvent>> {
        let encoded_id = urlencoding_encode(calendar_id);
        let url = format!("{}/calendars/{}/events", self.config.base_url, encoded_id);
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut req = self
                .build_get(&url)?
                .query("singleEvents", "true")
                .query("orderBy", "startTime")
                .query("timeMin", time_min)
                .query("timeMax", time_max)
                .query("maxResults", "250");
            if let Some(tok) = &page_token {
                req = req.query("pageToken", tok);
            }
            let resp = req.call().with_context(|| {
                format!("failed to fetch events for calendar {}", calendar_id)
            })?;
            let data: GoogleEventsResponse = resp
                .into_json()
                .context("failed to deserialize events response")?;
            out.extend(data.items);
            match data.next_page_token {
                Some(t) if !t.is_empty() => page_token = Some(t),
                _ => break,
            }
        }
        Ok(out)
    }
}

impl CalendarProvider for GoogleCalendarProvider {
    fn get_events(&self, date: NaiveDate, force_refresh: bool) -> Result<Vec<CalendarEvent>> {
        let calendars = self.get_calendars_cached(force_refresh)?;

        let start_of_day = date
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
            .and_local_timezone(Local)
            .single()
            .expect("valid local datetime");
        let end_of_day = start_of_day + chrono::Duration::hours(24);
        let time_min = start_of_day.to_rfc3339();
        let time_max = end_of_day.to_rfc3339();

        let mut events: Vec<CalendarEvent> = Vec::new();
        for cal in &calendars {
            // Filter by access role: exclude freeBusyReader (opaque busy blocks) and anything weird.
            let role_ok = matches!(
                cal.access_role.as_deref(),
                Some("owner") | Some("writer") | Some("reader")
            );
            if !role_ok {
                continue;
            }
            let ignored = self.config.calendars_to_ignore.contains(&cal.id)
                || cal
                    .summary
                    .as_ref()
                    .is_some_and(|s| self.config.calendars_to_ignore.contains(s));
            if ignored {
                continue;
            }

            let raw_events = self.fetch_events_for_calendar(&cal.id, &time_min, &time_max)?;
            for ge in raw_events {
                if let Some(ev) = convert_event(ge) {
                    events.push(ev);
                }
            }
        }

        Ok(events)
    }
}

fn convert_event(ge: GoogleEvent) -> Option<CalendarEvent> {
    if ge.status.as_deref() == Some("cancelled") {
        return None;
    }
    if let Some(attendees) = &ge.attendees
        && attendees.iter().any(|a| {
            a.is_self.unwrap_or(false) && a.response_status.as_deref() == Some("declined")
        })
    {
        return None;
    }

    let start = ge.start.as_ref()?;
    let end = ge.end.as_ref()?;

    let (start_time, end_time) = if let (Some(sd), Some(ed)) = (&start.date, &end.date) {
        // All-day event
        let start_date = NaiveDate::parse_from_str(sd, "%Y-%m-%d").ok()?;
        let end_date = NaiveDate::parse_from_str(ed, "%Y-%m-%d").ok()?;
        let start_dt = start_date
            .and_hms_opt(0, 0, 0)?
            .and_local_timezone(Local)
            .single()?;
        let end_dt = end_date
            .and_hms_opt(0, 0, 0)?
            .and_local_timezone(Local)
            .single()?;
        (start_dt, end_dt)
    } else if let (Some(sdt), Some(edt)) = (&start.date_time, &end.date_time) {
        // Timed event — RFC3339 with offset.
        let s = DateTime::parse_from_rfc3339(sdt).ok()?.with_timezone(&Local);
        let e = DateTime::parse_from_rfc3339(edt).ok()?.with_timezone(&Local);
        (s, e)
    } else {
        return None;
    };

    let title = ge
        .summary
        .clone()
        .unwrap_or_else(|| "(no title)".to_string());
    let attendees = ge
        .attendees
        .map(|v| v.into_iter().filter_map(|a| a.email).collect::<Vec<_>>())
        .unwrap_or_default();

    Some(CalendarEvent {
        id: ge.id,
        title,
        start_time,
        end_time,
        description: ge.description,
        location: ge.location,
        attendees,
    })
}

// Minimal percent-encoding for calendar IDs (emails have '@', etc.).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let is_unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn build_http_client() -> Result<ureq::Agent> {
    // oauth2 recommends disabling redirects on the HTTP client used for token exchange.
    Ok(ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(StdDuration::from_secs(30))
        .build())
}

fn build_oauth_client(
    client_id: &str,
    client_secret: &str,
    redirect_url: Option<String>,
) -> Result<BasicClient<
    oauth2::EndpointSet,  // HasAuthUrl
    oauth2::EndpointNotSet, // HasDeviceAuthUrl
    oauth2::EndpointNotSet, // HasIntrospectionUrl
    oauth2::EndpointNotSet, // HasRevocationUrl
    oauth2::EndpointSet,    // HasTokenUrl
>> {
    let mut client = BasicClient::new(ClientId::new(client_id.to_string()))
        .set_client_secret(ClientSecret::new(client_secret.to_string()))
        .set_auth_uri(AuthUrl::new(GOOGLE_AUTH_URL.to_string()).context("invalid auth URL")?)
        .set_token_uri(TokenUrl::new(GOOGLE_TOKEN_URL.to_string()).context("invalid token URL")?);
    if let Some(url) = redirect_url {
        client = client.set_redirect_uri(
            RedirectUrl::new(url).context("invalid redirect URL")?,
        );
    }
    Ok(client)
}

pub fn authenticate(config: &ProviderConfig) -> Result<()> {
    let client_id_var = config.env_oauth_client_id.as_deref().ok_or_else(|| {
        anyhow::anyhow!("google_calendar provider requires `env_oauth_client_id` in the config")
    })?;
    let client_secret_var = config.env_oauth_client_secret.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "google_calendar provider requires `env_oauth_client_secret` in the config"
        )
    })?;
    let client_id = read_env_var(client_id_var, "OAuth client ID")?;
    let client_secret = read_env_var(client_secret_var, "OAuth client secret")?;

    let listener = TcpListener::bind(("127.0.0.1", config.oauth_redirect_port))
        .context("failed to bind local loopback listener")?;
    let port = listener
        .local_addr()
        .context("failed to read listener address")?
        .port();
    let redirect_url = format!("http://127.0.0.1:{}", port);

    let client = build_oauth_client(&client_id, &client_secret, Some(redirect_url.clone()))?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(CALENDAR_SCOPE.to_string()))
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Opening browser to authorize agenda with Google Calendar...");
    if let Err(e) = webbrowser::open(auth_url.as_str()) {
        eprintln!("Could not auto-open browser ({}). Open this URL manually:", e);
        eprintln!("{}", auth_url);
    }

    let (mut stream, _addr) = listener
        .accept()
        .context("failed to accept redirect connection")?;
    stream
        .set_read_timeout(Some(StdDuration::from_secs(120)))
        .ok();
    stream
        .set_write_timeout(Some(StdDuration::from_secs(10)))
        .ok();

    let (code, state) = {
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .context("failed to read callback request")?;
        parse_redirect_query(request_line.trim())?
    };

    if state != *csrf_token.secret() {
        let _ = stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 19\r\nConnection: close\r\n\r\nCSRF state mismatch",
        );
        anyhow::bail!("CSRF state mismatch in OAuth callback");
    }

    let body = "<!doctype html><html><body style=\"font-family:system-ui;text-align:center;padding:4rem\"><h1>Authentication complete</h1><p>You can close this tab and return to your terminal.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    let http_client = build_http_client()?;
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request(&http_client)
        .map_err(|e| anyhow::anyhow!("failed to exchange authorization code: {}", e))?;

    let access_token = token_response.access_token().secret().clone();
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "token response did not include a refresh_token; revoke the app's access in your Google Account and retry"
            )
        })?;
    let expires_in = token_response
        .expires_in()
        .map(|d| d.as_secs() as i64)
        .unwrap_or(3600);
    let expires_at = Utc::now() + chrono::Duration::seconds(expires_in);
    let scope = token_response.scopes().map(|scopes| {
        scopes
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });

    let tokens = StoredTokens {
        access_token,
        refresh_token,
        expires_at,
        scope,
        token_type: "Bearer".to_string(),
    };
    let path = token_cache_path(&resolve_token_filename(config));
    save_tokens(&path, &tokens)?;
    println!("Authentication successful. Tokens stored at {}", path.display());
    Ok(())
}

fn parse_redirect_query(request_line: &str) -> Result<(String, String)> {
    // Example: "GET /?code=abc&state=xyz HTTP/1.1"
    let mut parts = request_line.split_whitespace();
    let _method = parts.next().ok_or_else(|| anyhow::anyhow!("empty request line"))?;
    let path = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing request path in callback"))?;
    let full = format!("http://127.0.0.1{}", path);
    let parsed = url::Url::parse(&full).context("failed to parse callback URL")?;
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;
    for (k, v) in parsed.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            "error" => error = Some(v.into_owned()),
            _ => {}
        }
    }
    if let Some(err) = error {
        anyhow::bail!("OAuth error from Google: {}", err);
    }
    let code = code.ok_or_else(|| anyhow::anyhow!("missing `code` in callback URL"))?;
    let state = state.ok_or_else(|| anyhow::anyhow!("missing `state` in callback URL"))?;
    Ok((code, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn timed_event_json() -> &'static str {
        r#"{
            "id": "evt1",
            "status": "confirmed",
            "summary": "Standup",
            "description": "Daily sync",
            "location": "Room 3",
            "start": { "dateTime": "2026-04-16T09:00:00-04:00", "timeZone": "America/New_York" },
            "end":   { "dateTime": "2026-04-16T09:30:00-04:00", "timeZone": "America/New_York" },
            "attendees": [
                { "email": "alice@example.com", "responseStatus": "accepted", "self": false },
                { "email": "me@example.com",    "responseStatus": "accepted", "self": true  }
            ]
        }"#
    }

    #[test]
    fn converts_timed_event() {
        let ge: GoogleEvent = serde_json::from_str(timed_event_json()).unwrap();
        let ev = convert_event(ge).expect("should convert");
        assert_eq!(ev.id, "evt1");
        assert_eq!(ev.title, "Standup");
        assert_eq!(ev.description.as_deref(), Some("Daily sync"));
        assert_eq!(ev.location.as_deref(), Some("Room 3"));
        // 30-minute event regardless of local TZ
        assert_eq!(
            (ev.end_time - ev.start_time).num_minutes(),
            30
        );
        assert_eq!(ev.attendees.len(), 2);
    }

    #[test]
    fn converts_all_day_event() {
        let json = r#"{
            "id": "allday1",
            "status": "confirmed",
            "summary": "Holiday",
            "start": { "date": "2026-04-16" },
            "end":   { "date": "2026-04-17" }
        }"#;
        let ge: GoogleEvent = serde_json::from_str(json).unwrap();
        let ev = convert_event(ge).expect("should convert");
        // Start is 00:00 local on 2026-04-16; end is 00:00 local on 2026-04-17.
        let expected_start = Local
            .with_ymd_and_hms(2026, 4, 16, 0, 0, 0)
            .single()
            .unwrap();
        let expected_end = Local
            .with_ymd_and_hms(2026, 4, 17, 0, 0, 0)
            .single()
            .unwrap();
        assert_eq!(ev.start_time, expected_start);
        assert_eq!(ev.end_time, expected_end);
    }

    #[test]
    fn skips_cancelled_event() {
        let json = r#"{
            "id": "c1",
            "status": "cancelled",
            "summary": "Cancelled meeting",
            "start": { "dateTime": "2026-04-16T09:00:00-04:00" },
            "end":   { "dateTime": "2026-04-16T10:00:00-04:00" }
        }"#;
        let ge: GoogleEvent = serde_json::from_str(json).unwrap();
        assert!(convert_event(ge).is_none());
    }

    #[test]
    fn skips_self_declined_event() {
        let json = r#"{
            "id": "d1",
            "status": "confirmed",
            "summary": "Declined by me",
            "start": { "dateTime": "2026-04-16T09:00:00-04:00" },
            "end":   { "dateTime": "2026-04-16T10:00:00-04:00" },
            "attendees": [
                { "email": "me@example.com", "responseStatus": "declined", "self": true },
                { "email": "a@example.com",  "responseStatus": "accepted", "self": false }
            ]
        }"#;
        let ge: GoogleEvent = serde_json::from_str(json).unwrap();
        assert!(convert_event(ge).is_none());
    }

    #[test]
    fn keeps_event_when_others_declined() {
        let json = r#"{
            "id": "k1",
            "status": "confirmed",
            "summary": "Others declined",
            "start": { "dateTime": "2026-04-16T09:00:00-04:00" },
            "end":   { "dateTime": "2026-04-16T10:00:00-04:00" },
            "attendees": [
                { "email": "me@example.com", "responseStatus": "accepted", "self": true },
                { "email": "a@example.com",  "responseStatus": "declined", "self": false }
            ]
        }"#;
        let ge: GoogleEvent = serde_json::from_str(json).unwrap();
        let ev = convert_event(ge).expect("should convert");
        assert_eq!(ev.attendees.len(), 2);
    }

    #[test]
    fn null_summary_becomes_no_title() {
        let json = r#"{
            "id": "n1",
            "status": "confirmed",
            "start": { "dateTime": "2026-04-16T09:00:00-04:00" },
            "end":   { "dateTime": "2026-04-16T10:00:00-04:00" }
        }"#;
        let ge: GoogleEvent = serde_json::from_str(json).unwrap();
        let ev = convert_event(ge).expect("should convert");
        assert_eq!(ev.title, "(no title)");
    }

    #[test]
    fn is_expired_respects_60s_skew() {
        let now = Utc::now();
        assert!(is_expired(now + chrono::Duration::seconds(30), now));
        assert!(is_expired(now, now));
        assert!(!is_expired(now + chrono::Duration::seconds(120), now));
    }

    #[test]
    fn stored_tokens_roundtrip_json() {
        let original = StoredTokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: Utc.with_ymd_and_hms(2026, 4, 16, 12, 0, 0).unwrap(),
            scope: Some(CALENDAR_SCOPE.to_string()),
            token_type: "Bearer".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: StoredTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, original.access_token);
        assert_eq!(parsed.refresh_token, original.refresh_token);
        assert_eq!(parsed.scope.as_deref(), Some(CALENDAR_SCOPE));
    }

    #[test]
    fn parse_redirect_query_extracts_code_and_state() {
        let line = "GET /?code=abc123&state=xyz HTTP/1.1";
        let (code, state) = parse_redirect_query(line).unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz");
    }

    #[test]
    fn parse_redirect_query_reports_oauth_error() {
        let line = "GET /?error=access_denied&state=x HTTP/1.1";
        let err = parse_redirect_query(line).unwrap_err();
        assert!(err.to_string().contains("access_denied"));
    }

    #[test]
    fn urlencoding_encodes_special_chars() {
        assert_eq!(urlencoding_encode("a@b.com"), "a%40b.com");
        assert_eq!(urlencoding_encode("plain"), "plain");
    }
}
