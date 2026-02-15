# Request Body Types

Perseus supports six request body types, each with a dedicated editor and automatic Content-Type management. Body type settings are configured through the **Body tab** in the request panel and are automatically applied when sending requests.

## Supported Body Types

| Type | Description | Content-Type (auto-set) |
|------|-------------|------------------------|
| **Raw** | Plain text body (default) | None (manually set if needed) |
| **JSON** | JSON body with validation indicator | `application/json` |
| **XML** | XML body | `application/xml` |
| **Form URL-Encoded** | Key-value pairs for form submissions | `application/x-www-form-urlencoded` |
| **Multipart Form** | Key-value pairs with file attachments | `multipart/form-data` (with boundary) |
| **Binary** | Send a file as the raw request body | `application/octet-stream` |

## Getting Started

### Opening the Body Tab

The Body tab sits after the Auth tab in the request panel:

```
Headers | Auth | Body
```

Navigate to the Body tab using:

- `Ctrl+L` or `Ctrl+H` to cycle between request tabs (Headers, Auth, Body)
- `Tab` to switch panels, then navigate to the Body tab

The tab label dynamically reflects the active body mode (e.g., `Body (JSON)`, `Body (Form)`, `Body (Multipart)`). When set to Raw, the tab simply displays `Body`.

### Selecting a Body Type

1. Navigate to the Body tab — the `Type: [Raw]` selector is at the top
2. Press `Enter` on the type selector to open the body type popup
3. Use `j`/`k` or arrow keys to highlight a type
4. Press `Enter` to confirm, or `Esc` to cancel

When you select a new body type, the cursor moves to the appropriate content area (text editor, KV table, or file path field).

### Body Panel Layout

The Body tab has two zones:

```
┌──────────────────────────────┐
│ Type: [JSON] ✓               │  ← Mode selector row
├──────────────────────────────┤
│                              │
│ (content editor area)        │  ← Text editor, KV table, or file path
│                              │
└──────────────────────────────┘
```

Navigate between the mode selector and content area using `j`/`k` or arrow keys.

## Body Type Details

### Raw

Plain text body sent as-is. No Content-Type header is automatically added — set it manually in the Headers tab if needed.

**Editor:** Full vim-powered text editor (same as the Headers editor).

**Example usage:** Sending GraphQL queries, plain text, or any body format not covered by other modes.

### JSON

JSON body with live syntax validation and automatic Content-Type injection.

**Editor:** Full vim-powered text editor.

**Validation indicator:** A green checkmark (✓) or red cross (✗) appears next to the type selector, showing whether the current body text is valid JSON. The indicator only appears when the body is non-empty.

```
Type: [JSON] ✓     ← valid JSON
Type: [JSON] ✗     ← invalid JSON
```

**What happens at send time:**

Perseus automatically adds the header:
```
Content-Type: application/json
```

If you manually set a `Content-Type` header in the Headers tab, the manual header takes precedence and the auto-injection is skipped.

**Example usage:** Sending JSON payloads to REST APIs, webhook endpoints, or any service expecting `application/json`.

### XML

XML body with automatic Content-Type injection.

**Editor:** Full vim-powered text editor.

**What happens at send time:**

Perseus automatically adds the header:
```
Content-Type: application/xml
```

Manual `Content-Type` headers in the Headers tab take precedence over auto-injection.

**Example usage:** Sending SOAP requests, XML-RPC calls, or any XML-based API payloads.

### Form URL-Encoded

A key-value pair editor for submitting form data. Each pair is sent as `key=value` with proper URL encoding.

**Editor:** Interactive KV table:

```
┌───┬──────────────────┬──────────────────┐
│   │ Key              │ Value            │
├───┼──────────────────┼──────────────────┤
│ ✓ │ username         │ admin            │
│ ✓ │ password         │ secret           │
│ ✓ │                  │                  │  ← empty trailing row
└───┴──────────────────┴──────────────────┘
```

- The `✓` column shows whether a row is enabled (sent) or disabled (skipped)
- An empty trailing row is always present for adding new pairs
- Disabled rows appear dimmed

**What happens at send time:**

Perseus encodes the enabled pairs and sets:
```
Content-Type: application/x-www-form-urlencoded
```

The body is sent as `username=admin&password=secret` with proper percent-encoding handled by the HTTP client.

**Example usage:** Login forms, API endpoints expecting form-encoded POST data, OAuth token requests.

### Multipart Form

A key-value pair editor with per-row type selection (Text or File) for multipart submissions. Supports file uploads alongside text fields.

**Editor:** Interactive KV table with a Type column:

```
┌───┬──────────────────┬──────┬──────────────────┐
│   │ Key              │ Type │ Value            │
├───┼──────────────────┼──────┼──────────────────┤
│ ✓ │ name             │ Text │ John             │
│ ✓ │ avatar           │ File │ /path/to/img.png │
│ ✓ │                  │ Text │                  │
└───┴──────────────────┴──────┴──────────────────┘
```

- Toggle the Type column between `Text` and `File` with the `t` key
- When type is `File`, the Value column should contain a file path
- Files are read from disk at send time

**What happens at send time:**

Perseus builds a multipart form:
- Text fields are sent as form text parts
- File fields are read from disk and sent as file attachments with the original filename preserved
- Content-Type is set to `multipart/form-data` with an auto-generated boundary

If a file path is invalid or the file cannot be read, an error message is displayed in the response area.

**Example usage:** File upload APIs, profile image uploads, form submissions with mixed text and file data.

### Binary

Send a file directly as the request body. The entire file contents become the body payload.

**Editor:** File path input field with file info display:

```
 File:
┌────────────────────────────────┐
│ /path/to/payload.bin           │
└────────────────────────────────┘
 1024 bytes
```

The info line below the path field shows:
- File size in bytes (when the file exists)
- `File not found` (when the path doesn't point to a valid file)
- `No file selected` (when the path is empty)

**What happens at send time:**

Perseus reads the file from disk and sends its raw contents as the body, with the header:
```
Content-Type: application/octet-stream
```

Manual `Content-Type` headers in the Headers tab take precedence over auto-injection.

**Example usage:** Uploading firmware images, sending binary protocols, posting raw file data to storage APIs.

## Navigation and Editing

### Navigating Within the Body Tab

| Key | Context | Action |
|-----|---------|--------|
| `j` / `Down` | Mode selector | Move to the content area below |
| `k` / `Up` | Content area | Move back to mode selector |
| `j` / `Down` | KV table | Move to next row |
| `k` / `Up` | KV table | Move to previous row |
| `Tab` / `l` / `Right` | KV table | Move to next column (Key → Value → next row) |
| `Shift+Tab` / `h` / `Left` | KV table | Move to previous column (Value → Key → previous row) |

When navigating past the last KV row (pressing `j`), focus moves to the response panel. When navigating before the mode selector (pressing `k`), focus returns to the URL bar.

### Editing Text Bodies (Raw, JSON, XML)

1. Navigate to the text editor area (below the mode selector)
2. Press `Enter` to enter vim normal mode, or `i` to enter vim insert mode directly
3. Edit using vim keybindings (insert, normal, visual modes)
4. Press `Esc` to exit back to navigation mode

All standard vim operations work: word motions (`w`, `b`, `e`), text objects (`ciw`, `diw`), yank/paste (`y`, `p`), visual selection (`v`), and clipboard integration (`Ctrl+C`/`Ctrl+V`).

### Editing KV Cells (Form URL-Encoded, Multipart)

1. Navigate to the desired cell using `j`/`k` for rows and `Tab`/`h`/`l` for columns
2. Press `Enter` or `i` to edit the cell — an inline text editor appears
3. Type or edit the cell value using vim keybindings
4. Press `Esc` to commit the edit and return to KV navigation

KV cells are single-line editors. The edited value is written back to the data model when you exit.

### KV Table Operations

| Key | Action |
|-----|--------|
| `a` or `o` | Add a new empty row below the current row |
| `d` | Delete the current row (minimum 1 row always remains) |
| `Space` | Toggle the current row enabled/disabled |
| `t` | Toggle field type between Text and File (Multipart only) |

### Editing the Binary Path

1. Navigate to the file path field
2. Press `Enter` or `i` to enter editing mode
3. Type or paste the file path
4. Press `Esc` to exit editing — the file info updates immediately

## Content-Type Auto-Injection

Perseus automatically sets the `Content-Type` header at send time based on the body mode:

| Body Mode | Auto-injected Content-Type |
|-----------|---------------------------|
| Raw | *(none)* |
| JSON | `application/json` |
| XML | `application/xml` |
| Form URL-Encoded | `application/x-www-form-urlencoded` |
| Multipart Form | `multipart/form-data; boundary=...` |
| Binary | `application/octet-stream` |

**Manual override:** If you set a `Content-Type` header in the Headers tab, it takes precedence. The auto-injected header is skipped for Raw, JSON, XML, and Binary modes. For Form URL-Encoded and Multipart Form, the Content-Type is managed by the HTTP client and both headers may be sent (matching Postman's behavior).

## Environment Variable Substitution

Environment variables (e.g., `{{base_url}}`, `{{api_key}}`) are substituted at send time across all body content:

- **Text bodies** (Raw, JSON, XML): Variables in the text are replaced
- **KV pairs** (Form URL-Encoded, Multipart): Variables in both keys and values are replaced
- **Binary path**: Variables in the file path are replaced

Variables are resolved from the active environment. If no environment is active or a variable is not defined, the placeholder text is sent as-is.

## Persistence

Body type settings are saved as part of the Postman Collection v2.1 format used by Perseus for request storage. When you save a request, its body mode and all associated data are persisted.

### Storage Format

Body data is stored in the `body` field of each request:

**Raw:**
```json
{
  "body": {
    "mode": "raw",
    "raw": "plain text content"
  }
}
```

**JSON:**
```json
{
  "body": {
    "mode": "raw",
    "raw": "{\"key\": \"value\"}",
    "options": {
      "raw": { "language": "json" }
    }
  }
}
```

**XML:**
```json
{
  "body": {
    "mode": "raw",
    "raw": "<root><item>value</item></root>",
    "options": {
      "raw": { "language": "xml" }
    }
  }
}
```

**Form URL-Encoded:**
```json
{
  "body": {
    "mode": "urlencoded",
    "urlencoded": [
      { "key": "username", "value": "admin" },
      { "key": "password", "value": "secret", "disabled": true }
    ]
  }
}
```

**Multipart Form:**
```json
{
  "body": {
    "mode": "formdata",
    "formdata": [
      { "key": "name", "value": "John", "type": "text" },
      { "key": "avatar", "src": "/path/to/image.png", "type": "file" },
      { "key": "inactive", "value": "test", "type": "text", "disabled": true }
    ]
  }
}
```

**Binary:**
```json
{
  "body": {
    "mode": "file",
    "file": { "src": "/path/to/payload.bin" }
  }
}
```

### Postman Compatibility

The body storage format is fully compatible with Postman Collection v2.1. This means:

- Collections exported from Postman with various body types are correctly loaded by Perseus
- Collections saved by Perseus can be imported into Postman with body data preserved
- Body mode, content, KV pairs, file references, and disabled states are all preserved in both directions

### Mode Switching and Data Preservation

Each body mode maintains its own data independently:

- **Text modes** (Raw, JSON, XML) share a single text editor — switching between them preserves the text content
- **Form URL-Encoded** pairs are stored separately and persist when switching away and back
- **Multipart** fields are stored separately and persist when switching away and back
- **Binary** file path is stored separately and persists when switching away and back

When a new request is created or loaded, the body mode resets to Raw with empty defaults for all mode-specific data.

## Keyboard Reference

Quick reference for all body-related keybindings:

| Context | Key | Action |
|---------|-----|--------|
| Request panel | `Ctrl+L` / `Ctrl+H` | Switch between Headers / Auth / Body tabs |
| Body tab (navigation) | `j` / `Down` | Next field or row |
| Body tab (navigation) | `k` / `Up` | Previous field or row |
| Body tab (navigation) | `Enter` | Open type popup, enter editing, or edit KV cell |
| Body tab (navigation) | `i` | Enter vim insert mode on text fields / edit KV cell |
| Body type popup | `j` / `Down` | Highlight next type |
| Body type popup | `k` / `Up` | Highlight previous type |
| Body type popup | `Enter` | Confirm selection |
| Body type popup | `Esc` | Cancel and close popup |
| KV table (navigation) | `Tab` / `l` / `Right` | Next column |
| KV table (navigation) | `Shift+Tab` / `h` / `Left` | Previous column |
| KV table (navigation) | `a` / `o` | Add row below current |
| KV table (navigation) | `d` | Delete current row |
| KV table (navigation) | `Space` | Toggle row enabled/disabled |
| KV table (navigation) | `t` | Toggle type Text/File (Multipart only) |
| KV cell (editing) | `Esc` | Commit edit and return to KV navigation |
| Text editor (editing) | `Esc` | Exit editing, return to navigation |
| Any mode | `Ctrl+R` | Send request (body is auto-included) |
