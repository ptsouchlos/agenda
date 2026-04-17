# CLI Options

## Global Options

| Option                  | Description                                        |
|-------------------------|----------------------------------------------------|
| `--config PATH`         | Custom configuration file path                     |
| `--provider NAME`       | Override the provider from config                  |
| `--time-format FORMAT`  | Override the time format (strftime)                |
| `--event-template TMPL` | Override the event template (Handlebars)           |
| `--verbose` / `-v`      | Enable verbose logging                             |
| `--date DATE`           | Date to fetch events for (YYYY-MM-DD, default: today) |

## Subcommands

| Command                    | Description                                   |
|----------------------------|-----------------------------------------------|
| `agenda`                   | Fetch and print today's events                |
| `agenda init`              | Write a default config file                   |
| `agenda auth <provider>`   | Authenticate with a provider (OAuth flows)    |
| `agenda version`           | Print version information                     |

## Examples

```bash
# Fetch today's events
agenda

# Fetch events for a specific date
agenda --date 2026-04-20

# Use a different provider without editing config
agenda --provider morgen

# Override time format inline
agenda --time-format "%I:%M %p"

# Authenticate with Google Calendar
agenda auth google_calendar
```
