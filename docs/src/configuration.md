# Configuration

The configuration file lives at:

- **Linux/macOS:** `~/.config/agenda/config.toml`
- **Windows:** `%APPDATA%\agenda\config.toml`

Run `agenda init` to create a default config.

## Top-Level Options

| Option           | Type   | Description                                 |
|------------------|--------|---------------------------------------------|
| `provider`       | string | Which calendar provider to use              |
| `time_format`    | string | strftime format string for displaying times |
| `event_template` | string | Handlebars template for each event          |
| `config_version` | int    | Internal version — do not change            |

## Sample Configuration

```toml
provider = "morgen"
time_format = "%H:%M"
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}"
config_version = 1

[providers.morgen]
base_url = "https://api.morgen.so/v3"
env_api_key = "MORGEN_API_KEY"
calendars_to_ignore = ["ignore_this_calendar"]

[providers.morgen.headers]
Authorization = "ApiKey {API_KEY}"
Content-Type = "application/json"
```
