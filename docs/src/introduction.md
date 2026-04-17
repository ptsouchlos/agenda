# Agenda CLI

A command-line tool that pulls calendar events from various calendar providers and formats them for easy copy-pasting into markdown documents.

## Why?

Inspired by [Jon Seager's blog post](https://jnsgr.uk/2024/07/how-i-computer-in-2024/), Agenda is designed for people who take notes in markdown (e.g. Obsidian) and want their daily calendar events automatically formatted and ready to paste.

## Features

- Support for multiple calendar providers (Morgen.so, Google Calendar)
- Configurable time formatting (strftime syntax)
- Customizable event templates using Handlebars
- Configuration via TOML file

## Example Output

```
- 09:00-10:00: Team Standup
- 10:30-11:30: Project Review
- 14:00-15:00: Client Call
- 16:00-16:30: 1:1 with Manager
```
