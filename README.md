# Agenda CLI

A command-line tool that pulls calendar events from various calendar providers and formats them for easy copy-pasting into markdown documents.

## Why?

I currently use [Obsidian](https://obsidian.md) to take notes. Obsidian uses markdown files, and part of the notes I take include a section for my meetings each day. In a [blog post](https://jnsgr.uk/2024/07/how-i-computer-in-2024/) by Jon Seager, he mentions:

> The agenda and the links are automatically generated using a small Go application I wrote - this application scrapes my Google Calendar, and according to some rules and the knowledge it has of my vault, generates the Markdown for the agenda and copies it to the clipboard. Each day, I sit down and type agenda at the command line, then paste into Obsidian.

I really liked this idea, so I decided to create my own version of this tool, written in Rust.

## Features

- Support for multiple calendar providers (currently Morgen.so, extensible for others)
- Configurable time formatting (strftime syntax)
- Customizable event templates using Handlebars
- Configuration via TOML file
- Environment variable support for API keys
- Clean markdown output

## Installation

### Build from Source

1. Clone the repository
2. Install [Rust](https://rustup.rs/) if you haven't already
3. Optionally install [just](https://github.com/casey/just)
4. Build: `just build` or `cargo build --release`
5. Install: `just install` or `cargo install --path .`

## Quick Start

1. Initialize a default configuration:

   ```bash
   agenda init
   ```

2. Set your API key:

   ```bash
   export MORGEN_API_KEY="your-morgen-api-key"
   ```

3. Run:

   ```bash
   agenda
   ```

## Configuration

The configuration file is at `~/.config/agenda/config.toml` (Linux/macOS) or `%APPDATA%\agenda\config.toml` (Windows).

> **Migrating from the Go version?** Re-run `agenda init` to create a new TOML config. The old YAML config is not compatible.

### Sample Configuration

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

### Configuration Options

| Option           | Type   | Description                                   | Example                                                      |
|------------------|--------|-----------------------------------------------|--------------------------------------------------------------|
| `provider`       | string | Which calendar provider to use                | `"morgen"`                                                   |
| `time_format`    | string | strftime format string for displaying times   | `"%H:%M"`, `"%I:%M %p"`                                     |
| `event_template` | string | Handlebars template for formatting each event | `"- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}"` |
| `config_version` | int    | Internal config version (do not change)       | `1`                                                          |

### Event Template Fields

| Field                    | Description                      |
|--------------------------|----------------------------------|
| `{{Title}}`              | Event title                      |
| `{{StartTimeFormatted}}` | Start time (per `time_format`)   |
| `{{EndTimeFormatted}}`   | End time (per `time_format`)     |
| `{{Duration}}`           | Duration (e.g. `1h30m`)          |
| `{{Description}}`        | Event description (may be empty) |
| `{{Location}}`           | Event location (may be empty)    |

#### Example Templates

```toml
# Simple
event_template = "- {{StartTimeFormatted}}: {{Title}}"

# With end time
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}"

# With duration
event_template = "- {{StartTimeFormatted}} ({{Duration}}): {{Title}}"

# Bold title
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: **{{Title}}**"
```

## Command Line Options

| Option                    | Description                                          |
|---------------------------|------------------------------------------------------|
| `--config PATH`           | Custom configuration file path                       |
| `--provider NAME`         | Override the provider from config                    |
| `--time-format FORMAT`    | Override the time format (strftime)                  |
| `--event-template TMPL`   | Override the event template                          |
| `--verbose` / `-v`        | Enable verbose logging                               |
| `--date DATE`             | Date to fetch (YYYY-MM-DD, default: today)           |

## Environment Variables

- `MORGEN_API_KEY` — Your Morgen.so API key

## Provider: Morgen.so

1. Sign up for an API key at [https://platform.morgen.so](https://platform.morgen.so/)
2. Go to **Developers API**
3. Generate and copy your API key
4. `export MORGEN_API_KEY="your-key"`

## Adding New Providers

Implement the `CalendarProvider` trait in a new file under `src/providers/`:

```rust
pub trait CalendarProvider: Send {
    fn name(&self) -> &str;
    fn get_events(&self, date: NaiveDate) -> anyhow::Result<Vec<CalendarEvent>>;
}
```

Then register the provider in `create_provider()` in `src/providers/mod.rs`.

## Output Example

```
- 09:00-10:00: Team Standup
- 10:30-11:30: Project Review
- 14:00-15:00: Client Call
- 16:00-16:30: 1:1 with Manager
```

## License

MIT — see [LICENSE](LICENSE) for details.

## Author

| [<img src="https://avatars0.githubusercontent.com/u/6591180?s=460&v=4" width="100"><br><sub>@ptsouchlos</sub>](https://github.com/ptsouchlos) |
| :-------------------------------------------------------------------------------------------------------------------------------------------------------: |
