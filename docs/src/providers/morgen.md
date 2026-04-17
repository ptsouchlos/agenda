# Morgen.so

[Morgen.so](https://morgen.so) is a cross-platform calendar app that provides an API for reading events.

## Setup

1. Sign up at [platform.morgen.so](https://platform.morgen.so/)
2. Go to **Developers API**
3. Generate and copy your API key
4. Export the key in your shell:

   ```bash
   export MORGEN_API_KEY="your-api-key"
   ```

   Or add it to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to persist it.

## Configuration

```toml
provider = "morgen"

[providers.morgen]
base_url = "https://api.morgen.so/v3"
env_api_key = "MORGEN_API_KEY"
calendars_to_ignore = []

[providers.morgen.headers]
Authorization = "ApiKey {API_KEY}"
Content-Type = "application/json"
```

### Options

| Option                | Description                                      |
|-----------------------|--------------------------------------------------|
| `base_url`            | Morgen API base URL (use the default)            |
| `env_api_key`         | Environment variable name that holds the API key |
| `calendars_to_ignore` | List of calendar names to exclude from output    |
| `headers`             | HTTP headers sent with each request              |

## Ignoring Calendars

Add calendar names to `calendars_to_ignore` to exclude them:

```toml
[providers.morgen]
calendars_to_ignore = ["Holidays in United States", "Birthdays"]
```
