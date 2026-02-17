---
title: "feat: add response metadata display, copy/save actions, and body search"
type: feat
date: 2026-02-17
---

# Response Metadata & Search

## Overview

Add four response-panel enhancements that round out Perseus's Phase 1 feature set: response size display in the tab bar, one-key copy of response content, save-response-to-file with a path input popup, and vim-style `/` search with incremental highlighting and `n`/`N` match navigation.

## Problem Statement / Motivation

Perseus currently shows status code and duration after a request completes but has no way to:

1. **See the response size** -- developers routinely check payload size when debugging APIs, measuring performance, or enforcing contract limits.
2. **Copy the full response body in one keystroke** -- the only path today is entering vim editing mode, selecting all (`ggVG`), and yanking. That is too many steps for a daily action.
3. **Save the response to a file** -- there is no export mechanism; users must manually copy-paste into a file.
4. **Search within the response** -- large JSON payloads are unnavigable without find. The sidebar already has `/` search for filtering items; the response panel has nothing equivalent.

These are the last Phase 1 features. Completing them makes Perseus viable for daily API development work.

## Proposed Solution

Four sub-features, ordered from simplest to most complex:

### 1. Response Size Display

Show the response body byte count alongside the existing status text in the response tab bar.

- Capture `body_size_bytes: usize` on `ResponseData` by calling `response.bytes().await` in `src/http.rs`, measuring `.len()`, then converting to a UTF-8 string with `String::from_utf8_lossy()`. This preserves the true wire size even when the body is later pretty-printed. (Note: reqwest's `.bytes()` and `.text()` both consume the response -- you must use `.bytes()` and convert manually.)
- Add a `format_size(bytes: usize) -> String` helper (1024-based thresholds: B / KB / MB / GB, one decimal place above bytes).
- Extend the right-aligned status text in `render_response_tab_bar()` from `"200 OK (245ms)"` to `"200 OK (245ms) · 1.2 KB"`.
- At narrow widths (< 50 cols inner), hide the size to avoid overlapping the tab labels.
- Size is body-only, not headers.

### 2. One-Key Copy Response Content

Copy the currently visible tab content (body **or** headers) to the system clipboard with a single keystroke.

| Mode | Key | Behavior |
|------|-----|----------|
| Navigation (response focused) | `c` | Copy current tab content, show toast |

Users in Editing mode press Esc first to return to Navigation, then `c`. One extra keystroke is acceptable -- it avoids adding a parallel Ctrl-combo binding with terminal compatibility concerns.

- If `response_tab == Body`, copy the formatted (pretty-printed) body text.
- If `response_tab == Headers`, copy the rendered header text.
- If no successful response exists, show toast "No response to copy".
- Reuse the existing `clipboard.set_text()` + `set_clipboard_toast()` pattern from `src/app.rs`.
- Toast message: `"Copied response body (1.2 KB)"` or `"Copied response headers"`.

### 3. Save Response to File

Save the current tab content (body or headers) to a user-specified file path.

| Mode | Key | Behavior |
|------|-----|----------|
| Navigation (response focused) | `S` | Open file path input popup |

Users in Editing mode press Esc first to return to Navigation, then `S`. This avoids `Ctrl+Shift+S`, which most terminals cannot distinguish from `Ctrl+S` (already bound to save-collection).

**Popup design:**
- Center-screen popup matching the existing `SidebarPopup` visual pattern (bordered block with title "Save Response").
- Reuse `TextInput` struct for path entry.
- Enter confirms and writes the file; Esc cancels.
- Paths are resolved relative to the CWD where Perseus was launched.
- Tilde expansion (`~/`) is supported via simple prefix replacement.
- If the path's parent directory does not exist, show an error toast.
- If the file already exists, overwrite silently (matches `curl -o` behavior).
- On success: toast `"Saved to <path> (1.2 KB)"`.
- On error: toast `"Save failed: <OS error>"`.

**State:**
- Add `save_popup: Option<TextInput>` to `App`.
- Intercept keys early in `handle_key()` when `save_popup.is_some()` -- route to `handle_text_input()`.
- Render via a new `render_save_popup()` function in `src/ui/mod.rs`.

### 4. Response Body Search

Vim-style `/` search with incremental highlighting and match navigation.

**Activation:**
- Press `/` while in Editing Normal mode on the response Body tab.
- A 1-row search bar appears at the bottom of the response content area (steals space from the content, does not overlay).
- Format: `/ query text here                          [Aa] 3/17`
  - Left: search input with `/` prefix
  - Right: case indicator (`Aa` = insensitive, `AA` = sensitive) and match count `current/total`

**State machine integration (Option B -- boolean flag):**
- Add to `App`:
  ```rust
  pub struct ResponseSearch {
      pub active: bool,          // search input bar is visible and receiving keys
      pub query: String,         // persisted after Enter so n/N can use it
      pub input: TextInput,      // current input field
      pub matches: Vec<SearchMatch>, // (line_index, byte_start, byte_end)
      pub current_match: usize,  // index into matches vec
      pub case_sensitive: bool,  // toggle state
  }
  ```
- `response_search: ResponseSearch` on `App` (always present, `active` toggled).
- When `active == true`, `handle_editing_mode()` intercepts **all** keys before vim dispatch:
  - Character keys go to `input`.
  - After each keystroke, recompute matches against the raw body text (pre-wrap, post-format).
  - Enter: confirm search -- set `active = false`, persist `query = input.text()`, keep highlights and `current_match`.
  - Esc: cancel -- set `active = false`, clear `query`, clear matches and highlights.
  - `Ctrl+I`: toggle `case_sensitive`, recompute matches.
- When `active == false` but `query` is non-empty, `n`/`N` in vim Normal mode navigate matches:
  - Intercept `n`/`N` in `handle_editing_mode()` before vim dispatch.
  - `n` advances `current_match` (wraps around).
  - `N` moves backwards (wraps around).
  - Auto-scroll to bring the current match into view.

**Highlighting:**
- Matches are highlighted with `bg(Color::Yellow) + fg(Color::Black)`.
- The current match is highlighted with `bg(Color::LightRed) + fg(Color::Black)`.
- Highlighting is applied as a post-processing pass over `cache.lines` (the colorized `Vec<Line<'static>>`) before word wrapping, by splitting spans at match boundaries and overriding styles. This preserves JSON syntax colors underneath for non-matching regions.
- The `ResponseBodyRenderCache` gains a `search_generation: u64` counter; when the search state changes, bump the generation to trigger a re-render of highlighted lines.

**Search operates on the raw (unwrapped) body text.** Match byte offsets are mapped to line/column positions in the pre-wrap lines. The wrapping pass carries highlight styles through to wrapped output.

**Edge cases:**
- Empty query on Enter: clears search state (same as Esc).
- No matches: show `0/0` in the search bar, no highlights.
- New response arrives: clear all search state (`query`, `matches`, `active`).
- Search only available on Body tab. `/` on Headers tab is a no-op.
- Very large responses: match computation is O(n) per keystroke. For responses > 1 MB, add debouncing if profiling shows > 50ms latency (not implemented upfront -- measure first).

**Layout change:**
- `ResponseLayout::new()` accepts a `search_active: bool` parameter.
- When true, adds `Constraint::Length(1)` at the bottom of `content_area`, splitting it into `content_area` + `search_bar_area`.

## Technical Considerations

**Render cache invalidation:**
The response body render cache has two layers: colorized lines and wrapped lines. Search highlighting adds a third concern. Rather than adding a full third cache layer, use a `search_generation` counter. When search state changes (new query, new match index, search cleared), bump the counter. The render function checks `search_generation` against the cache's stored value to decide whether to re-apply highlights. Highlights are applied on top of cached colorized lines, producing highlighted lines that are then wrapped.

**Performance for large responses:**
- Incremental search recomputes match positions on every keystroke. For typical API responses (< 100 KB), this is negligible. If profiling shows > 50ms latency on larger responses, add debouncing then.
- Highlight application iterates all matches and splits spans. For thousands of matches, this is O(matches * avg_spans_per_line). Acceptable for typical use.

**Keybinding conflicts:**
- `c` in Navigation mode on Response panel: currently unused. Safe.
- `S` in Navigation mode on Response panel: currently unused. Safe.
- `/` in Editing Normal mode: not handled by `transition_read_only()`, falls through to `Nop`. Safe to intercept before vim dispatch.
- `n`/`N` in Editing Normal mode: not handled by `transition_read_only()`. Safe to intercept.
- `Ctrl+I` during search input: not used elsewhere in text input contexts. Safe.

**`body_size_bytes` accuracy:**
Measuring size from `response.bytes().await` gives the decompressed wire size (reqwest decompresses gzip by default). This matches what Postman displays and is what users expect.

## Acceptance Criteria

### Response Size Display
- [x] `ResponseData` has `body_size_bytes: usize` populated from HTTP response
- [x] Tab bar shows size after duration: `"200 OK (245ms) · 1.2 KB"`
- [x] Size formatted correctly: `0 B`, `512 B`, `1.0 KB`, `2.5 MB`, `1.1 GB`
- [x] Size hidden when terminal inner width < 50 columns
- [x] No size shown for error/loading/empty/cancelled states

### One-Key Copy
- [x] `c` in Navigation mode (response focused) copies current tab content
- [x] Toast shows `"Copied response body (1.2 KB)"` or `"Copied response headers"`
- [x] `"No response to copy"` toast when no successful response exists
- [x] Copied text is the formatted (pretty-printed) body or rendered headers

### Save to File
- [x] `S` in Navigation mode (response focused) opens save popup
- [x] Center-screen popup with `TextInput` for path entry
- [x] Enter writes file, Esc cancels
- [x] Tilde expansion works (`~/output.json` resolves correctly)
- [x] Error toast on invalid path / permission error
- [x] Success toast with path and size
- [x] Save popup intercepts all keys (no bleed-through to vim/navigation)

### Response Body Search
- [x] `/` in Editing Normal mode on Body tab opens search bar
- [x] Search bar renders at bottom of response content area (1 row)
- [x] Typing incrementally highlights matches in the response body
- [x] Match count displayed as `current/total` in search bar
- [x] Enter confirms search, closes input bar, keeps highlights
- [x] Esc cancels search, clears query and highlights
- [x] `n` moves to next match (wraps around)
- [x] `N` moves to previous match (wraps around)
- [x] `Ctrl+I` toggles case sensitivity with visual indicator
- [x] Current match highlighted differently (LightRed) from other matches (Yellow)
- [x] Search highlights overlay JSON syntax colors correctly
- [x] Auto-scroll to current match position
- [x] Search state cleared when new response arrives
- [x] Search only active on Body tab; `/` is no-op on Headers tab
- [x] Help overlay updated with new search keybindings

### General
- [x] Help overlay (`?`) documents all new keybindings
- [x] Status bar hints updated for response-focused context
- [ ] All features work correctly in both narrow (80 cols) and wide terminals
- [ ] No regressions in existing response panel behavior

## Success Metrics

- All four sub-features are accessible via documented keybindings without leaving the keyboard.
- Response search finds matches in < 50ms for typical API responses (< 100 KB).
- No frame drops or visible lag when searching in responses up to 1 MB.
- Zero clipboard/file-write failures on macOS and Linux with valid paths.

## Dependencies & Risks

| Dependency / Risk | Impact | Mitigation |
|-------------------|--------|------------|
| Search highlighting must integrate with JSON colorization pipeline | High complexity -- span splitting is fiddly | Implement size display and copy/save first; tackle search last with dedicated testing |
| `ResponseLayout` changes affect all response rendering | Medium -- layout change could break existing rendering | Conditional layout (search_active flag), thorough visual testing |
| `body_size_bytes` requires change to `http.rs` response handling | Low -- isolated change | Call `.bytes().await`, measure, then convert to String manually |
| `response.bytes()` consumes the response in reqwest | Low -- requires rewriting the response-read path | Replace `.text().await` with `.bytes().await` + `String::from_utf8_lossy()` |

## References & Research

### Internal References
- Response data struct: `src/app.rs:92-98` (`ResponseData`)
- Response tab bar rendering: `src/ui/mod.rs:1206-1244` (`render_response_tab_bar`)
- Response body rendering + caching: `src/ui/mod.rs:1278-1323` (`render_response_body`)
- JSON colorization: `src/ui/mod.rs:1376-1510` (`colorize_json`)
- Word wrapping with cursor: `src/ui/mod.rs:1528-1597` (`render_wrapped_response_cached`)
- Span wrapping with selection highlights: `src/ui/mod.rs:1656-1704` (`wrap_line_spans_with_cursor`)
- Clipboard provider: `src/clipboard.rs` (`ClipboardProvider`)
- Clipboard toast pattern: `src/app.rs:2150-2153` (`copy_selected_path`)
- Sidebar search pattern: `src/app.rs:1818-1887` (sidebar `/` search with `TextInput`)
- TextInput struct: `src/app.rs:431-475`
- Read-only vim handler: `src/vim.rs:460-644` (`handle_read_only_normal_visual_operator`)
- HTTP response handling: `src/http.rs:192` (`response.text().await`)
- Response arrival in event loop: `src/app.rs:2731-2767`
- Key dispatch: `src/app.rs:2823-2828` (`handle_key`)
- Status bar: `src/ui/mod.rs:1717-1840` (`render_status_bar`)
- Help overlay: `src/ui/mod.rs:1861+`
- ResponseLayout: `src/ui/layout.rs:118-139`

### Brainstorm Reference
- Feature spec: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` lines 92-99
