# Google Calendar

Google Calendar uses OAuth 2.0. You'll need to create a Google Cloud project and OAuth credentials before running `agenda auth google_calendar`.

## 1. Create a Google Cloud Project

1. Go to the [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project (or select an existing one)
3. In the left sidebar, go to **APIs & Services → Library**
4. Search for **Google Calendar API** and enable it

## 2. Create OAuth Credentials

1. Go to **APIs & Services → Credentials**
2. Click **Create Credentials → OAuth client ID**
3. If prompted, configure the OAuth consent screen first:
   - Choose **External** (unless you have a Google Workspace org)
   - Fill in the required app name and email fields
   - Add the scope `https://www.googleapis.com/auth/calendar.readonly`
   - Add your own email as a test user
4. For application type, choose **Desktop app**
5. Copy the **Client ID** and **Client Secret**

## 3. Set Environment Variables

```bash
export GOOGLE_CALENDAR_CLIENT_ID="your-client-id"
export GOOGLE_CALENDAR_CLIENT_SECRET="your-client-secret"
```

Add these to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to persist them.

## 4. Authenticate

```bash
agenda auth google_calendar
```

This opens a browser window. Sign in and grant access. A success page will appear and tokens will be saved to `~/.cache/agenda/google_calendar_tokens.json`.

## 5. Run

Update your config to use `google_calendar` as the provider and run:

```bash
agenda
```

## Configuration

```toml
provider = "google_calendar"

[providers.google_calendar]
base_url = "https://www.googleapis.com/calendar/v3"
env_oauth_client_id = "GOOGLE_CALENDAR_CLIENT_ID"
env_oauth_client_secret = "GOOGLE_CALENDAR_CLIENT_SECRET"
oauth_redirect_port = 0
calendars_to_ignore = []
```

### Options

| Option                  | Description                                                       |
|-------------------------|-------------------------------------------------------------------|
| `env_oauth_client_id`   | Environment variable name that holds the OAuth client ID          |
| `env_oauth_client_secret` | Environment variable name that holds the OAuth client secret    |
| `oauth_redirect_port`   | Local port for the OAuth redirect (0 = random available port)     |
| `calendars_to_ignore`   | List of calendar names or IDs to exclude from output              |
| `calendar_cache_ttl_seconds` | How long to cache the calendar list in seconds (default: 86400) |

## Re-authenticating

If your refresh token is revoked (e.g. after revoking access in your Google Account), run:

```bash
agenda auth google_calendar
```

## Ignoring Calendars

You can ignore calendars by name or by calendar ID:

```toml
[providers.google_calendar]
calendars_to_ignore = ["Holidays in United States", "someone@example.com"]
```
