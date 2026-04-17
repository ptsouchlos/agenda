# Event Templates

Event output is controlled by the `event_template` config option, which uses [Handlebars](https://handlebarsjs.com/) syntax.

## Available Fields

| Field                    | Description                      |
|--------------------------|----------------------------------|
| `{{Title}}`              | Event title                      |
| `{{StartTimeFormatted}}` | Start time (per `time_format`)   |
| `{{EndTimeFormatted}}`   | End time (per `time_format`)     |
| `{{Duration}}`           | Duration (e.g. `1h30m`)          |
| `{{Description}}`        | Event description (may be empty) |
| `{{Location}}`           | Event location (may be empty)    |

## Examples

```toml
# Simple — just start time and title
event_template = "- {{StartTimeFormatted}}: {{Title}}"

# With end time
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}}"

# With duration
event_template = "- {{StartTimeFormatted}} ({{Duration}}): {{Title}}"

# Bold title
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: **{{Title}}**"

# Include location when present
event_template = "- {{StartTimeFormatted}}-{{EndTimeFormatted}}: {{Title}} @ {{Location}}"
```

## Time Format

The `time_format` option uses [strftime](https://strftime.org/) syntax and applies to both `{{StartTimeFormatted}}` and `{{EndTimeFormatted}}`.

| Format | Example  | Description       |
|--------|----------|-------------------|
| `%H:%M` | `14:30` | 24-hour clock     |
| `%I:%M %p` | `2:30 PM` | 12-hour clock |
| `%H:%M:%S` | `14:30:00` | With seconds |
