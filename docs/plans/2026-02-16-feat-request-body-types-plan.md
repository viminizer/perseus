---
title: "feat: Add request body types (JSON, Form, Multipart, XML, Binary)"
type: feat
date: 2026-02-16
---

# feat: Add Request Body Types

## Overview

Add support for multiple request body types beyond raw text: JSON (with validation indicator and auto Content-Type), Form URL-encoded (key-value editor), Multipart Form Data (key-value with file attachments), XML (mode indicator and auto Content-Type), and Binary (file path input). Includes a body type selector popup, per-mode editor UI, Content-Type auto-injection at send time, and Postman Collection v2.1 compatible storage.

## Problem Statement

Perseus currently sends all request bodies as raw text. Users must manually set `Content-Type` headers and manually format form data. This creates friction for standard API workflows:

| Gap | Impact |
|-----|--------|
| No body type awareness | Users must manually type `Content-Type: application/json` for every JSON request |
| No form editor | Form URL-encoded and multipart data must be manually formatted as raw text (`key=value&key2=value2`) |
| No file uploads | Binary file sending and multipart file attachments are impossible |
| No JSON validation | Users get no feedback on whether their JSON body is syntactically valid until they receive a 400 error |
| No Postman body interop | Importing Postman collections with urlencoded/formdata/file bodies would lose structured data (when import is later implemented) |
| No body mode indicator | The Body tab shows the same editor regardless of content semantics — a JSON body is indistinguishable from raw text |

## Proposed Solution

An eight-phase implementation, each phase independently compilable and committable:

1. **Phase A**: Postman-compatible body data model (storage structs)
2. **Phase B**: `BodyMode` enum + in-memory state on `RequestState`
3. **Phase C**: Body type selector popup + mode switching
4. **Phase D**: Raw/JSON/XML text modes with Content-Type auto-setting
5. **Phase E**: Key-value pair data model + shared renderer
6. **Phase F**: Form URL-encoded mode
7. **Phase G**: Multipart form data mode (with file type fields)
8. **Phase H**: Binary file mode + save/load integration for all modes

## Technical Approach

### Current Architecture

```
User selects Body tab
    │
    ▼
render_request_panel()
    │
    ▼
RequestTab::Body → frame.render_widget(&app.request.body_editor, area)
                            │
                            ▼
                   Single TextArea<'static>
                            │
                            ▼
send_request() → body = self.request.body_text()
    │                       │
    ▼                       ▼
http::send_request(... body: &str ...)
    │
    ▼
builder.body(body.to_string())   ← Always raw text, no Content-Type
    │
    ▼
PostmanBody { mode: "raw", raw: Some(body) }   ← Storage: raw only
```

### Target Architecture

```
User selects Body tab
    │
    ▼
render_request_panel()
    │
    ▼
RequestTab::Body → render_body_panel(frame, app, area)
                      │
                      ├── Body mode selector row: [JSON ▾]
                      │
                      ├── Mode-specific editor:
                      │     ├── Raw/JSON/XML → TextArea (shared) + validation indicator
                      │     ├── FormUrlEncoded → Key-value table editor
                      │     ├── Multipart → Key-value table + file type per row
                      │     └── Binary → File path TextArea (single line)
                      │
                      ▼
send_request() → match body_mode {
    Raw       → builder.body(text)
    Json      → builder.header("Content-Type", "application/json").body(text)
    Xml       → builder.header("Content-Type", "application/xml").body(text)
    FormUrl   → builder.header("Content-Type", "application/x-www-form-urlencoded")
                       .body(encode_pairs(pairs))
    Multipart → builder.multipart(build_multipart_form(fields))
    Binary    → builder.body(read_file(path)?)  // read in async task
}
    │
    ▼
PostmanBody {
    mode: "raw" | "urlencoded" | "formdata" | "file",
    raw: Option<String>,
    options: Option<PostmanBodyOptions>,          ← language hint for raw modes
    urlencoded: Option<Vec<PostmanKvPair>>,       ← form pairs
    formdata: Option<Vec<PostmanFormParam>>,       ← multipart fields
    file: Option<PostmanFileRef>,                  ← binary file path
}
```

### Key Files and Touchpoints

| File | What Changes |
|------|-------------|
| `src/storage/postman.rs:74-79` | Extend `PostmanBody` with urlencoded, formdata, file, options fields |
| `src/app.rs:67-71` | No change to `RequestTab` (Body tab already exists) |
| `src/app.rs:414-418` | Add `body_mode`, form pairs, multipart fields, binary path editor to `RequestState` |
| `src/app.rs:523-525` | Update `body_text()` → `body_content()` that returns mode-aware content |
| `src/app.rs:566-571` | Update `active_editor()` for body sub-editors |
| `src/app.rs:1268-1276` | Update `build_postman_request()` to serialize body mode |
| `src/app.rs:2162-2188` | Update `prepare_editors()` for body mode-specific editors |
| `src/app.rs:2388-2463` | Add body type popup handling (before method popup check) |
| `src/app.rs:2950-2956` | Update `send_request()` for mode-aware body building |
| `src/http.rs:14-77` | Extend `send_request()` to accept `BodyContent` enum instead of `&str` |
| `src/ui/mod.rs:434-471` | Replace direct body_editor render with `render_body_panel()` |
| `src/ui/mod.rs:474-521` | Update tab bar to show "Body (JSON)" etc. |
| `src/ui/layout.rs` | Add `BodyLayout` for mode selector + content area |

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Shared TextArea for text modes | Single `body_editor` used by Raw, JSON, XML | Preserves content when switching between text modes. No data loss on Raw ↔ JSON ↔ XML. |
| Separate state for KV modes | `Vec<KvPair>` for urlencoded, `Vec<MultipartField>` for multipart | Structured data can't share a TextArea. Separate vectors allow independent state. |
| Content-Type injection | At send time, not stored in visible headers | Matches Postman behavior. Avoids conflict with user-set headers. Auto-injected header is invisible in the Headers tab. |
| Content-Type override behavior | Auto-set only if user hasn't manually set Content-Type in headers | Respect user's explicit header. Check headers text for existing Content-Type before injecting. |
| Key-value cell editing | Temporary TextArea for active cell | Avoids N*2 persistent TextAreas. Create TextArea on Enter, extract text on Esc. |
| KV pair enable/disable | `enabled: bool` per pair, toggle with Space | Matches Postman behavior. Users can disable params without deleting. |
| Body mode label in tab bar | "Body (JSON)" / "Body (Form)" / etc. | At-a-glance visibility of body mode. Matches auth tab pattern "Auth (Bearer)". |
| JSON validation | Visual indicator only (checkmark/X in mode selector row) | Don't block sending — user may intentionally send malformed JSON to test error handling. |
| JSON pretty-format | Not auto-applied on paste for MVP | Too magical. Can be added later as a keyboard shortcut. Content preservation is more important. |
| Multipart file type | Per-row type toggle (Text/File) | Matches Postman's multipart model where each field can be text or file. |
| Binary file validation | At send time only | File may not exist yet during editing. Show error in response area if file not found at send. |
| Body mode on type switch | Preserve all mode data in memory | Switching modes doesn't clear data. User can switch back without losing work. Only the active mode's data is sent. |
| Body popup trigger | Enter/i on body mode selector row (like auth type) | Consistent interaction pattern across method, auth type, and body mode selectors. |

---

## Implementation Phases

### Phase A: Postman-Compatible Body Data Model

Extend the storage structs to support all Postman v2.1 body modes.

**A.1: Add body-related structs to `src/storage/postman.rs`**

- [ ] Add `PostmanBodyOptions` struct (raw language hint):
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanBodyOptions {
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub raw: Option<PostmanRawLanguage>,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanRawLanguage {
      pub language: String, // "json", "xml", "text"
  }
  ```

- [ ] Add `PostmanKvPair` struct (for urlencoded):
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanKvPair {
      pub key: String,
      pub value: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub disabled: Option<bool>,
  }
  ```

- [ ] Add `PostmanFormParam` struct (for multipart formdata):
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanFormParam {
      pub key: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub value: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub src: Option<String>,       // file path for type="file"
      #[serde(rename = "type", default = "default_form_type")]
      pub param_type: String,        // "text" or "file"
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub disabled: Option<bool>,
  }

  fn default_form_type() -> String {
      "text".to_string()
  }
  ```

- [ ] Add `PostmanFileRef` struct (for binary):
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanFileRef {
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub src: Option<String>,
  }
  ```

**A.2: Extend `PostmanBody` struct (`src/storage/postman.rs:74-79`)**

- [ ] Add new fields to `PostmanBody`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanBody {
      pub mode: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub raw: Option<String>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub options: Option<PostmanBodyOptions>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub urlencoded: Option<Vec<PostmanKvPair>>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub formdata: Option<Vec<PostmanFormParam>>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub file: Option<PostmanFileRef>,
  }
  ```

**A.3: Update `PostmanRequest::new()` (`src/storage/postman.rs:121-142`)**

- [ ] Update the body construction to include `options: None`, `urlencoded: None`, `formdata: None`, `file: None`
- [ ] Keep existing raw body creation logic unchanged

**A.4: Add helper constructors on `PostmanBody`**

- [ ] `PostmanBody::raw(text: &str) -> PostmanBody` — mode "raw", no language
- [ ] `PostmanBody::json(text: &str) -> PostmanBody` — mode "raw" with options.raw.language = "json"
- [ ] `PostmanBody::xml(text: &str) -> PostmanBody` — mode "raw" with options.raw.language = "xml"
- [ ] `PostmanBody::urlencoded(pairs: Vec<PostmanKvPair>) -> PostmanBody` — mode "urlencoded"
- [ ] `PostmanBody::formdata(params: Vec<PostmanFormParam>) -> PostmanBody` — mode "formdata"
- [ ] `PostmanBody::file(path: &str) -> PostmanBody` — mode "file"

**A.5: Verify backward compatibility**

- [ ] Compile — no other code changes needed (new fields are `Option` with `serde(default)`)
- [ ] Existing collection JSON without new body fields deserializes correctly
- [ ] JSON with extended body fields round-trips correctly

**Commit**: `feat(storage): extend Postman body model for all body types`

---

### Phase B: BodyMode Enum + In-Memory State

Add the runtime body mode state model to `RequestState`.

**B.1: Define body mode enum in `src/app.rs`**

- [ ] Add `BodyMode` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum BodyMode {
      #[default]
      Raw,
      Json,
      Xml,
      FormUrlEncoded,
      Multipart,
      Binary,
  }
  ```

- [ ] Add constants on `BodyMode`:
  - `BodyMode::ALL: [BodyMode; 6]` — for popup rendering
  - `BodyMode::as_str(&self) -> &str` — "Raw", "JSON", "XML", "Form URL-Encoded", "Multipart Form", "Binary"
  - `BodyMode::from_index(usize) -> BodyMode`
  - `BodyMode::index(&self) -> usize`
  - `BodyMode::is_text_mode(&self) -> bool` — true for Raw, Json, Xml

**B.2: Define key-value pair structs**

- [ ] Add `KvPair` struct (shared by form modes):
  ```rust
  #[derive(Debug, Clone)]
  pub struct KvPair {
      pub key: String,
      pub value: String,
      pub enabled: bool,
  }

  impl KvPair {
      pub fn new_empty() -> Self {
          Self { key: String::new(), value: String::new(), enabled: true }
      }
  }
  ```

- [ ] Add `MultipartFieldType` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum MultipartFieldType {
      #[default]
      Text,
      File,
  }
  ```

- [ ] Add `MultipartField` struct:
  ```rust
  #[derive(Debug, Clone)]
  pub struct MultipartField {
      pub key: String,
      pub value: String,        // text value or file path
      pub field_type: MultipartFieldType,
      pub enabled: bool,
  }

  impl MultipartField {
      pub fn new_empty() -> Self {
          Self {
              key: String::new(),
              value: String::new(),
              field_type: MultipartFieldType::Text,
              enabled: true,
          }
      }
  }
  ```

**B.3: Define body focus state**

- [ ] Add `BodyField` enum (tracks focused sub-field within Body tab):
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum BodyField {
      #[default]
      ModeSelector,    // The mode selector row
      TextEditor,      // Raw/JSON/XML text area
      KvRow,           // Active row in key-value editor
      BinaryPath,      // File path input for binary mode
  }
  ```

- [ ] Add `KvColumn` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum KvColumn {
      #[default]
      Key,
      Value,
  }
  ```

- [ ] Add `KvFocus` struct (tracks position in key-value editors):
  ```rust
  #[derive(Debug, Clone, Copy, Default)]
  pub struct KvFocus {
      pub row: usize,
      pub column: KvColumn,
  }
  ```

**B.4: Add body mode state to `RequestState`**

- [ ] Add fields to `RequestState` (after existing `body_editor`):
  ```rust
  pub body_mode: BodyMode,
  // body_editor (existing TextArea) shared by Raw, JSON, XML
  pub body_form_pairs: Vec<KvPair>,                // Form URL-encoded
  pub body_multipart_fields: Vec<MultipartField>,   // Multipart
  pub body_binary_path_editor: TextArea<'static>,   // Binary file path
  ```

- [ ] Add body focus state to `FocusState`:
  ```rust
  pub body_field: BodyField,
  pub kv_focus: KvFocus,
  ```

- [ ] Add temporary editing TextArea to `App`:
  ```rust
  pub kv_edit_textarea: Option<TextArea<'static>>,  // Active when editing a KV cell
  ```

- [ ] Update `RequestState::new()`:
  - `body_mode: BodyMode::Raw`
  - `body_form_pairs: vec![KvPair::new_empty()]` (start with one empty row)
  - `body_multipart_fields: vec![MultipartField::new_empty()]`
  - `body_binary_path_editor: TextArea::default()` configured with placeholder "File path..."

- [ ] Update `FocusState::default()`:
  - `body_field: BodyField::ModeSelector`
  - `kv_focus: KvFocus::default()`

**B.5: Add text extraction methods**

- [ ] `body_binary_path_text(&self) -> String` on `RequestState`

**B.6: Compile and verify**

- [ ] Compile — body mode state exists but is not yet wired into UI or HTTP
- [ ] All existing functionality works unchanged (body_mode defaults to Raw, existing flow untouched)

**Commit**: `feat(app): add body mode state model with form pairs and multipart fields`

---

### Phase C: Body Type Selector Popup + Mode Switching

Wire the body mode selector popup into the Body tab.

**C.1: Add body type popup state to `App`**

- [ ] Add fields to `App`:
  ```rust
  pub show_body_mode_popup: bool,
  pub body_mode_popup_index: usize,
  ```

- [ ] Initialize in `App::new()`: both false/0

**C.2: Update Body tab rendering to include mode selector row**

- [ ] Add `BodyLayout` struct to `src/ui/layout.rs`:
  ```rust
  pub struct BodyLayout {
      pub mode_selector_area: Rect,   // 1 line: mode selector
      pub spacer_area: Rect,          // 1 line: separator
      pub content_area: Rect,         // remaining: mode-specific editor
  }

  impl BodyLayout {
      pub fn new(area: Rect) -> Self {
          let chunks = Layout::vertical([
              Constraint::Length(1),   // Mode selector
              Constraint::Length(1),   // Spacer
              Constraint::Min(3),      // Content
          ])
          .split(area);

          Self {
              mode_selector_area: chunks[0],
              spacer_area: chunks[1],
              content_area: chunks[2],
          }
      }
  }
  ```

- [ ] Create `render_body_panel()` in `src/ui/mod.rs`:
  ```rust
  fn render_body_panel(frame: &mut Frame, app: &App, area: Rect) {
      let layout = BodyLayout::new(area);

      // Render mode selector row
      render_body_mode_selector(frame, app, layout.mode_selector_area);

      // Render mode-specific content
      match app.request.body_mode {
          BodyMode::Raw | BodyMode::Json | BodyMode::Xml => {
              frame.render_widget(&app.request.body_editor, layout.content_area);
          }
          // KV modes and Binary: Phase E-H
          _ => {
              let placeholder = Paragraph::new("(not yet implemented)")
                  .style(Style::default().fg(Color::DarkGray));
              frame.render_widget(placeholder, layout.content_area);
          }
      }
  }
  ```

- [ ] Create `render_body_mode_selector()` — renders a single line showing current mode:
  ```
  Type: [JSON                ▾]
  ```
  - Highlight row when `body_field == BodyField::ModeSelector` and panel focused
  - Show mode name with dropdown indicator

- [ ] Update `render_request_panel()` (`src/ui/mod.rs:468-470`):
  - Replace `frame.render_widget(&app.request.body_editor, layout.content_area)` with `render_body_panel(frame, app, layout.content_area)`

**C.3: Update tab bar label**

- [ ] Update `render_request_tab_bar()` to show body mode in tab label:
  ```rust
  let body_label = match app.request.body_mode {
      BodyMode::Raw => "Body".to_string(),
      BodyMode::Json => "Body (JSON)".to_string(),
      BodyMode::Xml => "Body (XML)".to_string(),
      BodyMode::FormUrlEncoded => "Body (Form)".to_string(),
      BodyMode::Multipart => "Body (Multipart)".to_string(),
      BodyMode::Binary => "Body (Binary)".to_string(),
  };
  ```

**C.4: Render body mode popup**

- [ ] Create `render_body_mode_popup()` — follows method/auth popup pattern:
  - Options: "Raw", "JSON", "XML", "Form URL-Encoded", "Multipart Form", "Binary"
  - j/k navigation with wrap-around
  - Enter selects, Esc cancels
  - Render as overlay centered in the body content area

**C.5: Handle body mode popup keys**

- [ ] In the main key handler, check `show_body_mode_popup` before other popup checks:
  - `j`/`Down`: increment index (mod 6)
  - `k`/`Up`: decrement index (mod 6)
  - `Enter`: set `self.request.body_mode = BodyMode::from_index(index)`, close popup, set `request_dirty = true`
  - `Esc`: close popup without changing mode

**C.6: Handle Body sub-field navigation**

Note: This introduces a behavior change. Currently, switching to the Body tab places focus directly on the text editor. After this phase, focus lands on the ModeSelector row first (user presses `j` to reach the editor). This is consistent with the auth tab pattern where focus lands on AuthType first. The mode selector row is useful — users need quick access to change body type.

- [ ] When `focus.request_field == RequestField::Body` and in Navigation mode:
  - `j`/`Down` from ModeSelector: move to content area (`BodyField::TextEditor` for text modes, `BodyField::KvRow` for KV modes, `BodyField::BinaryPath` for binary)
  - `k`/`Up` from content: move back to ModeSelector
  - `Enter` on ModeSelector: open body mode popup
  - `Enter`/`i` on TextEditor: enter Editing mode on body_editor (existing behavior)

- [ ] Update `is_editable_field()`:
  - `RequestField::Body` with `BodyField::TextEditor` → true (text modes)
  - `RequestField::Body` with `BodyField::BinaryPath` → true (binary mode)
  - `RequestField::Body` with `BodyField::ModeSelector` → false (popup trigger)
  - `RequestField::Body` with `BodyField::KvRow` → handled separately (cell editing)

- [ ] Update `active_editor()`:
  - When `body_field == BodyField::TextEditor`: return `&mut self.request.body_editor` (existing)
  - When `body_field == BodyField::BinaryPath`: return `&mut self.request.body_binary_path_editor`
  - When `body_field == BodyField::KvRow` and `kv_edit_textarea.is_some()`: return the temp TextArea
  - Otherwise: `None`

**C.7: Update `prepare_editors()` for body sub-fields**

- [ ] Prepare body_editor only when body_field == TextEditor and body_mode is a text mode
- [ ] Prepare body_binary_path_editor only when body_field == BinaryPath and body_mode == Binary
- [ ] Prepare kv_edit_textarea when actively editing a KV cell
- [ ] Set cursor styles based on focus (same pattern as auth editors)

**C.8: Update status bar hints**

- [ ] `BodyField::ModeSelector`: "Enter: change body type"
- [ ] `BodyField::TextEditor`: "i/Enter: edit body | Shift+H/L: switch tab"
- [ ] `BodyField::KvRow`: "i/Enter: edit cell | a: add row | d: delete row | Space: toggle"
- [ ] `BodyField::BinaryPath`: "i/Enter: edit file path | Shift+H/L: switch tab"

**C.9: Compile and verify**

- [ ] Compile
- [ ] Manual test: Enter on mode selector → popup appears → select JSON → tab shows "Body (JSON)"
- [ ] Manual test: j/k navigates between mode selector and text editor
- [ ] Manual test: Text content preserved when switching Raw ↔ JSON ↔ XML
- [ ] Manual test: Shift+H/L still switches tabs from within Body tab
- [ ] Manual test: i on text editor → vim editing mode → works as before

**Commit**: `feat(app): add body type selector popup and mode switching`

---

### Phase D: Raw/JSON/XML Text Modes with Content-Type

Wire text-based body modes to auto-set Content-Type at send time.

**D.1: Add JSON validation indicator to body panel**

- [ ] When `body_mode == BodyMode::Json`, add a validation indicator in the mode selector row:
  - Parse body text as JSON (`serde_json::from_str::<Value>`)
  - Valid: green checkmark `✓` after mode name
  - Invalid: red `✗` after mode name
  - Empty: no indicator
  - Run validation on each render (body text is already in memory, parsing is cheap for typical API bodies)

**D.2: Add Content-Type auto-injection to `send_request()`**

- [ ] Create `BodyContent` enum in `src/http.rs`:
  ```rust
  pub enum BodyContent {
      None,
      Raw(String),
      Json(String),
      Xml(String),
      FormUrlEncoded(Vec<(String, String)>),
      Multipart(Vec<MultipartPart>),
      Binary(String),  // file path — read in async task to avoid blocking UI
  }

  pub struct MultipartPart {
      pub key: String,
      pub value: String,
      pub field_type: MultipartPartType,
  }

  pub enum MultipartPartType {
      Text,
      File,  // value is file path
  }
  ```

- [ ] Update `send_request()` signature:
  - Change `body: &str` to `body: BodyContent`
  - Change `headers: &str` to `headers: &str` (keep as-is for now)

- [ ] Implement Content-Type logic:
  ```rust
  // Check if user has manually set Content-Type
  let has_manual_content_type = headers.lines()
      .any(|line| line.trim().to_lowercase().starts_with("content-type"));

  let builder = match body {
      BodyContent::None => builder,
      BodyContent::Raw(text) => {
          if !text.is_empty() && sends_body {
              builder.body(text)
          } else {
              builder
          }
      }
      BodyContent::Json(text) => {
          let mut b = builder;
          if !has_manual_content_type {
              b = b.header("Content-Type", "application/json");
          }
          if !text.is_empty() && sends_body {
              b = b.body(text);
          }
          b
      }
      BodyContent::Xml(text) => {
          let mut b = builder;
          if !has_manual_content_type {
              b = b.header("Content-Type", "application/xml");
          }
          if !text.is_empty() && sends_body {
              b = b.body(text);
          }
          b
      }
      // FormUrlEncoded, Multipart, Binary: handled in later phases
      _ => builder,
  };
  ```

**D.3: Build `BodyContent` from `RequestState`**

- [ ] Add `build_body_content(&self) -> BodyContent` method on `RequestState`:
  ```rust
  pub fn build_body_content(&self) -> BodyContent {
      match self.body_mode {
          BodyMode::Raw => {
              let text = self.body_text();
              if text.trim().is_empty() { BodyContent::None } else { BodyContent::Raw(text) }
          }
          BodyMode::Json => {
              let text = self.body_text();
              if text.trim().is_empty() { BodyContent::None } else { BodyContent::Json(text) }
          }
          BodyMode::Xml => {
              let text = self.body_text();
              if text.trim().is_empty() { BodyContent::None } else { BodyContent::Xml(text) }
          }
          // Other modes: later phases
          _ => BodyContent::None,
      }
  }
  ```

**D.4: Update `App::send_request()` (`src/app.rs:2950-2956`)**

- [ ] Replace:
  ```rust
  let body = self.request.body_text();
  ```
  With:
  ```rust
  let body = self.request.build_body_content();
  ```
- [ ] Update the spawned async task to pass `BodyContent` instead of `String`

**D.5: Update `build_postman_request()` for text mode persistence**

- [ ] Serialize body mode to Postman format:
  ```rust
  let body = match self.request.body_mode {
      BodyMode::Raw => {
          let text = self.request.body_text();
          if text.trim().is_empty() { None } else { Some(PostmanBody::raw(&text)) }
      }
      BodyMode::Json => {
          let text = self.request.body_text();
          if text.trim().is_empty() { None } else { Some(PostmanBody::json(&text)) }
      }
      BodyMode::Xml => {
          let text = self.request.body_text();
          if text.trim().is_empty() { None } else { Some(PostmanBody::xml(&text)) }
      }
      // Other modes: later phases
      _ => {
          let text = self.request.body_text();
          if text.trim().is_empty() { None } else { Some(PostmanBody::raw(&text)) }
      }
  };
  ```

**D.6: Update `open_request()` to load body mode**

- [ ] When loading a `PostmanItem`, detect body mode from Postman body:
  ```rust
  if let Some(body) = &postman_request.body {
      match body.mode.as_str() {
          "raw" => {
              // Check options.raw.language for JSON/XML
              let language = body.options.as_ref()
                  .and_then(|o| o.raw.as_ref())
                  .map(|r| r.language.as_str());
              self.request.body_mode = match language {
                  Some("json") => BodyMode::Json,
                  Some("xml") => BodyMode::Xml,
                  _ => BodyMode::Raw,
              };
              if let Some(raw) = &body.raw {
                  self.request.body_editor = TextArea::new(
                      raw.lines().map(String::from).collect()
                  );
                  configure_editor(&mut self.request.body_editor, "Request body...");
              }
          }
          // Other modes: later phases
          _ => {
              self.request.body_mode = BodyMode::Raw;
          }
      }
  }
  ```

**D.7: Compile and verify**

- [ ] Compile
- [ ] Manual test: **Raw mode** — behavior unchanged from before
- [ ] Manual test: **JSON mode** — select JSON, type `{"key": "value"}`, send → verify Content-Type: application/json in request headers, green checkmark shown
- [ ] Manual test: **JSON validation** — type `{invalid`, verify red X indicator
- [ ] Manual test: **XML mode** — select XML, type `<root/>`, send → verify Content-Type: application/xml
- [ ] Manual test: **Content-Type override** — set JSON mode, manually add `Content-Type: text/plain` in headers → verify text/plain is sent (auto-inject skipped)
- [ ] Manual test: **Mode persistence** — save JSON body request, reopen → JSON mode and content restored

**Commit**: `feat(http): add Content-Type auto-injection for JSON and XML body modes`

---

### Phase E: Key-Value Pair Editor Component

Build the shared key-value table editor used by Form URL-Encoded and Multipart modes.

**E.1: Define KV display trait for shared rendering**

- [ ] Both `Vec<KvPair>` (FormUrlEncoded) and `Vec<MultipartField>` (Multipart) need to be rendered by the same `render_kv_table()`. Define a trait that both implement:
  ```rust
  pub trait KvRow {
      fn key(&self) -> &str;
      fn value(&self) -> &str;
      fn enabled(&self) -> bool;
      fn has_type_column(&self) -> bool { false }
      fn type_label(&self) -> &str { "" }
  }

  impl KvRow for KvPair {
      fn key(&self) -> &str { &self.key }
      fn value(&self) -> &str { &self.value }
      fn enabled(&self) -> bool { self.enabled }
  }

  impl KvRow for MultipartField {
      fn key(&self) -> &str { &self.key }
      fn value(&self) -> &str { &self.value }
      fn enabled(&self) -> bool { self.enabled }
      fn has_type_column(&self) -> bool { true }
      fn type_label(&self) -> &str {
          match self.field_type {
              MultipartFieldType::Text => "Text",
              MultipartFieldType::File => "File",
          }
      }
  }
  ```
  This keeps `render_kv_table()` generic: `fn render_kv_table<T: KvRow>(frame: ..., rows: &[T], ...)`.

**E.2: Implement KV table rendering**

- [ ] Create `render_kv_table()` in `src/ui/mod.rs`:
  ```
  ┌───┬──────────────────┬──────────────────┐
  │ ✓ │ Key              │ Value            │  ← Header row
  ├───┼──────────────────┼──────────────────┤
  │ ✓ │ username         │ admin            │  ← Row 0
  │ ✓ │ password         │ secret           │  ← Row 1
  │ ✗ │ debug            │ true             │  ← Row 2 (disabled)
  │ ✓ │                  │                  │  ← Row 3 (empty, for adding)
  └───┴──────────────────┴──────────────────┘
  ```

- [ ] Layout with `Layout::horizontal()`:
  - Toggle column: `Constraint::Length(3)` — checkbox/enabled indicator
  - Key column: `Constraint::Percentage(50)`
  - Value column: `Constraint::Percentage(50)`

- [ ] Rendering rules:
  - Header row: bold text "Key" / "Value"
  - Active row: highlighted background
  - Active cell (key or value): bright accent border
  - Disabled rows: dim/strikethrough styling
  - Empty trailing row always present (for adding new pairs)
  - When editing a cell: render the `kv_edit_textarea` in place of the cell text

- [ ] Scroll support: if more rows than visible area, scroll to keep active row visible

**E.3: Implement KV table navigation**

- [ ] When `body_field == BodyField::KvRow` in Navigation mode:
  - `j`/`Down`: move to next row (wrap to first after last)
  - `k`/`Up`: move to previous row (or back to ModeSelector from first row)
  - `Tab`/`l`: move to next column (Key → Value, wrap to next row Key)
  - `Shift+Tab`/`h`: move to previous column
  - `Enter`/`i`: enter editing mode on current cell
  - `a`: add new empty row after current, focus it
  - `o`: add new empty row below current, focus it (alias for `a`)
  - `d`: delete current row (if more than 1 row exists)
  - `Space`: toggle enabled/disabled on current row

**E.4: Implement KV cell editing**

- [ ] On `Enter`/`i` with a KV cell focused:
  1. Create a temporary `TextArea` initialized with the cell's current text:
     ```rust
     let text = match self.focus.kv_focus.column {
         KvColumn::Key => pair.key.clone(),
         KvColumn::Value => pair.value.clone(),
     };
     let mut textarea = TextArea::new(vec![text]);
     configure_editor(&mut textarea, "");
     self.kv_edit_textarea = Some(textarea);
     self.app_mode = AppMode::Editing;
     ```
  2. Enter `AppMode::Editing` — vim mode applies to this TextArea
  3. On `Esc` (back to Navigation from vim Normal mode):
     - Extract text from TextArea: `let text = textarea.lines().join("");`
     - Write back to the appropriate KvPair field
     - Clear `kv_edit_textarea = None`

- [ ] `active_editor()` returns `&mut kv_edit_textarea.as_mut().unwrap()` when editing a KV cell

**E.5: Ensure auto-append empty row**

- [ ] After any edit to the last row that makes it non-empty (key or value has text), automatically append a new empty `KvPair` at the end
- [ ] After deleting a row, if no rows remain, add one empty row

**E.6: Compile and verify**

- [ ] Compile (KV editor exists as a component but is not yet wired to a body mode)
- [ ] Manual test: render KV table with test data in FormUrlEncoded mode placeholder
- [ ] Manual test: navigate rows and columns, verify focus highlighting

**Commit**: `feat(ui): add key-value pair table editor component`

---

### Phase F: Form URL-Encoded Mode

Wire the KV editor to Form URL-Encoded body mode with encoding at send time.

**F.1: Wire KV editor to FormUrlEncoded mode**

- [ ] In `render_body_panel()`, add `BodyMode::FormUrlEncoded` branch:
  ```rust
  BodyMode::FormUrlEncoded => {
      render_kv_table(frame, app, &app.request.body_form_pairs,
          app.focus.kv_focus, app.focus.body_field == BodyField::KvRow,
          &app.kv_edit_textarea, layout.content_area);
  }
  ```

- [ ] When `body_mode == FormUrlEncoded` and `body_field == KvRow`:
  - Navigation reads from `body_form_pairs`
  - Cell edits write back to `body_form_pairs`

**F.2: Implement form encoding at send time**

- [ ] Add `BodyContent::FormUrlEncoded` handling in `send_request()` using reqwest's built-in form encoding:
  ```rust
  BodyContent::FormUrlEncoded(pairs) => {
      if !pairs.is_empty() && sends_body {
          builder.form(&pairs)  // reqwest handles Content-Type and percent-encoding
      } else {
          builder
      }
  }
  ```
  Note: `builder.form()` auto-sets `Content-Type: application/x-www-form-urlencoded`. No need for the `has_manual_content_type` check or a separate encoding crate — reqwest handles both correctly. Edge case: if the user manually sets Content-Type in headers, both headers are sent. This matches Postman's behavior and is acceptable for MVP.

**F.3: Update `build_body_content()` for FormUrlEncoded**

- [ ] Add case:
  ```rust
  BodyMode::FormUrlEncoded => {
      let pairs: Vec<(String, String)> = self.body_form_pairs.iter()
          .filter(|p| p.enabled && !(p.key.is_empty() && p.value.is_empty()))
          .map(|p| (p.key.clone(), p.value.clone()))
          .collect();
      if pairs.is_empty() { BodyContent::None } else { BodyContent::FormUrlEncoded(pairs) }
  }
  ```

**F.4: Persist form pairs to Postman collection**

- [ ] In `build_postman_request()`:
  ```rust
  BodyMode::FormUrlEncoded => {
      let pairs: Vec<PostmanKvPair> = self.request.body_form_pairs.iter()
          .filter(|p| !(p.key.is_empty() && p.value.is_empty()))
          .map(|p| PostmanKvPair {
              key: p.key.clone(),
              value: p.value.clone(),
              disabled: if p.enabled { None } else { Some(true) },
          })
          .collect();
      if pairs.is_empty() { None } else { Some(PostmanBody::urlencoded(pairs)) }
  }
  ```

- [ ] In `open_request()`, add `"urlencoded"` mode handling:
  ```rust
  "urlencoded" => {
      self.request.body_mode = BodyMode::FormUrlEncoded;
      if let Some(pairs) = &body.urlencoded {
          self.request.body_form_pairs = pairs.iter().map(|p| KvPair {
              key: p.key.clone(),
              value: p.value.clone(),
              enabled: !p.disabled.unwrap_or(false),
          }).collect();
      }
      // Ensure trailing empty row
      if self.request.body_form_pairs.is_empty()
          || !self.request.body_form_pairs.last().unwrap().key.is_empty() {
          self.request.body_form_pairs.push(KvPair::new_empty());
      }
  }
  ```

**F.5: Compile and verify**

- [ ] Compile
- [ ] Manual test: select Form URL-Encoded → KV table appears
- [ ] Manual test: add pairs username=admin, password=secret → send to httpbin.org/post → verify form data in response
- [ ] Manual test: toggle row disabled → row dimmed, not sent
- [ ] Manual test: save request, reopen → form pairs restored
- [ ] Manual test: Content-Type auto-set to application/x-www-form-urlencoded

**Commit**: `feat(http): add Form URL-Encoded body mode with key-value editor`

---

### Phase G: Multipart Form Data Mode

Extend the KV editor with per-row type (text/file) for multipart submissions.

**G.1: Extend KV table renderer for multipart**

- [ ] Add an optional "Type" column to `render_kv_table()` (only shown for multipart):
  ```
  ┌───┬──────────┬──────────┬──────────────────┐
  │ ✓ │ Key      │ Type     │ Value            │
  ├───┼──────────┼──────────┼──────────────────┤
  │ ✓ │ name     │ Text     │ John             │
  │ ✓ │ avatar   │ File     │ /path/to/img.png │
  │ ✓ │          │ Text     │                  │
  └───┴──────────┴──────────┴──────────────────┘
  ```

- [ ] Type column: `Constraint::Length(6)` — shows "Text" or "File"
- [ ] Toggle type with `t` key on the Type column (or add a third `KvColumn::Type`)

- [ ] Extend `KvColumn`:
  ```rust
  pub enum KvColumn {
      Key,
      Type,   // Only relevant for multipart
      Value,
  }
  ```

**G.2: Wire multipart to body panel**

- [ ] In `render_body_panel()`, add `BodyMode::Multipart` branch — same KV table but with `body_multipart_fields` and type column enabled

- [ ] Navigation when `body_mode == Multipart`:
  - `Tab`/`l`: Key → Type → Value → next row Key
  - `Enter` on Type column: toggle Text ↔ File (no popup needed, only 2 options)
  - When type is File, the Value column placeholder shows "File path..."

**G.3: Implement multipart form building at send time**

- [ ] Add to `send_request()`:
  ```rust
  BodyContent::Multipart(parts) => {
      if !parts.is_empty() && sends_body {
          let mut form = reqwest::multipart::Form::new();
          for part in parts {
              match part.field_type {
                  MultipartPartType::Text => {
                      form = form.text(part.key.clone(), part.value.clone());
                  }
                  MultipartPartType::File => {
                      let path = std::path::Path::new(&part.value);
                      let file_bytes = std::fs::read(path)
                          .map_err(|e| format!("Failed to read file '{}': {}", part.value, e))?;
                      let file_name = path.file_name()
                          .and_then(|n| n.to_str())
                          .unwrap_or("file")
                          .to_string();
                      let file_part = reqwest::multipart::Part::bytes(file_bytes)
                          .file_name(file_name);
                      form = form.part(part.key.clone(), file_part);
                  }
              }
          }
          builder.multipart(form)  // reqwest sets Content-Type with boundary automatically
      } else {
          builder
      }
  }
  ```

  Note: `reqwest::multipart::Form` requires the `multipart` feature on reqwest. Verify it's enabled in `Cargo.toml`. Also verify `Form` is `Send` (required for `tokio::spawn`) — it is, per reqwest docs.

- [ ] Add `multipart` feature to reqwest in `Cargo.toml` if not already present:
  ```toml
  reqwest = { version = "...", features = ["json", "multipart"] }
  ```

**G.4: Update `build_body_content()` for Multipart**

- [ ] Add case:
  ```rust
  BodyMode::Multipart => {
      let parts: Vec<MultipartPart> = self.body_multipart_fields.iter()
          .filter(|f| f.enabled && !f.key.is_empty())
          .map(|f| MultipartPart {
              key: f.key.clone(),
              value: f.value.clone(),
              field_type: match f.field_type {
                  MultipartFieldType::Text => MultipartPartType::Text,
                  MultipartFieldType::File => MultipartPartType::File,
              },
          })
          .collect();
      if parts.is_empty() { BodyContent::None } else { BodyContent::Multipart(parts) }
  }
  ```

**G.5: Persist multipart fields to Postman collection**

- [ ] In `build_postman_request()`:
  ```rust
  BodyMode::Multipart => {
      let params: Vec<PostmanFormParam> = self.request.body_multipart_fields.iter()
          .filter(|f| !f.key.is_empty())
          .map(|f| PostmanFormParam {
              key: f.key.clone(),
              value: if f.field_type == MultipartFieldType::Text { Some(f.value.clone()) } else { None },
              src: if f.field_type == MultipartFieldType::File { Some(f.value.clone()) } else { None },
              param_type: match f.field_type {
                  MultipartFieldType::Text => "text".to_string(),
                  MultipartFieldType::File => "file".to_string(),
              },
              disabled: if f.enabled { None } else { Some(true) },
          })
          .collect();
      if params.is_empty() { None } else { Some(PostmanBody::formdata(params)) }
  }
  ```

- [ ] In `open_request()`, add `"formdata"` mode handling:
  ```rust
  "formdata" => {
      self.request.body_mode = BodyMode::Multipart;
      if let Some(params) = &body.formdata {
          self.request.body_multipart_fields = params.iter().map(|p| MultipartField {
              key: p.key.clone(),
              value: match p.param_type.as_str() {
                  "file" => p.src.clone().unwrap_or_default(),
                  _ => p.value.clone().unwrap_or_default(),
              },
              field_type: match p.param_type.as_str() {
                  "file" => MultipartFieldType::File,
                  _ => MultipartFieldType::Text,
              },
              enabled: !p.disabled.unwrap_or(false),
          }).collect();
      }
      // Ensure trailing empty row
      if self.request.body_multipart_fields.is_empty()
          || !self.request.body_multipart_fields.last().unwrap().key.is_empty() {
          self.request.body_multipart_fields.push(MultipartField::new_empty());
      }
  }
  ```

**G.6: Compile and verify**

- [ ] Compile
- [ ] Manual test: select Multipart Form → KV table with Type column appears
- [ ] Manual test: add text field name=John, toggle type to File for avatar field, enter path → send to httpbin.org/post → verify multipart response
- [ ] Manual test: file not found → error message in response area
- [ ] Manual test: save request with multipart fields, reopen → fields restored with correct types

**Commit**: `feat(http): add Multipart Form Data body mode with file support`

---

### Phase H: Binary File Mode + Final Save/Load Integration

Complete binary mode and ensure all body modes round-trip through storage.

**H.1: Wire binary mode to body panel**

- [ ] In `render_body_panel()`, add `BodyMode::Binary` branch:
  ```rust
  BodyMode::Binary => {
      let layout_binary = Layout::vertical([
          Constraint::Length(1),  // Label "File:"
          Constraint::Length(3),  // Path editor
          Constraint::Min(0),    // File info or empty
      ]).split(layout.content_area);

      let label = Paragraph::new("File:")
          .style(Style::default().fg(Color::DarkGray));
      frame.render_widget(label, layout_binary[0]);
      frame.render_widget(&app.request.body_binary_path_editor, layout_binary[1]);

      // Show file info (exists? size?) below path editor
      let path_text = app.request.body_binary_path_text();
      let info = if path_text.trim().is_empty() {
          "No file selected".to_string()
      } else {
          match std::fs::metadata(&path_text) {
              Ok(meta) => format!("{} bytes", meta.len()),
              Err(_) => "File not found".to_string(),
          }
      };
      let info_widget = Paragraph::new(info)
          .style(Style::default().fg(Color::DarkGray));
      frame.render_widget(info_widget, layout_binary[2]);
  }
  ```

**H.2: Implement binary body at send time**

- [ ] Add to `send_request()` — file is read in the async task to avoid blocking the UI:
  ```rust
  BodyContent::Binary(path) => {
      if !path.is_empty() && sends_body {
          let bytes = std::fs::read(&path)
              .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;
          let mut b = builder;
          if !has_manual_content_type {
              b = b.header("Content-Type", "application/octet-stream");
          }
          b.body(bytes)
      } else {
          builder
      }
  }
  ```
  Note: The file read happens inside `send_request()` (which runs in a `tokio::spawn` task), not in `build_body_content()`. This prevents large files from blocking the UI thread.

**H.3: Update `build_body_content()` for Binary**

- [ ] `build_body_content()` passes the file path, not the file contents:
  ```rust
  BodyMode::Binary => {
      let path = self.body_binary_path_text();
      if path.trim().is_empty() {
          BodyContent::None
      } else {
          BodyContent::Binary(path)
      }
  }
  ```

**H.4: Persist binary path to Postman collection**

- [ ] In `build_postman_request()`:
  ```rust
  BodyMode::Binary => {
      let path = self.request.body_binary_path_text();
      if path.trim().is_empty() { None } else { Some(PostmanBody::file(&path)) }
  }
  ```

- [ ] In `open_request()`, add `"file"` mode handling:
  ```rust
  "file" => {
      self.request.body_mode = BodyMode::Binary;
      if let Some(file_ref) = &body.file {
          if let Some(src) = &file_ref.src {
              self.request.body_binary_path_editor = TextArea::new(vec![src.clone()]);
              configure_editor(&mut self.request.body_binary_path_editor, "File path...");
          }
      }
  }
  ```

**H.5: Update `set_contents()` on RequestState**

- [ ] When `set_contents()` is called (for resetting or new request), also reset:
  - `body_mode` to `BodyMode::Raw`
  - `body_form_pairs` to `vec![KvPair::new_empty()]`
  - `body_multipart_fields` to `vec![MultipartField::new_empty()]`
  - `body_binary_path_editor` to `TextArea::default()` with placeholder

**H.6: Update session state for body mode**

- [ ] Body mode doesn't need session persistence (it's stored per-request in the collection)
- [ ] Verify: switching requests correctly loads the saved body mode

**H.7: Mark request dirty on body mode changes**

- [ ] Set `self.request_dirty = true` when:
  - Body mode changes (popup selection)
  - Any KV pair content changes (add/edit/delete/toggle)
  - Binary path changes
  - Multipart field type toggles

**H.8: Compile and verify end-to-end**

- [ ] Compile
- [ ] Manual test: **Raw mode** — send raw text → no auto Content-Type → works as before
- [ ] Manual test: **JSON mode** — send `{"key":"value"}` → Content-Type: application/json auto-set → green checkmark shown
- [ ] Manual test: **XML mode** — send `<root/>` → Content-Type: application/xml auto-set
- [ ] Manual test: **Form URL-Encoded** — add pairs → send to httpbin.org/post → verify form in response
- [ ] Manual test: **Multipart** — add text + file fields → send → verify multipart in response
- [ ] Manual test: **Binary** — enter valid file path → send → file contents sent as body
- [ ] Manual test: **Binary error** — enter invalid file path → send → error message shown
- [ ] Manual test: **Mode switching preservation** — type JSON text, switch to Form, switch back → text preserved
- [ ] Manual test: **Save/load roundtrip** — save each body mode, reopen → all data restored correctly
- [ ] Manual test: **Backward compatibility** — open old collection (no body mode data) → defaults to Raw, no crash

**Commit**: `feat(http): add Binary file body mode and complete body type save/load`

---

## Alternative Approaches Considered

| Approach | Why Rejected |
|----------|-------------|
| Separate TextArea per text mode (Raw, JSON, XML) | Wastes memory, content lost on mode switch. Shared TextArea preserves content across text modes. |
| Persistent TextAreas for every KV cell | N*2 TextAreas for form data is expensive and complex. Temporary TextArea for active cell is simpler. |
| JSON pretty-format on paste | Too magical, breaks user intent. Better as explicit keyboard shortcut (deferred). |
| Block sending on invalid JSON | Developer tools should not prevent requests. User may intentionally test error handling. |
| Custom Content-Type management panel | Over-engineered. Auto-inject with manual override via headers is sufficient. |
| Body type as a separate tab (not within Body tab) | Adds a 4th tab to the request panel. Mode selector within Body tab is more compact. |

## Acceptance Criteria

### Functional Requirements

- [ ] Six body modes available: Raw, JSON, XML, Form URL-Encoded, Multipart Form, Binary
- [ ] Body mode selector popup with j/k + Enter navigation
- [ ] JSON mode auto-sets Content-Type: application/json
- [ ] XML mode auto-sets Content-Type: application/xml
- [ ] Form URL-encoded sends properly encoded key=value&key2=value2 body
- [ ] Multipart form sends proper multipart/form-data with text and file parts
- [ ] Binary mode reads file from path and sends as body with application/octet-stream
- [ ] Content-Type auto-injection respects user's manually-set Content-Type header

### Data Integrity

- [ ] All body modes persist per-request in Postman Collection v2.1 format
- [ ] Save → reload roundtrip preserves body mode, text content, form pairs (with enabled state), multipart fields (with types), and binary path
- [ ] Collections without extended body fields load correctly (default Raw mode) — backward compatible
- [ ] Disabled form pairs are stored but not sent

### UI/UX

- [ ] Body tab label shows mode: "Body (JSON)" / "Body (Form)" / etc. ("Body" for Raw)
- [ ] JSON validation indicator (green checkmark / red X) in mode selector row
- [ ] Key-value table editor with j/k row navigation, Tab column navigation, Enter cell editing
- [ ] KV row operations: add (a), delete (d), toggle enabled (Space)
- [ ] Multipart type column: toggle Text/File per row
- [ ] Binary mode shows file info (size or "not found") below path editor
- [ ] Full vim editing on all text fields (body editor, KV cells, binary path)
- [ ] Status bar hints update for each body sub-field
- [ ] Shift+H/L tab cycling works from all body sub-fields

### Edge Cases

- [ ] Empty body with non-Raw mode: Content-Type still set (for modes that auto-set it)
- [ ] Switching between text modes preserves content
- [ ] Switching text ↔ KV modes: both states preserved independently in memory
- [ ] Deleting all KV rows leaves one empty row (can't have zero rows)
- [ ] Binary file read error shows user-friendly error (not a crash)
- [ ] Large file binary: no preview, just file info (size)
- [ ] KV editing with very long values: TextArea handles scrolling

### Quality Gates

- [ ] Compiles with no warnings
- [ ] All existing tests pass
- [ ] Each phase independently committed and functional

---

## Dependencies & Prerequisites

| Dependency | Status | Notes |
|-----------|--------|-------|
| reqwest `multipart` feature | Check `Cargo.toml` | Required for Phase G. May need feature flag addition. |
| Auth feature (Phase 1.2) | Completed | Auth popup pattern provides blueprint. No runtime dependency. |
| Config file (Phase 1.1) | Completed | No direct dependency. |

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| KV table editor complexity (new UI pattern) | High | Medium | Build as isolated component first (Phase E). Test thoroughly before wiring to body modes. |
| Temporary TextArea lifecycle bugs (create/destroy on edit) | Medium | Medium | Clear `kv_edit_textarea` on mode switch, tab switch, and request switch. Defensive None checks. |
| reqwest multipart feature breaks compile | Low | Low | Check feature compatibility early. Multipart is a well-supported reqwest feature. |
| Large file binary reads block UI | Low | High | Mitigated: file reads happen in the async `send_request()` task, not on the main thread. Already uses same `tokio::spawn` pattern as HTTP sending. |
| Body mode selector popup conflicts with method/auth popups | Low | Low | Only one popup at a time. Check body_mode_popup before method_popup in key handler priority chain. |
| Postman body format edge cases on import | Low | Low | Import doesn't exist yet. Handle gracefully — unknown modes default to Raw. |
| KV table scroll/overflow in narrow terminals | Medium | Low | Standard ratatui scroll handling. Truncate cell text with ellipsis when needed. |

## Future Considerations

- **JSON pretty-format**: Add `Ctrl+Shift+F` to format/beautify JSON in the body editor
- **JSON schema validation**: Validate against a schema URL (advanced)
- **Body preview**: Show encoded form body preview for URL-encoded mode
- **File browser**: TUI file picker for binary/multipart file selection (instead of manual path entry)
- **Drag-and-drop import**: Paste file path from system clipboard
- **Content-Type detection**: Auto-detect body type from Content-Type header when importing

## References

### Internal References

- Brainstorm: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` — Phase 1.5
- Auth plan (blueprint): `docs/plans/2026-02-15-feat-authentication-support-plan.md` — popup pattern, TextArea editing, save/load
- Current body handling: `src/http.rs:75-77` — raw `builder.body(body.to_string())`
- Current body storage: `src/storage/postman.rs:74-79` — `PostmanBody { mode, raw }`
- Request state: `src/app.rs:414-418` — `body_editor: TextArea<'static>`
- Body rendering: `src/ui/mod.rs:468-470` — `frame.render_widget(&app.request.body_editor, ...)`
- Method popup pattern: `src/app.rs:2398-2463` — reusable for body mode popup
- Auth popup pattern: `src/app.rs:3117-3157` — reusable for body mode popup

### External References

- Postman Collection v2.1 Body Schema: body.mode supports "raw", "urlencoded", "formdata", "file", "graphql"
- reqwest multipart API: `reqwest::multipart::Form`, `reqwest::multipart::Part`
- reqwest form API: `RequestBuilder::form()` for URL-encoded form data
