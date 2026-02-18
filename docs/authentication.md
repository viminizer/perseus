# Authentication

Perseus supports per-request authentication, allowing you to attach credentials to individual API requests. Auth settings are configured through a dedicated **Auth tab** in the request panel and are automatically injected into outgoing requests.

## Supported Auth Types

| Type | Description | How it's sent |
|------|-------------|---------------|
| **No Auth** | No authentication (default) | Nothing added to the request |
| **Bearer Token** | OAuth 2.0 / JWT token | `Authorization: Bearer <token>` header |
| **Basic Auth** | Username and password | `Authorization: Basic <base64>` header |
| **API Key** | Custom key-value pair | Header or query parameter (configurable) |

## Getting Started

### Opening the Auth Tab

The Auth tab sits between the Headers and Body tabs in the request panel:

```
Headers | Auth | Body
```

Navigate to the Auth tab using:

- `Ctrl+L` or `Ctrl+H` to cycle between request tabs (Headers, Auth, Body)
- `Tab` to switch panels, then navigate to the Auth tab

The tab label dynamically reflects the active auth type (e.g., `Auth (Bearer)`, `Auth (Basic)`).

### Selecting an Auth Type

1. Navigate to the Auth tab — the `Type: [No Auth]` selector is at the top
2. Press `Enter` on the type selector to open the auth type popup
3. Use `j`/`k` or arrow keys to highlight a type
4. Press `Enter` to confirm, or `Esc` to cancel

When you select a new auth type, the previous type's fields are cleared and the cursor moves to the first editable field.

## Auth Type Details

### No Auth

The default state. Displays "No authentication configured" and sends no auth-related headers or parameters. Use this to explicitly disable authentication for a request.

### Bearer Token

For OAuth 2.0 access tokens, JWTs, or any token-based authentication.

**Fields:**

| Field | Description |
|-------|-------------|
| Token | The bearer token value |

**What happens at send time:**

Perseus adds the header:
```
Authorization: Bearer <your-token>
```

**Example usage:** Authenticating with a GitHub API personal access token, an OAuth 2.0 access token, or a JWT issued by your auth server.

### Basic Auth

For HTTP Basic authentication using a username and password.

**Fields:**

| Field | Description |
|-------|-------------|
| Username | The authentication username |
| Password | The authentication password |

**What happens at send time:**

Perseus Base64-encodes the `username:password` pair and adds the header:
```
Authorization: Basic <base64-encoded-credentials>
```

**Example usage:** Authenticating with APIs that require HTTP Basic auth, such as private package registries, legacy REST APIs, or services behind basic auth proxies.

### API Key

For services that authenticate via a custom key-value pair sent as a header or query parameter.

**Fields:**

| Field | Description |
|-------|-------------|
| Key | The parameter name (e.g., `X-API-Key`, `api_key`) |
| Value | The parameter value (your API key) |
| Add to | Where to send the key — `Header` or `Query Param` |

**What happens at send time:**

- **Header mode:** Perseus adds a custom header with your key and value:
  ```
  X-API-Key: your-api-key-value
  ```
- **Query Param mode:** Perseus appends the key-value pair to the URL query string:
  ```
  https://api.example.com/data?api_key=your-api-key-value
  ```

To toggle the location between Header and Query Param, navigate to the `Add to: [Header]` field and press `Enter`.

**Example usage:** Authenticating with services like OpenAI (`Authorization` header), Google Maps (`key` query param), or any API that uses custom API key headers.

## Navigation and Editing

### Navigating Auth Fields

Within the Auth tab, fields are arranged vertically. Navigate between them using:

| Key | Action |
|-----|--------|
| `j` / `Down` | Move to the next field |
| `k` / `Up` | Move to the previous field |

Navigation wraps at the boundaries: pressing `k` on the type selector moves focus to the URL bar above; pressing `j` past the last field moves focus to the response panel below.

### Editing Text Fields

Auth text fields (Token, Username, Password, Key, Value) use the same vim-based editing as the rest of Perseus:

1. Navigate to a text field (it highlights green when focused)
2. Press `Enter` or `i` to enter editing mode
3. Edit using vim keybindings (insert mode, normal mode, visual mode)
4. Press `Esc` to exit back to navigation mode

All standard vim operations work in auth fields: word motions (`w`, `b`, `e`), text objects (`ciw`, `diw`), yank/paste (`y`, `p`), visual selection (`v`), and clipboard integration (`Ctrl+C` to copy, `Ctrl+V` to paste).

### The Type Selector and Location Toggle

The `Type: [...]` selector and `Add to: [...]` toggle are not text fields — they open popups or cycle values when you press `Enter`:

- **Type selector:** Opens a popup list to choose the auth type
- **Location toggle:** Cycles between `Header` and `Query Param`

## Persistence

Auth settings are saved as part of the Postman Collection v2.1 format used by Perseus for request storage. When you save a request, its auth configuration is persisted alongside the method, URL, headers, and body.

### Storage Format

Auth data is stored in the `auth` field of each request in the collection JSON file:

**Bearer Token:**
```json
{
  "auth": {
    "type": "bearer",
    "bearer": [
      { "key": "token", "value": "your-token-here", "type": "string" }
    ]
  }
}
```

**Basic Auth:**
```json
{
  "auth": {
    "type": "basic",
    "basic": [
      { "key": "username", "value": "your-username", "type": "string" },
      { "key": "password", "value": "your-password", "type": "string" }
    ]
  }
}
```

**API Key:**
```json
{
  "auth": {
    "type": "apikey",
    "apikey": [
      { "key": "key", "value": "X-API-Key", "type": "string" },
      { "key": "value", "value": "your-api-key", "type": "string" },
      { "key": "in", "value": "header", "type": "string" }
    ]
  }
}
```

The `"in"` field for API Key auth accepts `"header"` or `"queryparams"`.

### Postman Compatibility

The auth storage format is fully compatible with Postman Collection v2.1. This means:

- Collections exported from Postman with auth settings are correctly loaded by Perseus
- Collections saved by Perseus with auth settings can be imported into Postman
- Auth type, credentials, and API key location are preserved in both directions

## Keyboard Reference

Quick reference for all auth-related keybindings:

| Context | Key | Action |
|---------|-----|--------|
| Request panel | `Ctrl+L` / `Ctrl+H` | Switch between Headers / Auth / Body tabs |
| Auth tab (navigation) | `j` / `Down` | Next auth field |
| Auth tab (navigation) | `k` / `Up` | Previous auth field |
| Auth tab (navigation) | `Enter` | Open type popup, toggle location, or enter editing |
| Auth tab (navigation) | `i` | Enter vim insert mode on text fields |
| Auth type popup | `j` / `Down` | Highlight next type |
| Auth type popup | `k` / `Up` | Highlight previous type |
| Auth type popup | `Enter` | Confirm selection |
| Auth type popup | `Esc` | Cancel and close popup |
| Auth field (editing) | `Esc` | Exit editing, return to navigation |
| Any mode | `Ctrl+R` | Send request (auth is auto-injected) |

## Auth and Manual Headers

Auth credentials are injected **before** custom headers are applied. If you set auth to Bearer Token and also manually add an `Authorization` header in the Headers tab, the manual header will take precedence (reqwest appends both, and servers typically use the last value).

To avoid conflicts, use either the Auth tab or manual headers for authentication — not both.
