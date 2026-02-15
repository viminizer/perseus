# Configuration

Perseus uses TOML configuration files to customize HTTP behavior, proxy settings, SSL/TLS, and UI preferences. Configuration is optional — Perseus works with sensible defaults when no config files are present.

## File Locations

Perseus supports two configuration layers that are merged together:

| Layer | Path | Purpose |
|-------|------|---------|
| Global | `$XDG_CONFIG_HOME/perseus/config.toml` | User-wide defaults |
| Project | `{project_root}/.perseus/config.toml` | Per-project overrides |

The global path falls back to `~/.config/perseus/config.toml` when `$XDG_CONFIG_HOME` is not set.

The project root is detected by walking up from the current directory looking for `.git`, `Cargo.toml`, `package.json`, or `.perseus`.

## Layered Resolution

Settings are resolved in this order, where each layer overrides the previous:

```
Hardcoded defaults
    |
    v
Global config (~/.config/perseus/config.toml)
    |
    v
Project config (.perseus/config.toml)
```

Merging is **field-level**, not table-level. If the global config sets `proxy.url` and `proxy.no_proxy`, and the project config only sets `proxy.url`, the global `proxy.no_proxy` value is preserved.

Missing config files are silently skipped. If neither file exists, all defaults apply.

## Configuration Reference

### `[http]`

Controls HTTP request behavior.

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `timeout` | integer | `30` | 0 -- 600 | Request timeout in seconds. `0` disables the timeout. |
| `follow_redirects` | boolean | `true` | | Whether to follow HTTP redirects. |
| `max_redirects` | integer | `10` | 0 -- 100 | Maximum number of redirects to follow. |

```toml
[http]
timeout = 10
follow_redirects = true
max_redirects = 5
```

### `[proxy]`

Configures an HTTP/HTTPS proxy. Both fields are optional — omit the entire section to use direct connections.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `url` | string | none | Proxy URL (e.g., `"http://proxy.corp:8080"`). Must be a valid URL. |
| `no_proxy` | string | none | Comma-separated hostnames that bypass the proxy. |

```toml
[proxy]
url = "http://proxy.corp:8080"
no_proxy = "localhost,127.0.0.1,.internal"
```

### `[ssl]`

Controls SSL/TLS verification and client certificates.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `verify` | boolean | `true` | Whether to verify server SSL certificates. |
| `ca_cert` | string | none | Path to a custom CA certificate (PEM format). |
| `client_cert` | string | none | Path to a client certificate for mutual TLS (PEM format). |
| `client_key` | string | none | Path to a client private key for mutual TLS (PEM format). |

`client_cert` and `client_key` must both be set or both be omitted. Setting only one is a validation error.

All path fields support tilde expansion (`~` is replaced with `$HOME`).

```toml
[ssl]
verify = true
ca_cert = "~/certs/corporate-ca.pem"
client_cert = "~/certs/client.pem"
client_key = "~/certs/client-key.pem"
```

> **Warning:** Setting `verify = false` disables certificate verification entirely. Use this only for development or testing against self-signed certificates.

### `[ui]`

Controls user interface defaults.

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `sidebar_width` | integer | `32` | 28 -- 60 | Default sidebar width in characters. |

The sidebar width from config is used as the initial default. If you resize the sidebar during a session, the session-persisted width takes precedence on the next launch.

```toml
[ui]
sidebar_width = 40
```

### `[editor]`

Controls text editor behavior for request fields.

| Key | Type | Default | Range | Description |
|-----|------|---------|-------|-------------|
| `tab_size` | integer | `2` | 1 -- 8 | Number of spaces inserted when pressing Tab. |

```toml
[editor]
tab_size = 4
```

## Full Example

```toml
# ~/.config/perseus/config.toml

[http]
timeout = 15
follow_redirects = true
max_redirects = 5

[proxy]
url = "http://proxy.corp:8080"
no_proxy = "localhost,127.0.0.1,.internal"

[ssl]
verify = true
ca_cert = "~/certs/corporate-ca.pem"

[ui]
sidebar_width = 36

[editor]
tab_size = 4
```

## Common Scenarios

### Corporate Environment with Proxy

Create a global config at `~/.config/perseus/config.toml`:

```toml
[proxy]
url = "http://proxy.corp:8080"
no_proxy = "localhost,127.0.0.1,.corp.internal"

[ssl]
ca_cert = "~/certs/corporate-ca.pem"
```

### Staging Server with Self-Signed Certs

Create a project config at `.perseus/config.toml` in your staging project:

```toml
[ssl]
verify = false

[http]
timeout = 60
```

This overrides the global SSL settings only for that project. Other projects keep the global defaults.

### Mutual TLS (mTLS) Authentication

```toml
[ssl]
client_cert = "~/certs/client.pem"
client_key = "~/certs/client-key.pem"
```

Both `client_cert` and `client_key` must point to PEM-encoded files. Perseus reads both files and sends the client identity with every request.

### Disable Request Timeout

```toml
[http]
timeout = 0
```

Setting `timeout` to `0` removes the timeout entirely. Useful for long-running API calls, but be aware that requests may hang indefinitely.

## Error Handling

Perseus validates all configuration at startup. If any errors are found, it prints descriptive messages to stderr and exits with a non-zero code.

### Invalid TOML Syntax

```
config error: failed to parse "/home/user/.config/perseus/config.toml":
TOML parse error at line 3, column 10
  |
3 | timeout = fast
  |           ^
expected a boolean, integer, float, string, array, or table
```

### Out-of-Range Value

```
config error: http.timeout = 999 is out of range (0..=600)
```

### Invalid Proxy URL

```
config error: proxy.url = "not-a-url" is not a valid URL
```

### Missing Certificate File

```
config error: ssl.ca_cert = "/home/user/certs/ca.pem" — file not found
```

### Mismatched Client Cert/Key

```
config error: ssl.client_cert and ssl.client_key must both be set or both be unset
```

## Forward Compatibility

Unknown keys in the config file are silently ignored. This means config files written for newer versions of Perseus will still work with older versions — unrecognized settings are simply skipped. Only the keys documented above are read.

## Sample Config File

A fully commented sample config with all keys and their defaults is available at [`docs/sample-config.toml`](sample-config.toml). Copy it to `~/.config/perseus/config.toml` and uncomment the settings you want to change.
