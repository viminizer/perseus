# Environment Variables

Perseus supports named environments with key-value variable pairs and `{{variable}}` substitution in all request fields. Create environment files as JSON, switch between them with `Ctrl+N`, and see the active environment in the status bar.

## Overview

| Concept | Description |
|---------|-------------|
| **Environment** | A named set of key-value pairs (e.g., `dev`, `staging`, `production`) |
| **Variable** | A key-value pair within an environment (e.g., `base_url` = `http://localhost:3000`) |
| **Substitution** | `{{variable_name}}` placeholders in request fields are replaced with values at send time |
| **Quick-switch** | `Ctrl+N` opens a popup to change the active environment from any mode |

Variables are resolved when you press `Ctrl+R` to send a request. The editor always shows raw `{{var}}` templates — substitution only happens in the outgoing request.

## Getting Started

### 1. Create an Environment File

Environment files live in `.perseus/environments/` inside your project root. Each file is a standalone JSON file named after the environment.

Create `.perseus/environments/dev.json`:

```json
{
  "name": "dev",
  "values": [
    {
      "key": "base_url",
      "value": "http://localhost:3000",
      "enabled": true,
      "type": "default"
    },
    {
      "key": "api_token",
      "value": "dev-token-abc123",
      "enabled": true,
      "type": "default"
    }
  ]
}
```

Create `.perseus/environments/staging.json`:

```json
{
  "name": "staging",
  "values": [
    {
      "key": "base_url",
      "value": "https://api.staging.example.com",
      "enabled": true,
      "type": "default"
    },
    {
      "key": "api_token",
      "value": "staging-token-xyz789",
      "enabled": true,
      "type": "default"
    }
  ]
}
```

### 2. Select an Environment

1. Launch Perseus (environments are loaded automatically at startup)
2. Press `Ctrl+N` to open the environment switcher popup
3. Use `j`/`k` or arrow keys to highlight an environment
4. Press `Enter` to activate it

The active environment name appears in the status bar as a blue badge.

### 3. Use Variables in Requests

Type `{{variable_name}}` anywhere in a request field:

- **URL:** `{{base_url}}/api/v1/users`
- **Headers:** `Authorization: Bearer {{api_token}}`
- **Body:** `{"server": "{{base_url}}", "key": "{{api_key}}"}`
- **Auth fields:** Put `{{api_token}}` in a Bearer token field

When you send the request (`Ctrl+R`), Perseus replaces all `{{variables}}` with values from the active environment before sending.

## Environment File Format

Environment files use a Postman-compatible JSON schema. Each file contains a single environment with a name and an array of variables.

### Schema

```json
{
  "name": "<environment-name>",
  "values": [
    {
      "key": "<variable-name>",
      "value": "<variable-value>",
      "enabled": true,
      "type": "default"
    }
  ]
}
```

### Field Reference

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Environment name (must match the filename stem) |
| `values` | array | no | `[]` | List of variable definitions |
| `values[].key` | string | yes | — | Variable name used in `{{key}}` placeholders |
| `values[].value` | string | yes | — | Replacement value |
| `values[].enabled` | boolean | no | `true` | Whether this variable is active for substitution |
| `values[].type` | string | no | `"default"` | Variable type (for Postman compatibility; use `"default"` or `"secret"`) |

### Naming Rules

Environment names (and therefore filenames) must contain only:

- Alphanumeric characters (`a-z`, `A-Z`, `0-9`)
- Underscores (`_`)
- Hyphens (`-`)

Valid: `dev`, `staging`, `production`, `my-local`, `team_shared`
Invalid: `my env` (space), `dev.local` (dot), `env/test` (slash)

### File Location

```
{project_root}/
└── .perseus/
    ├── collection.json          # Existing — requests and folders
    ├── config.toml              # Existing — project configuration
    └── environments/            # Environment files
        ├── dev.json
        ├── staging.json
        └── production.json
```

Each `.json` file in the `environments/` directory is loaded as a separate environment. The filesystem acts as the index — no registry file is needed.

## Substitution

### How It Works

When you press `Ctrl+R` to send a request, Perseus:

1. Reads the raw text from the URL, headers, body, and auth fields
2. Looks up the active environment's enabled variables
3. Scans each field for `{{variable_name}}` patterns
4. Replaces matched patterns with variable values
5. Sends the resolved request

The editor always displays the raw template text. Substitution only affects the outgoing HTTP request.

### Substitution Scope

Variables are substituted in all user-editable request fields:

| Field | Example Template | Resolved Value |
|-------|-----------------|----------------|
| URL | `{{base_url}}/api/users` | `http://localhost:3000/api/users` |
| Headers | `X-Api-Key: {{api_key}}` | `X-Api-Key: sk-abc123` |
| Body | `{"token": "{{auth_token}}"}` | `{"token": "my-secret-token"}` |
| Bearer Token | `{{api_token}}` | `dev-token-abc123` |
| Basic Username | `{{username}}` | `admin` |
| Basic Password | `{{password}}` | `secret` |
| API Key Name | `{{key_header}}` | `X-Custom-Auth` |
| API Key Value | `{{key_value}}` | `key-12345` |

Method and response fields are **not** substituted.

### Unresolved Variables

If a `{{variable}}` has no matching key in the active environment (or no environment is selected), it is left as literal text in the sent request. This matches Postman behavior and is non-blocking — the request still sends.

For example, with only `base_url` defined:

```
Template:  {{base_url}}/api/{{version}}/users
Sent as:   http://localhost:3000/api/{{version}}/users
```

This makes it easy to spot missing variables in the response or URL bar.

### Disabled Variables

Variables with `"enabled": false` are excluded from substitution. Use this to temporarily disable a variable without deleting it from the file:

```json
{
  "key": "debug_token",
  "value": "super-secret",
  "enabled": false,
  "type": "default"
}
```

`{{debug_token}}` will remain as literal text in sent requests until you set `enabled` back to `true`.

### Edge Cases

| Input | Result | Reason |
|-------|--------|--------|
| `{{}}` | `{{}}` | Empty variable name — left as literal |
| `{{name` | `{{name` | Unclosed braces — left as literal |
| `{{a}}{{b}}` | Both resolved | Adjacent variables are handled correctly |
| `{{a}}` with no env | `{{a}}` | No environment selected — left as literal |
| `{{a}}` where `a` resolves to `{{b}}` | Value of `a` (no re-scan) | Single-pass substitution; no nested resolution |

## Environment Switching

### Quick-Switch Popup

Press `Ctrl+N` from any mode (Navigation, Editing, or Sidebar) to open the environment switcher popup.

```
┌─ Environment ──────────────┐
│ ✓ No Environment           │
│   dev                      │
│   production               │
│   staging                  │
└────────────────────────────┘
```

- A checkmark (`✓`) marks the currently active environment
- The highlighted item is shown with inverted colors
- "No Environment" disables all variable substitution

### Popup Controls

| Key | Action |
|-----|--------|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `Enter` | Activate the selected environment |
| `Esc` / `q` | Close without changing |

The popup closes automatically when you press `Enter` or `Esc`. Only one popup can be open at a time — opening the environment popup closes any other open popup (method, auth type).

### Status Bar Indicator

When an environment is active, its name appears as a blue badge in the status bar:

```
 NAVIGATION  Request > URL  │  hjkl:nav ...  │   dev
```

When no environment is selected, the indicator is hidden.

## Practical Examples

### Multi-Environment API Development

Create three environments for a typical development workflow:

**`.perseus/environments/local.json`**
```json
{
  "name": "local",
  "values": [
    { "key": "base_url", "value": "http://localhost:3000", "enabled": true, "type": "default" },
    { "key": "api_key", "value": "dev-key-local", "enabled": true, "type": "default" }
  ]
}
```

**`.perseus/environments/staging.json`**
```json
{
  "name": "staging",
  "values": [
    { "key": "base_url", "value": "https://api.staging.example.com", "enabled": true, "type": "default" },
    { "key": "api_key", "value": "staging-key-abc123", "enabled": true, "type": "default" }
  ]
}
```

**`.perseus/environments/production.json`**
```json
{
  "name": "production",
  "values": [
    { "key": "base_url", "value": "https://api.example.com", "enabled": true, "type": "default" },
    { "key": "api_key", "value": "prod-key-xyz789", "enabled": true, "type": "default" }
  ]
}
```

Set your URL to `{{base_url}}/api/v1/users` and headers to `X-Api-Key: {{api_key}}`. Switch between environments with `Ctrl+N` — each request hits the right server with the right credentials.

### Auth Token Rotation

Store auth tokens in environment variables to update them in one place:

```json
{
  "name": "dev",
  "values": [
    { "key": "base_url", "value": "http://localhost:3000", "enabled": true, "type": "default" },
    { "key": "access_token", "value": "eyJhbGciOiJIUzI1NiIs...", "enabled": true, "type": "default" }
  ]
}
```

In the Auth tab, select Bearer Token and set the token field to `{{access_token}}`. When the token expires, update the value in `dev.json` and restart Perseus — all requests using `{{access_token}}` pick up the new value.

### Shared Team Environments

Environment files are plain JSON in the `.perseus/` directory and can be committed to version control:

```
# .gitignore
.perseus/environments/local.json    # Personal env — don't commit
```

```
# Committed to git
.perseus/environments/dev.json       # Team dev environment
.perseus/environments/staging.json   # Shared staging config
```

Team members get the shared environments automatically when they pull the repository. Personal environments stay local.

## Postman Compatibility

The environment file format is compatible with Postman's environment schema. This means:

- Postman environment exports can be placed directly in `.perseus/environments/` and used by Perseus
- Perseus environment files can be imported into Postman as environments
- The `key`, `value`, `enabled`, and `type` fields are preserved in both directions

To use a Postman environment export:

1. Export the environment from Postman (Settings > Environments > Export)
2. Copy the exported `.json` file into `.perseus/environments/`
3. Ensure the `"name"` field matches the filename (e.g., `dev.json` contains `"name": "dev"`)
4. Restart Perseus

## Keyboard Reference

| Context | Key | Action |
|---------|-----|--------|
| Any mode | `Ctrl+N` | Toggle environment switcher popup |
| Env popup | `j` / `Down` | Move selection down |
| Env popup | `k` / `Up` | Move selection up |
| Env popup | `Enter` | Activate selected environment |
| Env popup | `Esc` / `q` | Close popup without changing |
| Any mode | `Ctrl+R` | Send request (variables are substituted) |

## Limitations

These limitations are intentional for v1 and may be addressed in future versions:

| Limitation | Current Behavior | Workaround |
|------------|-----------------|------------|
| No in-app environment editing | Edit JSON files directly | Terminal users can edit `.perseus/environments/*.json` in any text editor |
| No global variables | Each environment is independent | Create a "shared" or "globals" environment with common values |
| No session persistence of active env | Active environment resets to "None" on restart | Press `Ctrl+N` once after launching |
| No nested substitution | `{{a}}` values are not re-scanned for `{{b}}` patterns | Flatten variable references |
| No dynamic variables | No `{{$timestamp}}` or `{{$randomUUID}}` support | Compute values externally and paste them into the environment file |
| No variable autocomplete | Typing `{{` does not show suggestions | Reference the environment file for variable names |
| String values only | All variable values are treated as strings | Consistent with Postman; sufficient for URL/header/body text |
