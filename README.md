# Agenda CLI

A command-line tool that pulls calendar events from various calendar providers and formats them for easy copy-pasting into markdown documents.

## Why?

I currently use [Obsidian](https://obsidian.md) to take notes. Obsidian uses markdown files, and part of the notes I take include a section for my meetings each day. In a [blog post](https://jnsgr.uk/2024/07/how-i-computer-in-2024/) by Jon Seager, he mentions:

> The agenda and the links are automatically generated using a small Go application I wrote - this application scrapes my Google Calendar, and according to some rules and the knowledge it has of my vault, generates the Markdown for the agenda and copies it to the clipboard. Each day, I sit down and type agenda at the command line, then paste into Obsidian.

I really liked this idea, so I decided to create my own version of this tool. It is written in Go and can be extended to support multiple calendar providers.

## Features

- Support for multiple calendar providers (currently Morgen.so, extensible for others)
- Configurable time formatting
- Customizable event templates using Go templates
- Configuration via YAML file
- Environment variable support for API keys
- Clean markdown output

## Installation

### Build from Source

1. Clone or download the source code
2. Install [Go](https://golang.org/dl/) if you haven't already

- You can also optionally install [just](https://github.com/casey/just)

3. Build the application: `just build` or `go build`
4. Install using `go install`

## Quick Start

1. Initialize a default configuration:

   ```bash
   ./agenda init
   ```

2. Set your API key in the environment:

   ```bash
   export MORGEN_API_KEY="your-morgen-api-key"
   ```

3. Run the agenda:
   ```bash
   ./agenda
   ```

## Configuration

The configuration file is located at `~/.config/agenda/config.yaml` by default.

### Example Configuration

```yaml
provider: morgen
time_format: "15:04"
event_template: "- {{.StartTimeFormatted}}-{{.EndTimeFormatted}}: {{.Title}}"

providers:
  morgen:
    base_url: "https://api.morgen.so/v1"
    headers:
      Authorization: "Bearer {API_KEY}"
      Content-Type: "application/json"
    env_api_key: "MORGEN_API_KEY"
    calendars_to_ignore:
      - "Work"
      - "Extra Calendar"
```

### Configuration Options

| Option           | Type   | Description                                | Example                                                       |
| ---------------- | ------ | ------------------------------------------ | ------------------------------------------------------------- |
| `provider`       | string | Which calendar provider to use             | "morgen"                                                      |
| `time_format`    | string | Go time format string for displaying times | "15:04", "3:04 PM"                                            |
| `event_template` | string | Go template string for formatting events   | "- {{.StartTimeFormatted}}-{{.EndTimeFormatted}}: {{.Title}}" |

#### Event Template Fields

| Field                     | Description                       |
| ------------------------- | --------------------------------- |
| `{{.StartTimeFormatted}}` | Formatted start time of the event |
| `{{.EndTimeFormatted}}`   | Formatted end time of the event   |
| `{{.Title}}`              | Title of the event                |
| `{{.Duration}}`           | Duration of the event             |
| `{{.Description}}`        | Description of the event          |

##### Example Templates

```yaml
# Simple format
event_template: "- {{.StartTimeFormatted}}: {{.Title}}"

# Detailed format with location
event_template: "- {{.StartTimeFormatted}}-{{.EndTimeFormatted}}: **{{.Title}}**"

# With duration
event_template: "- {{.StartTimeFormatted}} ({{.Duration}}): {{.Title}}"
```

### Provider Configuration Options

| Field                 | Type              | Description                                                                  |
| --------------------- | ----------------- | ---------------------------------------------------------------------------- |
| `base_url`            | string            | Base URL for the API                                                         |
| `headers`             | map[string]string | HTTP headers to include in requests (e.g., for authentication with API keys) |
| `env_api_key`         | string            | Environment variable name for the API key                                    |
| `calendars_to_ignore` | list              | List of calendar names to ignore when fetching events                        |

## Command Line Options

| Option                     | Description                                                    |
| -------------------------- | -------------------------------------------------------------- |
| `-config PATH`             | Specify a custom configuration file path                       |
| `-provider NAME`           | Override the provider from config                              |
| `-time-format FORMAT`      | Override the time format from config                           |
| `-event-template TEMPLATE` | Override the event template from config                        |
| `-verbose`                 | Enable verbose logging                                         |
| `-date DATE`               | Specify a date to fetch events for. Use the format YYYY-MM-DD. |

## Environment Variables

- `MORGEN_API_KEY` - Your Morgen.so API key

## Adding New Providers

To add support for a new calendar provider:

1. Implement the `CalendarProvider` interface:

   ```go
   type CalendarProvider interface {
       GetTodaysEvents() ([]CalendarEvent, error)
       GetName() string
   }
   ```

2. Add the provider to the `CreateProvider` function in the `ProviderFactory`
3. Add the provider configuration to the default config

## Output Example

```markdown
- 09:00-10:00 Team Standup
- 10:30-11:30 Project Review
- 14:00-15:00 Client Call
- 16:00-16:30 1:1 with Manager
```

## Provider Specific Setup

### Morgen.so

1. Sign up for an API key at [https://platform.morgen.so](https://platform.morgen.so/)
2. Go to `Developers API`
3. Generate and copy your API key
4. Set it as the value for the `MORGEN_API_KEY` environment variable

## License

The project is licensed under the MIT license. See [LICENSE](LICENSE) for more details.

## Author

| [<img src="https://avatars0.githubusercontent.com/u/6591180?s=460&v=4" width="100"><br><sub>@ptsouchlos</sub>](https://github.com/ptsouchlos) |
| :-------------------------------------------------------------------------------------------------------------------------------------------------------: |
