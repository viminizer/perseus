# Response Panel

Perseus provides a full-featured response panel for inspecting API responses. After sending a request with `Ctrl+R`, the response panel displays the status code, timing, body size, and full response content. You can navigate the response with vim keybindings, search within the body, copy content to the clipboard, and save responses to files — all without leaving the keyboard.

## Response Panel Layout

```
┌─ Response ──────────────────────────────────────┐
│ Body | Headers          200 OK (245ms) · 1.2 KB │  <- Tab bar with status
│                                                  │
│ {                                                │
│   "id": 1,                                      │  <- Response content
│   "name": "Perseus",                            │     (Body or Headers tab)
│   ...                                            │
│                                                  │
│ /search query                       [Aa] 3/17   │  <- Search bar (when active)
└──────────────────────────────────────────────────┘
```

The tab bar shows two tabs (**Body** and **Headers**) on the left and the response status on the right. The status line includes the HTTP status code, reason phrase, response time in milliseconds, and body size formatted in human-readable units.

## Features Overview

| Feature | Key | Context | Description |
|---------|-----|---------|-------------|
| **Size Display** | *(automatic)* | Tab bar | Body size shown after duration |
| **Copy Content** | `c` | Navigation mode, response focused | Copy body or headers to clipboard |
| **Save to File** | `S` | Navigation mode, response focused | Save body or headers to a file |
| **Body Search** | `/` | Editing Normal mode, Body tab | Vim-style incremental search |
| **Search Navigation** | `n` / `N` | Editing Normal mode, Body tab | Next / previous match |
| **Case Toggle** | `Ctrl+I` | During search input | Toggle case sensitivity |

## Response Size Display

After a successful response, the tab bar displays the response body size alongside the status code and duration:

```
200 OK (245ms) · 1.2 KB
```

Size formatting uses 1024-based thresholds:

| Size | Display |
|------|---------|
| 0 bytes | `0 B` |
| 512 bytes | `512 B` |
| 1536 bytes | `1.5 KB` |
| 2621440 bytes | `2.5 MB` |
| 1181116006 bytes | `1.1 GB` |

The size reflects the raw (decompressed) response body as received from the server, matching what tools like Postman display. This is the wire size before any pretty-printing or formatting.

**Narrow terminals:** When the response panel inner width is less than 50 columns, the size is hidden to prevent overlap with the tab labels. The status code and duration remain visible.

**Non-success states:** Size is only shown for successful responses. Loading, error, cancelled, and empty states show their own status text without size information.

## Copy Response Content

Copy the currently visible tab content (body or headers) to the system clipboard with a single keystroke.

### How to Copy

1. Focus the response panel using `h`/`j`/`k`/`l` navigation in Navigation mode
2. Switch tabs if needed: press `Enter` to enter editing mode, then `H`/`L` to switch between Body and Headers, then `Esc` back to Navigation mode
3. Press `c` to copy

### What Gets Copied

| Active Tab | What is copied | Toast message |
|------------|---------------|---------------|
| Body | The formatted (pretty-printed) response body | `Copied response body (1.2 KB)` |
| Headers | The rendered header lines (`Key: Value` format) | `Copied response headers` |

If no successful response exists (empty, loading, error, or cancelled state), pressing `c` shows the toast `No response to copy`.

The toast message appears in the status bar for 2 seconds.

### Example Workflow

```
1. Send request:    Ctrl+R
2. Wait for response
3. Copy body:       c         -> Toast: "Copied response body (4.7 KB)"
4. Switch to headers: Enter -> L -> Esc
5. Copy headers:    c         -> Toast: "Copied response headers"
```

## Save Response to File

Save the current tab content (body or headers) to a file on disk.

### How to Save

1. Focus the response panel in Navigation mode
2. Press `S` (uppercase) to open the save popup
3. Type a file path in the input field
4. Press `Enter` to write the file, or `Esc` to cancel

### The Save Popup

```
┌─ Save Response ──────────────────────────────┐
│ ~/output.json                                │
└──────────────────────────────────────────────┘
```

The popup appears centered on screen with a text input field. The input supports standard editing keys:

| Key | Action |
|-----|--------|
| Characters | Insert at cursor position |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Left` / `Right` | Move cursor |
| `Home` / `End` | Jump to start / end of input |
| `Enter` | Confirm and write file |
| `Esc` | Cancel and close popup |

While the save popup is open, all other keybindings are suspended — no navigation or vim input can bleed through.

### Path Resolution

- **Relative paths** are resolved relative to the working directory where Perseus was launched
- **Tilde expansion** is supported: `~/output.json` resolves to `/Users/you/output.json`
- **Parent directories** must already exist; Perseus does not create intermediate directories

### Success and Error Handling

| Outcome | Toast message |
|---------|---------------|
| File written successfully | `Saved to ~/output.json (4.7 KB)` |
| Parent directory does not exist | `Save failed: directory does not exist` |
| Permission denied or OS error | `Save failed: Permission denied (os error 13)` |
| No successful response | `No response to save` |

If the file already exists, it is overwritten silently (matching `curl -o` behavior).

### What Gets Saved

The content saved matches the active response tab:

- **Body tab:** The formatted (pretty-printed) response body
- **Headers tab:** The rendered header lines in `Key: Value` format

### Example Workflow

```
1. Send request:        Ctrl+R
2. Focus response:      l  (or navigate with arrows)
3. Open save popup:     S
4. Type path:           ~/api-response.json
5. Confirm:             Enter  -> Toast: "Saved to ~/api-response.json (4.7 KB)"
```

## Response Body Search

Vim-style `/` search with incremental highlighting and match navigation. Search lets you find text within large API response bodies without scrolling manually.

### Opening the Search Bar

1. Focus the response panel and enter editing mode: press `Enter` while on the response panel
2. Make sure you are on the **Body** tab (search is only available on the Body tab; `/` is a no-op on the Headers tab)
3. Press `/` in vim Normal mode

The search bar appears at the bottom of the response content area, stealing one row from the content:

```
┌─ Response ──────────────────────────────────────┐
│ Body | Headers          200 OK (245ms) · 1.2 KB │
│                                                  │
│ {                                                │
│   "id": 1,                                      │
│   "name": "Perseus",                            │
│ }                                                │
│ /Perseus                            [Aa] 1/3    │  <- Search bar
└──────────────────────────────────────────────────┘
```

### Search Bar Layout

```
/ query text here                          [Aa] 3/17
│                                           │    │
│                                           │    └─ Match count (current/total)
│                                           └─ Case indicator
└─ Search input with / prefix
```

| Indicator | Meaning |
|-----------|---------|
| `[Aa]` | Case-insensitive search (default) |
| `[AA]` | Case-sensitive search |
| `3/17` | Currently on the 3rd match out of 17 total |
| `0/0` | No matches found |

### Searching

While the search bar is active:

| Key | Action |
|-----|--------|
| Characters | Type to build the search query |
| `Backspace` | Delete character before cursor |
| `Delete` | Delete character at cursor |
| `Left` / `Right` | Move cursor within search input |
| `Enter` | Confirm search — close input bar, keep highlights and query |
| `Esc` | Cancel search — close input bar, clear query and highlights |
| `Ctrl+I` | Toggle case sensitivity and recompute matches |

**Incremental search:** Matches are recomputed on every keystroke as you type. You see highlights update in real time without pressing Enter.

### Match Highlighting

Matches are highlighted directly over the response body content, preserving JSON syntax colors for non-matching regions:

| Highlight | Color | Meaning |
|-----------|-------|---------|
| Yellow background, black text | All matches | Every occurrence of the search query |
| Light red background, black text | Current match | The match you are currently navigated to |

### Navigating Matches

After confirming a search with `Enter` (or while the search bar is active), use `n` and `N` in vim Normal mode to jump between matches:

| Key | Action |
|-----|--------|
| `n` | Jump to the **next** match (wraps around to the first match after the last) |
| `N` | Jump to the **previous** match (wraps around to the last match from the first) |

The response view auto-scrolls to bring the current match into the visible area.

### Search Lifecycle

```
                    ┌──────────┐
          /         │  Search  │       Enter (non-empty)
     ────────────>  │  Active  │  ──────────────────────┐
                    └──────────┘                        │
                         │                              v
                    Esc  │                     ┌──────────────┐
                    or   │                     │  Highlights  │
                 Enter   │                     │   Visible    │
               (empty)   │                     │  (n/N work)  │
                         │                     └──────────────┘
                         v                              │
                    ┌──────────┐        Esc (nav mode)  │
                    │   No     │  <─────────────────────┘
                    │  Search  │        or new response
                    └──────────┘
```

1. **Search Active:** The search bar is visible, receiving keystrokes. Matches update on every keystroke.
2. **Highlights Visible:** The search bar shows the confirmed query and match count. `n`/`N` navigate between matches. Pressing `/` reopens the search input with the current query pre-filled.
3. **No Search:** No highlights, no search bar. Triggered by `Esc` during search, empty `Enter`, or a new response arriving.

### Clearing Search

Search state is automatically cleared when:

- You press `Esc` while the search bar is active
- You press `Enter` with an empty query
- A new response arrives (new request sent)

### Edge Cases

- **Empty query + Enter:** Clears all search state (same as Esc)
- **No matches:** The search bar shows `0/0` and no highlights appear in the body
- **Headers tab:** `/` is a no-op when the Headers tab is active — search is only available on the Body tab
- **Large responses:** Match computation is O(n) per keystroke and runs on the formatted body text. For typical API responses under 100 KB, latency is negligible

### Example Workflow

```
1. Send request:          Ctrl+R
2. Enter response:        Enter          (vim Normal mode on response body)
3. Start search:          /
4. Type query:            error          (highlights appear incrementally)
5. Confirm search:        Enter          (search bar shows "1/5")
6. Next match:            n              (jumps to match 2/5)
7. Next match:            n              (jumps to match 3/5)
8. Previous match:        N              (back to match 2/5)
9. New search:            /              (reopens with "error" pre-filled)
10. Clear and retype:     Ctrl+A, then type "warning"
11. Cancel search:        Esc            (highlights cleared)
```

## Navigation and Editing

### Focusing the Response Panel

From Navigation mode, use directional keys to move focus to the response panel:

| Key | Action |
|-----|--------|
| `h` / `j` / `k` / `l` | Move focus between panels |
| Arrow keys | Same as h/j/k/l |

The response panel border turns green when focused.

### Switching Response Tabs

In Navigation mode, press `Enter` to enter editing mode, then:

| Key | Action |
|-----|--------|
| `H` | Switch to the previous tab (Headers -> Body) |
| `L` | Switch to the next tab (Body -> Headers) |

### Reading the Response

In editing mode (vim Normal), you can navigate within the response body using standard vim motions:

| Key | Action |
|-----|--------|
| `h` / `j` / `k` / `l` | Cursor movement |
| `w` / `b` / `e` | Word forward / back / end |
| `0` / `^` / `$` | Line start / first non-blank / line end |
| `gg` / `G` | Jump to top / bottom |
| `v` / `V` | Enter visual / visual line mode |

The response body is **read-only** — insert mode and text modification commands are disabled. Visual mode works for selecting text to yank (copy).

### Copying with Vim Yank

In addition to the `c` one-key copy, you can use vim visual mode to copy specific selections:

1. Enter editing mode: `Enter`
2. Enter visual mode: `v` (character) or `V` (line)
3. Move to expand selection: `h`/`j`/`k`/`l` or word motions
4. Yank to clipboard: `y` or `Cmd+C` / `Ctrl+C`
5. Exit: `Esc`

## Keyboard Reference

Quick reference for all response panel keybindings:

### Navigation Mode (Response Focused)

| Key | Action |
|-----|--------|
| `Enter` | Enter editing mode (vim Normal) |
| `i` | Enter editing mode (vim Normal for response) |
| `c` | Copy current tab content to clipboard |
| `S` | Open save-to-file popup |
| `h` / `j` / `k` / `l` | Move focus to adjacent panel |
| `?` | Toggle help overlay |

### Editing Mode (Response Body)

| Key | Mode | Action |
|-----|------|--------|
| `H` / `L` | Normal | Switch response tab (Body / Headers) |
| `/` | Normal (Body tab) | Open search bar |
| `n` | Normal (Body tab) | Jump to next search match |
| `N` | Normal (Body tab) | Jump to previous search match |
| `h` / `j` / `k` / `l` | Normal | Cursor movement |
| `w` / `b` / `e` | Normal | Word motions |
| `gg` / `G` | Normal | Jump to top / bottom |
| `v` / `V` | Normal | Enter visual / visual line mode |
| `y` | Visual | Yank selection to clipboard |
| `Cmd+C` / `Ctrl+C` | Visual | Copy selection to system clipboard |
| `Esc` | Any | Exit to navigation mode |

### Search Bar (Active)

| Key | Action |
|-----|--------|
| Characters | Type search query |
| `Backspace` / `Delete` | Delete character |
| `Left` / `Right` | Move cursor within input |
| `Enter` | Confirm search, close input, keep highlights |
| `Esc` | Cancel search, clear query and highlights |
| `Ctrl+I` | Toggle case sensitivity |

### Save Popup

| Key | Action |
|-----|--------|
| Characters | Type file path |
| `Backspace` / `Delete` | Delete character |
| `Left` / `Right` | Move cursor |
| `Home` / `End` | Jump to start / end |
| `Enter` | Write file and close |
| `Esc` | Cancel and close |
