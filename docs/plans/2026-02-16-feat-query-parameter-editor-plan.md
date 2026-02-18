---
title: "feat: Add query parameter editor with bidirectional URL sync"
type: feat
date: 2026-02-16
---

# feat: Add Query Parameter Editor

## Overview

Add a dedicated Params tab to the request panel with a key-value table editor for URL query parameters. Features bidirectional sync between the KV table and the URL field, toggle to enable/disable individual params without deleting them, and Postman Collection v2.1 compatible storage using the structured `url.query` format.

## Problem Statement

Perseus currently requires users to manually type query parameters directly into the URL field (e.g., `https://api.example.com/search?q=test&page=1&limit=20`). This creates friction:

| Gap | Impact |
|-----|--------|
| No structured param editing | Users must type `?key=value&key=value` manually, error-prone for complex queries |
| No param toggling | To temporarily exclude a param, users must delete it from the URL and remember to re-add it later |
| No visibility into params | Long URLs with many params are hard to read and edit in a single-line URL field |
| No Postman query compatibility | Postman collections with structured `url.query` arrays lose param metadata (disabled state, descriptions) on import |
| URL encoding burden | Users must manually URL-encode special characters in param values |

## Proposed Solution

A five-phase implementation, each phase independently compilable and committable:

1. **Phase A**: URL parsing utility module + query param data model
2. **Phase B**: Params tab integration (RequestTab, RequestField, navigation, focus)
3. **Phase C**: KV table rendering + cell editing for params
4. **Phase D**: Bidirectional sync (KV-to-URL + URL-to-KV)
5. **Phase E**: Postman storage upgrade + save/load integration

## Technical Approach

### Current Architecture

```
User types URL: https://api.example.com/search?q=test&page=1
    │
    ▼
url_editor: TextArea<'static>    ← Single text field, no structure
    │
    ▼
send_request() → raw_url = self.request.url_text()
    │
    ▼
http::send_request(&client, &method, &url, ...)   ← URL sent as-is
    │
    ▼
PostmanRequest { url: Value::String("https://...?q=test&page=1") }
                                    ← Stored as plain string
```

### Target Architecture

```
User edits URL OR KV table
    │                    │
    ▼                    ▼
url_editor          query_params: Vec<KvPair>
    │                    │
    ├── URL edit → Esc → parse_query_params() → update KV table
    │                    │
    └── KV edit/toggle → rebuild_url_query_string() → update URL
    │
    ▼
send_request() → raw_url = self.request.url_text()
                 ← URL always has current enabled params
    │
    ▼
PostmanRequest {
    url: Value::Object {
        "raw": "https://api.example.com/search?q=test&page=1",
        "query": [
            { "key": "q", "value": "test" },
            { "key": "page", "value": "1" },
            { "key": "debug", "value": "true", "disabled": true }
        ]
    }
}   ← Structured storage with disabled params preserved
```

### Key Architectural Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Source of truth | URL string is source of truth | KV table is a structured view/editor of the URL query string. URL always contains enabled params. Matches Postman behavior. |
| Disabled params storage | `query_params: Vec<KvPair>` on `RequestState` + Postman `url.query` array | Disabled params cannot exist in the URL string, so they live in the KV table and the Postman `query` array. |
| KV-to-URL sync trigger | Immediate on every KV mutation | Cell edit confirm, row add/delete, toggle all immediately update the URL string. User sees the URL change in real-time. |
| URL-to-KV sync trigger | On leaving URL editing mode (Esc) | Parsing mid-keystroke would be disruptive. Sync fires when user finishes URL editing. |
| Tab position | First: Params \| Headers \| Auth \| Body | Params relates directly to the URL input above. First position creates spatial consistency. |
| KV table pattern | Reuse `KvPair`, `KvFocus`, `KvColumn` from body types | Same component pattern. Separate `params_kv_focus` in `FocusState` avoids state pollution with body KV editors. |
| URL encoding | KV table shows decoded values | Users type `hello world`, URL shows `hello%20world`. Decoding happens on URL-to-KV parse; encoding on KV-to-URL rebuild. |
| Fragment preservation | Fragment is preserved during sync | `https://host/path?q=test#section` — the `#section` fragment is kept intact when rebuilding the query string. |
| Param reordering | Deferred | Not supported in initial implementation. Can be added later with Shift+J/K keybindings. |
| Auth API Key visibility | Not shown in Params tab | Consistent with auth headers being invisible in the Headers tab. Injected at send time only. |
| URL parser | Standalone `src/url.rs` module | Avoids adding the `url` crate dependency. Query param parsing is simple string splitting. |

### Key Files and Touchpoints

| File | What Changes |
|------|-------------|
| `src/url.rs` | **New file** — URL parsing utilities (extract base, parse query, rebuild URL) |
| `src/app.rs:67-73` | Add `RequestTab::Params` variant and update `request_tab_from_str`, `request_tab_to_str` |
| `src/app.rs:411-419` | Add `RequestField::Params` variant |
| `src/app.rs:421-428` | Add `params_kv_focus: KvFocus` to `FocusState` |
| `src/app.rs:534-550` | Add `query_params: Vec<KvPair>` to `RequestState` |
| `src/app.rs:607-629` | Update `set_contents()` to reset query params |
| `src/app.rs:700-715` | Update `active_editor()` for Params KV cell editing |
| `src/app.rs:3281-3291` | Update `is_editable_field()` for Params |
| `src/app.rs:3293-3336` | Update `next_horizontal()`, `prev_horizontal()` for Params |
| `src/app.rs:3338-3385` | Update `next_vertical()`, `prev_vertical()` for Params |
| `src/app.rs:3387-3403` | Update `next_request_tab()`, `prev_request_tab()` for 4-tab cycling |
| `src/app.rs:3405-3416` | Update `sync_field_to_tab()` for Params |
| `src/app.rs:1432-1464` | Update `build_postman_request()` to emit structured URL with query array |
| `src/app.rs:1466-1489` | Update `open_request()` to read query array from Postman URL |
| `src/app.rs:3208-3240` | Update `send_request()` if needed (URL already has params, so minimal change) |
| `src/app.rs:2162-2188` | Update `prepare_editors()` for params KV cell editing |
| `src/ui/mod.rs:576-614` | Update `render_request_panel()` to render Params tab content |
| `src/ui/mod.rs:616-679` | Update `render_request_tab_bar()` to include Params tab |
| `src/storage/postman.rs:54-64` | Add `PostmanQueryParam` struct, update `PostmanRequest::new()` for structured URL |
| `src/main.rs` | Add `mod url;` declaration |

---

## Implementation Phases

### Phase A: URL Parsing Utility + Query Param Data Model

Build the URL parsing module and extend `RequestState` with query param state.

**A.1: Create `src/url.rs` — URL parsing utility module**

- [ ] Create `src/url.rs` with the following functions:

  ```rust
  /// Split a URL into (base, query_string, fragment).
  /// base: everything before '?'
  /// query_string: between '?' and '#' (without the '?')
  /// fragment: after '#' (without the '#')
  ///
  /// Examples:
  ///   "https://api.com/search?q=test#top"
  ///     → ("https://api.com/search", Some("q=test"), Some("top"))
  ///   "https://api.com/search"
  ///     → ("https://api.com/search", None, None)
  ///   "https://api.com/search?"
  ///     → ("https://api.com/search", Some(""), None)
  pub fn split_url(url: &str) -> (&str, Option<&str>, Option<&str>)
  ```

  ```rust
  /// Parse a query string into key-value pairs.
  /// Splits on '&', then each pair on the first '='.
  /// URL-decodes both key and value.
  ///
  /// Examples:
  ///   "q=test&page=1" → [("q", "test"), ("page", "1")]
  ///   "data=a%3Db%3Dc" → [("data", "a=b=c")]  (decoded)
  ///   "key=" → [("key", "")]
  ///   "key" → [("key", "")]  (no value, treat as empty)
  ///   "" → []
  ///   "&&&" → []  (skip empty segments)
  pub fn parse_query_string(query: &str) -> Vec<(String, String)>
  ```

  ```rust
  /// Rebuild a full URL from base + enabled params + optional fragment.
  /// URL-encodes keys and values.
  ///
  /// Example:
  ///   build_url("https://api.com/search",
  ///     &[("q", "hello world"), ("page", "1")],
  ///     Some("top"))
  ///   → "https://api.com/search?q=hello%20world&page=1#top"
  pub fn build_url(base: &str, params: &[(&str, &str)], fragment: Option<&str>) -> String
  ```

  ```rust
  /// URL-encode a string (percent-encoding for query param components).
  /// Encodes everything except unreserved characters (A-Z, a-z, 0-9, '-', '_', '.', '~').
  /// Spaces encoded as %20 (not +, to match URL standard).
  /// Preserves {{variable}} template markers without encoding.
  pub fn percent_encode(input: &str) -> String
  ```

  ```rust
  /// URL-decode a percent-encoded string.
  /// Decodes %XX sequences back to characters.
  /// Decodes '+' as space (for compatibility with form-encoded strings).
  /// Preserves {{variable}} template markers without decoding.
  pub fn percent_decode(input: &str) -> String
  ```

- [ ] Handle edge cases:
  - `{{variable}}` markers in keys/values: preserve them as-is during encode/decode (match `{{...}}` pattern and skip encoding within those markers)
  - Empty query string (`?` with nothing after): return empty vec
  - Malformed pairs (`&&&`, `=value`, `key=`): skip empty segments, handle gracefully
  - Value with `=` in it (`data=a=b=c`): split on first `=` only → key=`data`, value=`a=b=c`
  - Fragment after query (`?q=test#section`): fragment preserved separately

**A.2: Add `mod url;` to `src/main.rs`**

- [ ] Add `mod url;` to the module declarations in `src/main.rs`
- [ ] Make functions `pub` so `src/app.rs` can use them

**A.3: Add `query_params` to `RequestState` (`src/app.rs:534-550`)**

- [ ] Add field to `RequestState`:
  ```rust
  pub query_params: Vec<KvPair>,
  ```

- [ ] Initialize in `RequestState::new()`:
  ```rust
  query_params: vec![KvPair::new_empty()],
  ```

**A.4: Add `params_kv_focus` to `FocusState` (`src/app.rs:421-428`)**

- [ ] Add field to `FocusState`:
  ```rust
  pub params_kv_focus: KvFocus,
  ```

- [ ] Default: `params_kv_focus: KvFocus::default()` (row 0, column Key)

**A.5: Add temporary KV edit TextArea for params to `App`**

- [ ] Add `kv_edit_textarea: Option<TextArea<'static>>` to `App` struct (does not exist yet — the body types plan describes it but hasn't been implemented). Initialize to `None`. This field is shared between Params and Body KV editors since only one can be active at a time.

**A.6: Update `set_contents()` (`src/app.rs:607-629`)**

- [ ] Reset query params when loading new request content:
  ```rust
  self.query_params = vec![KvPair::new_empty()];
  ```

**A.7: Compile and verify**

- [ ] Compile — new module exists, data model extended, no wiring yet
- [ ] All existing functionality unchanged

**Commit**: `feat(url): add URL parsing utilities and query param data model`

---

### Phase B: Params Tab + Navigation Integration

Wire the Params tab into the tab system, field navigation, and focus management.

**B.1: Add `RequestTab::Params` variant (`src/app.rs:67-73`)**

- [ ] Extend enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum RequestTab {
      Params,
      #[default]
      Headers,
      Auth,
      Body,
  }
  ```
  Note: `#[default]` stays on `Headers` intentionally. Params is first *visually* in the tab bar, but `Headers` remains the default landing tab for new sessions, first launch, and backward compatibility (existing sessions without "Params" in saved state fall through to `Headers`).

- [ ] Update `request_tab_from_str()`:
  ```rust
  fn request_tab_from_str(value: &str) -> RequestTab {
      match value {
          "Params" => RequestTab::Params,
          "Auth" => RequestTab::Auth,
          "Body" => RequestTab::Body,
          _ => RequestTab::Headers,
      }
  }
  ```

- [ ] Update `request_tab_to_str()`:
  ```rust
  fn request_tab_to_str(value: RequestTab) -> &'static str {
      match value {
          RequestTab::Params => "Params",
          RequestTab::Headers => "Headers",
          RequestTab::Auth => "Auth",
          RequestTab::Body => "Body",
      }
  }
  ```

**B.2: Add `RequestField::Params` variant (`src/app.rs:411-419`)**

- [ ] Extend enum:
  ```rust
  pub enum RequestField {
      Method,
      #[default]
      Url,
      Send,
      Params,
      Headers,
      Auth,
      Body,
  }
  ```

**B.3: Update tab cycling (`src/app.rs:3387-3403`)**

- [ ] Update `next_request_tab()` for 4-tab cycle:
  ```rust
  fn next_request_tab(&mut self) {
      self.request_tab = match self.request_tab {
          RequestTab::Params => RequestTab::Headers,
          RequestTab::Headers => RequestTab::Auth,
          RequestTab::Auth => RequestTab::Body,
          RequestTab::Body => RequestTab::Params,
      };
      self.sync_field_to_tab();
  }
  ```

- [ ] Update `prev_request_tab()`:
  ```rust
  fn prev_request_tab(&mut self) {
      self.request_tab = match self.request_tab {
          RequestTab::Params => RequestTab::Body,
          RequestTab::Headers => RequestTab::Params,
          RequestTab::Auth => RequestTab::Headers,
          RequestTab::Body => RequestTab::Auth,
      };
      self.sync_field_to_tab();
  }
  ```

**B.4: Update `sync_field_to_tab()` (`src/app.rs:3405-3416`)**

- [ ] Add Params mapping:
  ```rust
  fn sync_field_to_tab(&mut self) {
      if self.focus.panel == Panel::Request {
          self.focus.request_field = match self.focus.request_field {
              RequestField::Params | RequestField::Headers
              | RequestField::Auth | RequestField::Body => {
                  match self.request_tab {
                      RequestTab::Params => RequestField::Params,
                      RequestTab::Headers => RequestField::Headers,
                      RequestTab::Auth => RequestField::Auth,
                      RequestTab::Body => RequestField::Body,
                  }
              }
              other => other,
          };
      }
  }
  ```

**B.5: Update vertical navigation (`src/app.rs:3338-3385`)**

- [ ] In `next_vertical()`, add `RequestField::Params` alongside Headers/Auth/Body:
  ```rust
  RequestField::Method | RequestField::Url | RequestField::Send => {
      match self.request_tab {
          RequestTab::Params => RequestField::Params,
          RequestTab::Headers => RequestField::Headers,
          RequestTab::Auth => RequestField::Auth,
          RequestTab::Body => RequestField::Body,
      }
  }
  RequestField::Params | RequestField::Headers
  | RequestField::Auth | RequestField::Body => {
      self.focus.panel = Panel::Response;
      return;
  }
  ```

- [ ] In `prev_vertical()`, same pattern — add `RequestField::Params` to the content field group

**B.6: Update horizontal navigation (`src/app.rs:3293-3336`)**

- [ ] In `next_horizontal()` and `prev_horizontal()`, add `RequestField::Params` to the group that navigates to `RequestField::Url`:
  ```rust
  RequestField::Params | RequestField::Headers
  | RequestField::Auth | RequestField::Body => {
      RequestField::Url
  }
  ```

**B.7: Update `is_editable_field()` (`src/app.rs:3281-3291`)**

- [ ] Add `RequestField::Params` — not directly editable (KV cell editing uses temp TextArea):
  ```rust
  RequestField::Params => false, // KV cell editing handled separately
  ```

**B.8: Update `render_request_panel()` focus check (`src/ui/mod.rs:576-614`)**

- [ ] Add `RequestField::Params` to the `request_panel_focused` match:
  ```rust
  let request_panel_focused = app.focus.panel == Panel::Request
      && matches!(
          app.focus.request_field,
          RequestField::Params | RequestField::Headers
          | RequestField::Auth | RequestField::Body
      );
  ```

- [ ] Add Params tab rendering in the `match app.request_tab` block:
  ```rust
  RequestTab::Params => {
      // Placeholder for Phase C
      let placeholder = Paragraph::new("Query parameters (coming next phase)")
          .style(Style::default().fg(Color::DarkGray));
      frame.render_widget(placeholder, layout.content_area);
  }
  ```

**B.9: Update `render_request_tab_bar()` (`src/ui/mod.rs:616-679`)**

- [ ] Add Params tab to the tab bar, as the first tab:
  ```rust
  let tabs_line = Line::from(vec![
      Span::styled(
          "Params",
          if app.request_tab == RequestTab::Params {
              active_style
          } else {
              inactive_style
          },
      ),
      Span::styled(" | ", inactive_style),
      Span::styled(
          "Headers",
          if app.request_tab == RequestTab::Headers {
              active_style
          } else {
              inactive_style
          },
      ),
      // ... Auth and Body unchanged
  ]);
  ```

**B.10: Update all remaining `match` arms on `RequestField` and `RequestTab`**

- [ ] Search codebase for all `match` on `RequestField` and `RequestTab` — add Params variant to every match arm
- [ ] Key locations:
  - `active_editor()` on `RequestState` — return `None` for `RequestField::Params` (KV cell editing uses temp TextArea)
  - Key handler branches in `handle_key_event()` — add Params alongside other content fields
  - `prepare_editors()` — prepare params KV cell textarea when editing
  - Status bar hints — add Params-specific hints
  - Yank target matching — add Params

**B.11: Update status bar hints for Params**

- [ ] When `RequestField::Params` is focused:
  - Navigation mode: `"i/Enter: edit cell | a: add | d: delete | Space: toggle | Shift+H/L: switch tab"`
  - Editing mode: `"Esc: confirm | vim keys active"`

**B.12: Compile and verify**

- [ ] Compile — no warnings
- [ ] Manual test: Params tab visible in tab bar as first tab
- [ ] Manual test: Shift+H/L cycles through Params → Headers → Auth → Body → Params
- [ ] Manual test: j/k navigates from URL down to Params content area, then to Response
- [ ] Manual test: Params shows placeholder text
- [ ] Manual test: Session save/load preserves Params tab selection

**Commit**: `feat(app): add Params tab to request panel with navigation integration`

---

### Phase C: KV Table Rendering + Cell Editing

Build the params key-value table renderer and wire up cell editing.

**C.1: Create `render_params_panel()` in `src/ui/mod.rs`**

- [ ] Add function:
  ```rust
  fn render_params_panel(frame: &mut Frame, app: &App, area: Rect) {
      let params_focused = app.focus.panel == Panel::Request
          && app.focus.request_field == RequestField::Params;

      render_params_kv_table(
          frame,
          &app.request.query_params,
          app.focus.params_kv_focus,
          params_focused,
          app.app_mode == AppMode::Editing,
          &app.kv_edit_textarea,
          area,
      );
  }
  ```

- [ ] Replace placeholder in `render_request_panel()` with call to `render_params_panel()`

**C.2: Implement `render_params_kv_table()`**

- [ ] Render a table with 3 columns:
  ```
  ┌───┬──────────────────┬──────────────────┐
  │ ✓ │ Key              │ Value            │
  ├───┼──────────────────┼──────────────────┤
  │ ✓ │ q                │ test             │  ← Row 0, enabled
  │ ✓ │ page             │ 1                │  ← Row 1, enabled
  │ ✗ │ debug            │ true             │  ← Row 2, disabled (dimmed)
  │   │                  │                  │  ← Row 3, empty (for adding)
  └───┴──────────────────┴──────────────────┘
  ```

- [ ] Column layout with `Layout::horizontal()`:
  - Toggle column: `Constraint::Length(3)` — `✓` or `✗` indicator
  - Key column: `Constraint::Percentage(50)`
  - Value column: `Constraint::Percentage(50)`

- [ ] Rendering rules:
  - Active row (matching `params_kv_focus.row`): highlighted background (e.g., `Color::DarkGray` bg)
  - Active cell (matching row + column): brighter accent or underline
  - Disabled rows: dim foreground (`Color::DarkGray`) with strikethrough if supported
  - Empty trailing row: always present for adding new params
  - When editing a cell: render the `kv_edit_textarea` in place of the cell text

- [ ] Scroll support: calculate visible row range based on area height. Keep active row within visible range. Scroll offset tracked on `App` or computed from focus + area.

  ```rust
  // Params scroll offset (add to App struct)
  pub params_scroll_offset: usize,
  ```

  Scroll logic:
  ```rust
  let visible_rows = area.height as usize;
  if focus.row >= self.params_scroll_offset + visible_rows {
      self.params_scroll_offset = focus.row - visible_rows + 1;
  }
  if focus.row < self.params_scroll_offset {
      self.params_scroll_offset = focus.row;
  }
  ```

**C.3: Implement params KV navigation**

- [ ] When `RequestField::Params` in Navigation mode:
  - `j`/`Down`: move to next row. If at last row, wrap to first row (stay within the table).
  - `k`/`Up`: move to previous row. If at row 0, exit the KV table upward to the URL field via `prev_vertical()`.
  - Note: this asymmetry is deliberate — `k` at the top escapes to the URL (natural upward flow), while `j` at the bottom wraps (keeps focus in the table for rapid cycling). Matches how sidebar navigation works.
  - `Tab`/`l`: move to next column (Key → Value → next row Key)
  - `Shift+Tab`/`h`: move to previous column (Value → Key → prev row Value)
  - `Enter`/`i`: enter editing mode on current cell (create temp TextArea)
  - `a`/`o`: add new empty row after current, move focus to it
  - `d`: delete current row (if more than 1 non-empty row exists). If deleting leaves zero rows, add one empty row.
  - `Space`: toggle enabled/disabled on current row
  - `Shift+H`/`Shift+L`: switch tabs (existing behavior)

- [ ] Add key handling in `handle_key_event()`:
  - Before the general Navigation-mode handler, check for `in_request && self.focus.request_field == RequestField::Params`
  - Route j/k/h/l/Tab/Enter/i/a/d/Space to params-specific handlers

**C.4: Implement params KV cell editing**

- [ ] On `Enter`/`i` with a params cell focused:
  1. Read current cell text:
     ```rust
     let pair = &self.request.query_params[self.focus.params_kv_focus.row];
     let text = match self.focus.params_kv_focus.column {
         KvColumn::Key => pair.key.clone(),
         KvColumn::Value => pair.value.clone(),
     };
     ```
  2. Create temporary TextArea:
     ```rust
     let mut textarea = TextArea::new(vec![text]);
     configure_editor(&mut textarea, "");
     self.kv_edit_textarea = Some(textarea);
     self.app_mode = AppMode::Editing;
     self.vim = Vim::new(VimMode::Insert); // Start in insert mode for KV cells
     ```
  3. Vim editing applies to this TextArea (single-line behavior)

- [ ] On exit from editing (Esc → Normal → Esc → Navigation):
  1. Extract text from TextArea:
     ```rust
     let text = self.kv_edit_textarea.as_ref()
         .map(|ta| ta.lines().join(""))
         .unwrap_or_default();
     ```
  2. Write back to the appropriate `KvPair` field:
     ```rust
     let pair = &mut self.request.query_params[self.focus.params_kv_focus.row];
     match self.focus.params_kv_focus.column {
         KvColumn::Key => pair.key = text,
         KvColumn::Value => pair.value = text,
     }
     ```
  3. Clear temp textarea: `self.kv_edit_textarea = None;`
  4. Set `self.request_dirty = true`

- [ ] Note on single-line enforcement: KV cells are single-line. In the vim handler, intercept `Enter` in Insert mode to confirm the edit (exit to Navigation) rather than adding a newline. This can be done by checking if the active field is a KV cell and treating Enter as Esc in that context.

**C.5: Implement auto-append empty row**

- [ ] After any edit to the last row that makes it non-empty (key or value has text), automatically append a new empty `KvPair`:
  ```rust
  fn ensure_trailing_empty_row(params: &mut Vec<KvPair>) {
      if params.is_empty() || !params.last().unwrap().key.is_empty()
          || !params.last().unwrap().value.is_empty()
      {
          params.push(KvPair::new_empty());
      }
  }
  ```

- [ ] Call after every cell edit confirmation, row add, and row delete

**C.6: Update `prepare_editors()` for params KV editing**

- [ ] When `request_field == RequestField::Params` and `kv_edit_textarea.is_some()`:
  - Prepare the temp TextArea with cursor styles
  - Set block/border to match the cell being edited

**C.7: Update `active_editor()` for params KV editing**

- [ ] When `request_field == RequestField::Params` and `kv_edit_textarea.is_some()`:
  - Return `&mut kv_edit_textarea.as_mut().unwrap()`
  - This allows the vim state machine to operate on the temp TextArea

**C.8: Mark request dirty on params changes**

- [ ] Set `self.request_dirty = true` on:
  - Cell edit confirmation
  - Row add
  - Row delete
  - Toggle enable/disable

**C.9: Compile and verify**

- [ ] Compile — no warnings
- [ ] Manual test: Params tab shows KV table with one empty row
- [ ] Manual test: j/k navigates rows, h/l/Tab navigates columns
- [ ] Manual test: Enter on a cell → vim editing → type text → Esc → text appears in cell
- [ ] Manual test: `a` adds a new row, `d` deletes current row
- [ ] Manual test: `Space` toggles row enabled/disabled (visual dim)
- [ ] Manual test: Editing the last empty row auto-appends another empty row
- [ ] Manual test: Cannot delete the only remaining row

**Commit**: `feat(ui): add key-value table editor for query parameters`

---

### Phase D: Bidirectional Sync

Wire bidirectional synchronization between the KV table and the URL field.

**D.1: Implement KV-to-URL sync**

- [ ] Create `sync_params_to_url()` method on `App`:
  ```rust
  fn sync_params_to_url(&mut self) {
      let current_url = self.request.url_text();
      let (base, _, fragment) = url::split_url(&current_url);

      // Collect enabled, non-empty params
      let enabled_params: Vec<(&str, &str)> = self.request.query_params.iter()
          .filter(|p| p.enabled && !p.key.is_empty())
          .map(|p| (p.key.as_str(), p.value.as_str()))
          .collect();

      let new_url = url::build_url(base, &enabled_params, fragment);

      // Update URL editor only if changed (avoid cursor disruption)
      if new_url != current_url {
          self.request.url_editor = TextArea::new(vec![new_url]);
          configure_editor(&mut self.request.url_editor, "Enter URL...");
      }
  }
  ```

- [ ] Call `sync_params_to_url()` after every KV mutation:
  - After cell edit confirmation (in the Esc handler)
  - After row add (`a`/`o` key)
  - After row delete (`d` key)
  - After toggle (`Space` key)

**D.2: Implement URL-to-KV sync**

- [ ] Create `sync_url_to_params()` method on `App`:
  ```rust
  fn sync_url_to_params(&mut self) {
      let url = self.request.url_text();
      let (_, query_string, _) = url::split_url(&url);

      let parsed_params = match query_string {
          Some(qs) if !qs.is_empty() => url::parse_query_string(qs),
          _ => vec![],
      };

      // Merge strategy: replace KV table with parsed params.
      // Preserve disabled params that are NOT in the URL (user disabled them).
      // This is the key merge logic:
      //
      // 1. Start with parsed params (all enabled)
      // 2. Re-add any previously disabled params that aren't in the parsed set
      //    (they were disabled and thus not in the URL)
      //
      // This preserves disabled params across URL edits, as long as the user
      // doesn't add a param with the same key as a disabled one.

      let disabled_params: Vec<KvPair> = self.request.query_params.iter()
          .filter(|p| !p.enabled && !p.key.is_empty())
          .cloned()
          .collect();

      let mut new_params: Vec<KvPair> = parsed_params.into_iter()
          .map(|(key, value)| KvPair {
              key,
              value,
              enabled: true,
          })
          .collect();

      // Re-add disabled params at the end.
      // Note: this moves disabled params to the bottom after a URL edit.
      // Acceptable for MVP since param reordering is deferred.
      for disabled in disabled_params {
          new_params.push(disabled);
      }

      // Ensure trailing empty row
      new_params.push(KvPair::new_empty());

      self.request.query_params = new_params;
  }
  ```

- [ ] Call `sync_url_to_params()` when the user exits URL editing mode:
  - In the key handler, when transitioning from Editing → Navigation on the URL field (Esc handling for URL), call `sync_url_to_params()`
  - Specifically: after `app_mode` changes from `Editing` to `Navigation` and `request_field == RequestField::Url`

**D.3: Handle initial load sync**

- [ ] When `open_request()` loads a request and sets the URL, call `sync_url_to_params()` to populate the KV table from the URL
- [ ] If the Postman request has a structured `url.query` array, prefer loading from that (Phase E handles this — for now, parse from the URL string)

**D.4: Handle edge cases in sync**

- [ ] **Fragment preservation**: `build_url()` takes an optional fragment parameter. `split_url()` extracts it. The fragment is preserved across all sync operations.

- [ ] **Environment variables**: `{{var}}` markers in param keys/values are preserved as-is. The `percent_encode()` function skips encoding within `{{...}}` markers. At send time, environment substitution resolves them normally (substitution happens on the full URL string, which already contains the params).

- [ ] **Empty params**: A param with an empty key but non-empty value is skipped during KV-to-URL sync. A param with a non-empty key but empty value produces `key=` in the URL.

- [ ] **Duplicate keys**: Supported. Multiple rows with the same key produce `key=v1&key=v2` in the URL.

- [ ] **URL encoding round-trip**: KV table stores decoded values. `build_url()` encodes them. `parse_query_string()` decodes them. Round-trip: `hello world` → URL `hello%20world` → KV `hello world`. No double-encoding.

**D.5: Compile and verify**

- [ ] Compile
- [ ] Manual test: **KV-to-URL** — Add params `q=test` and `page=1` in KV table → URL shows `?q=test&page=1`
- [ ] Manual test: **URL-to-KV** — Type `?name=John&age=30` in URL → Esc → KV table shows 2 rows
- [ ] Manual test: **Toggle** — Disable `page=1` in KV → URL updates to `?q=test` → Re-enable → URL shows `?q=test&page=1`
- [ ] Manual test: **Fragment** — URL `https://api.com/path?q=test#section` → add param `page=1` → URL shows `?q=test&page=1#section`
- [ ] Manual test: **Encoding** — Type `hello world` as value → URL shows `hello%20world` → Edit URL to `hello%20world` → Esc → KV shows `hello world`
- [ ] Manual test: **Environment vars** — Type `{{token}}` as value → URL shows `{{token}}` (not encoded) → KV shows `{{token}}`
- [ ] Manual test: **Duplicate keys** — Add two rows with key `tag`, values `a` and `b` → URL shows `?tag=a&tag=b`
- [ ] Manual test: **Empty URL** — URL has no `?` → KV table has one empty row

**Commit**: `feat(app): add bidirectional sync between query params and URL`

---

### Phase E: Postman Storage Upgrade + Save/Load Integration

Upgrade URL storage to Postman structured format with `query` array for full persistence.

**E.1: Add `PostmanQueryParam` struct to `src/storage/postman.rs`**

- [ ] Add struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanQueryParam {
      pub key: String,
      pub value: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub disabled: Option<bool>,
  }
  ```

**E.2: Add URL builder helpers to `src/storage/postman.rs`**

- [ ] Add function to build structured Postman URL value:
  ```rust
  pub fn build_postman_url(raw: &str, query: Vec<PostmanQueryParam>) -> Value {
      if query.is_empty() {
          // No query params — store as plain string for simplicity
          Value::String(raw.to_string())
      } else {
          let mut map = serde_json::Map::new();
          map.insert("raw".to_string(), Value::String(raw.to_string()));
          map.insert(
              "query".to_string(),
              serde_json::to_value(&query).unwrap_or(Value::Array(vec![])),
          );
          Value::Object(map)
      }
  }
  ```

- [ ] Add function to extract query params from Postman URL value:
  ```rust
  pub fn extract_query_params(url_value: &Value) -> Vec<PostmanQueryParam> {
      match url_value {
          Value::Object(map) => {
              map.get("query")
                  .and_then(|v| serde_json::from_value::<Vec<PostmanQueryParam>>(v.clone()).ok())
                  .unwrap_or_default()
          }
          _ => vec![], // String URLs have no structured query params
      }
  }
  ```

**E.3: Update `build_postman_request()` (`src/app.rs:1432-1464`)**

- [ ] Build structured URL with query array:
  ```rust
  fn build_postman_request(&self) -> PostmanRequest {
      let method = self.request.method.as_str().to_string();
      let url_raw = self.request.url_text();
      let headers = storage::parse_headers(&self.request.headers_text());

      // Build query params for Postman storage
      let query_params: Vec<storage::PostmanQueryParam> = self.request.query_params.iter()
          .filter(|p| !p.key.is_empty()) // Skip empty rows
          .map(|p| storage::PostmanQueryParam {
              key: p.key.clone(),
              value: p.value.clone(),
              disabled: if p.enabled { None } else { Some(true) },
          })
          .collect();

      // ... body and auth unchanged ...

      let mut req = PostmanRequest::new(method, url_raw, headers, body);
      req.auth = auth;

      // Upgrade URL to structured format if we have query params
      if !query_params.is_empty() {
          req.url = storage::build_postman_url(&self.request.url_text(), query_params);
      }

      req
  }
  ```

**E.4: Update `open_request()` (`src/app.rs:1466-1489`)**

- [ ] Load query params from Postman URL:
  ```rust
  fn open_request(&mut self, request_id: Uuid) {
      // ... existing code to load request ...

      if let Some(request) = request_data {
          let method = Method::from_str(&request.method);
          let url = extract_url(&request.url);
          let headers = headers_to_text(&request.header);
          let body = request.body.as_ref()
              .and_then(|b| b.raw.clone())
              .unwrap_or_default();
          self.request.set_contents(method, url, headers, body);
          self.load_auth_from_postman(&request);

          // Reset params focus and scroll for the new request
          self.focus.params_kv_focus = KvFocus::default();
          self.params_scroll_offset = 0;

          // Load query params from structured URL
          let postman_params = storage::extract_query_params(&request.url);
          if !postman_params.is_empty() {
              self.request.query_params = postman_params.iter()
                  .map(|p| KvPair {
                      key: p.key.clone(),
                      value: p.value.clone(),
                      enabled: !p.disabled.unwrap_or(false),
                  })
                  .collect();
              // Ensure trailing empty row
              self.request.query_params.push(KvPair::new_empty());
          } else {
              // No structured query params — parse from URL string
              self.sync_url_to_params();
          }

          // ... rest of open_request unchanged ...
      }
  }
  ```

**E.5: Backward compatibility**

- [ ] Existing collections with URL as plain string (`Value::String`):
  - `extract_url()` already handles this (returns the string)
  - `extract_query_params()` returns empty vec for string URLs
  - Query params are parsed from the URL string via `sync_url_to_params()`
  - On next save, the URL is upgraded to structured format if params exist

- [ ] Collections with structured URL object but no `query` field:
  - `extract_query_params()` returns empty vec
  - Falls back to URL string parsing

- [ ] Collections with both `raw` and `query` that are inconsistent:
  - `query` array is authoritative (matches Postman behavior)
  - `raw` is used for the URL field display, but query params come from `query`
  - On next save, `raw` is rebuilt from base URL + enabled params

**E.6: Test save/load roundtrip**

- [ ] Manual test: **Save with params** — Add params `q=test`, `page=1`, disable `debug=true` → save → verify collection JSON has structured URL with `query` array
- [ ] Manual test: **Load with params** — Reopen saved request → KV table shows all 3 params (2 enabled, 1 disabled)
- [ ] Manual test: **Backward compat** — Open old collection (URL as plain string with `?q=test`) → KV table shows `q=test` (parsed from URL string)
- [ ] Manual test: **Disabled round-trip** — Save request with disabled param → reopen → param still disabled in KV table
- [ ] Manual test: **New request** — Create new request → save with no params → URL stored as plain string (not structured object)
- [ ] Manual test: **Request switch** — Switch between two requests with different params → each loads correctly

**E.7: Compile and verify end-to-end**

- [ ] Compile with no warnings
- [ ] All existing tests pass
- [ ] End-to-end: create request → add params via KV → URL updates → send request → params included → save → reopen → params restored with enabled/disabled state

**Commit**: `feat(storage): upgrade URL storage to Postman structured format with query params`

---

## Alternative Approaches Considered

| Approach | Why Rejected |
|----------|-------------|
| KV table as source of truth (URL shows no query string) | Confusing — users expect to see the full URL including query params. Also breaks copy-paste workflow where users paste a full URL and expect it to work. |
| Dual source with last-write-wins | Too complex. Race conditions between URL and KV edits. No clear mental model for users about which "wins". |
| Use `url` crate for parsing | Adds a dependency for simple string splitting. Perseus URLs may contain `{{var}}` template markers that the `url` crate would reject as invalid. Custom parser handles these gracefully. |
| Sync on every keystroke in URL editor | Disruptive — mid-typing, the KV table would flicker. `?q=t` → `?q=te` → `?q=tes` → `?q=test` would cause 4 KV table updates. Sync on Esc is cleaner. |
| Separate `ParamsField` enum for sub-navigation (like `BodyField`) | Unnecessary — params are always a KV table, unlike Body which has mode selector + content area. `KvFocus` (row + column) is sufficient. |
| Show API Key query param in Params tab | Breaks the separation between auth and params. Auth headers are also invisible in the Headers tab. Consistency wins over discoverability. |
| Inline param editing in the URL (highlight param segments) | Complex to implement, poor UX for long URLs. A dedicated KV table is clearer and more accessible. |

## Acceptance Criteria

### Functional Requirements

- [ ] Params tab visible in request panel as the first tab: `Params | Headers | Auth | Body`
- [ ] KV table editor with key-value columns and enable/disable toggle
- [ ] Adding params in KV table immediately updates the URL field
- [ ] Editing the URL and pressing Esc populates the KV table from the URL query string
- [ ] Toggling a param disabled removes it from the URL; re-enabling adds it back
- [ ] Sending a request includes only enabled params in the URL
- [ ] Query params persist in Postman Collection v2.1 format with `url.query` array
- [ ] Disabled params preserved across save/load

### Data Integrity

- [ ] Save → reload roundtrip preserves all query params (enabled and disabled)
- [ ] Backward compatible: collections with plain string URLs load correctly (params parsed from URL)
- [ ] Fragment (`#anchor`) preserved during all sync operations
- [ ] URL encoding: KV table shows decoded values; URL shows encoded values
- [ ] Duplicate keys supported (e.g., `?tag=a&tag=b`)
- [ ] `{{variable}}` templates in param values preserved without encoding

### UI/UX

- [ ] KV table renders with toggle, key, value columns
- [ ] Active row highlighted, active cell accented
- [ ] Disabled rows visually dimmed
- [ ] Trailing empty row always present for adding new params
- [ ] Full vim editing on KV cells (single-line mode)
- [ ] j/k for rows, h/l/Tab for columns, Enter/i to edit, a to add, d to delete, Space to toggle
- [ ] Shift+H/L tab cycling works from Params tab
- [ ] Status bar shows context-appropriate hints
- [ ] Scroll support for tables with many rows

### Edge Cases

- [ ] URL with no query string: KV table shows one empty row
- [ ] URL with empty query string (`path?`): KV table shows one empty row, trailing `?` removed on next sync
- [ ] Value containing `=`: correctly parsed (split on first `=` only)
- [ ] Malformed query string (`&&&`): empty segments skipped
- [ ] Empty key with non-empty value: skipped during KV-to-URL sync
- [ ] Very long URLs: KV table and URL both handle gracefully (scrolling)
- [ ] All params disabled: URL has no query string
- [ ] Deleting all KV rows: leaves one empty row, URL query string removed

### Quality Gates

- [ ] Compiles with no warnings
- [ ] All existing tests pass
- [ ] Each phase independently committed and functional
- [ ] URL parsing utilities have clear, documented behavior

---

## Dependencies & Prerequisites

| Dependency | Status | Notes |
|-----------|--------|-------|
| Body Types KV editor (Phase E of body plan) | Not yet implemented | Params builds its own KV rendering. If body KV editor is built first, consider extracting a shared component. Both use `KvPair`, `KvFocus`, `KvColumn` from `src/app.rs`. |
| `KvPair` struct | Already exists | Defined in `src/app.rs` — reused for query params |
| `KvFocus` / `KvColumn` | Already exists | Defined in `src/app.rs` — separate `params_kv_focus` added |
| Environment variables feature | Completed (current branch) | `{{var}}` substitution at send time works on the URL string, which already contains params |
| Auth feature | Completed | API Key with QueryParam location is independent — injected at send time by reqwest, not in the URL |

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Bidirectional sync bugs (URL and KV drift) | Medium | High | Clear source of truth (URL). Defined sync triggers. Sync on every KV mutation (immediate). Sync from URL only on Esc. |
| URL parsing edge cases | Medium | Medium | Comprehensive edge case handling in `src/url.rs`. `{{var}}` markers explicitly preserved. Fragment handling. |
| Tab order change confuses existing users | Low | Low | Params is first tab, so existing Headers/Auth/Body cycle is unchanged when starting from Headers. Old saved sessions default to Headers. |
| KV focus conflicts with body KV focus | Medium | Medium | Separate `params_kv_focus` field in `FocusState`. No shared state between Params and Body KV editors. |
| Postman storage format change breaks existing collections | Low | High | Backward compatible: `extract_url()` and `extract_query_params()` handle both string and object URL formats. New format only written when params exist. |
| Performance with many params (100+ rows) | Low | Low | Standard ratatui scroll handling. KV table renders only visible rows within the area height. |
| Double-encoding URLs (encode already-encoded values) | Medium | Medium | Clear encode/decode boundary: KV stores decoded, URL stores encoded. `parse_query_string()` always decodes. `build_url()` always encodes. |

## Future Considerations

- **Param reordering**: Add `Shift+J`/`Shift+K` to move rows up/down (useful for order-sensitive APIs)
- **Bulk param paste**: Paste `key=value\nkey=value` text to auto-populate multiple KV rows
- **Param description field**: Optional description per param (stored in Postman's `description` field)
- **Shared KV component**: Extract `render_kv_table()` into a reusable component shared between Params and Body form editors
- **Param count in tab label**: Show "Params (3)" in tab bar to indicate how many active params exist
- **URL bar visual indicator**: Dim/distinguish the query string portion of the URL to indicate it's managed by the KV editor

## References

### Internal References

- Brainstorm: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` — Phase 1.6
- Body Types plan (KV pattern): `docs/plans/2026-02-16-feat-request-body-types-plan.md` — Phases E-F define `KvRow` trait and `render_kv_table()`
- Auth plan (tab/popup pattern): `docs/plans/2026-02-15-feat-authentication-support-plan.md`
- Request state: `src/app.rs:534-550` — `RequestState` with url/headers/body editors
- Tab system: `src/app.rs:67-89` — `RequestTab` enum and cycling
- Navigation: `src/app.rs:3338-3416` — vertical/horizontal navigation and tab sync
- URL storage: `src/storage/postman.rs:54-64` — `PostmanRequest.url: Value`
- URL extraction: `src/app.rs:3706-3716` — `extract_url()` handles string and object URL
- KV data model: `src/app.rs:337-398` — `KvPair`, `KvFocus`, `KvColumn` already defined
- Send request: `src/app.rs:3208-3240` — URL text used directly (params already in URL via sync)

### External References

- Postman Collection v2.1 URL Schema: `url` field supports both string and structured object with `raw`, `host`, `path`, `query`, `variable` fields
- Postman `query` array format: `[{ "key": "name", "value": "value", "disabled": true }]`
- RFC 3986 (URI Generic Syntax): Query component follows `?`, fragment follows `#`
- Percent-encoding: RFC 3986 Section 2.1 — unreserved characters `A-Z a-z 0-9 - _ . ~` are not encoded
