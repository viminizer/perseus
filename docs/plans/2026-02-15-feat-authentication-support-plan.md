---
title: "feat: Add authentication support (Bearer, Basic, API Key)"
type: feat
date: 2026-02-15
---

# feat: Add Authentication Support (Bearer, Basic, API Key)

## Overview

Add per-request authentication support to Perseus with three auth types: Bearer Token, Basic Auth, and API Key. This includes a new Auth tab in the request panel, an auth type selector popup, per-type input fields with full vim editing, automatic header/query param injection at send time, and Postman Collection v2.1 compatible storage.

## Problem Statement

Perseus currently has no authentication support. Users must manually type `Authorization: Bearer <token>` or `Authorization: Basic <base64>` into the Headers tab. This is:

| Gap | Impact |
|-----|--------|
| No dedicated auth UI | Users must remember header formats and manually encode Base64 for Basic Auth |
| No auth persistence model | Auth semantics are lost — a Bearer token in the Headers tab is indistinguishable from any other header |
| No API Key support | Users must manually add headers or query params for API Key auth |
| No Postman auth interop | Importing Postman collections with auth configured would lose the auth data (when import is later implemented) |

## Proposed Solution

A six-phase implementation, each phase independently compilable and committable:

1. **Phase A**: Postman-compatible auth data model (storage structs)
2. **Phase B**: In-memory auth state on `RequestState` with TextArea editors
3. **Phase C**: Tab system + navigation extensions (RequestTab::Auth, RequestField::Auth)
4. **Phase D**: Auth tab rendering (type selector, per-type field layout)
5. **Phase E**: Auth type popup + field editing (interaction model)
6. **Phase F**: Auth injection into HTTP requests + save/load integration

## Technical Approach

### Current Architecture

```
User Input (keyboard)
    │
    ▼
AppMode::Navigation ──Shift+H/L──▶ RequestTab { Headers, Body }
    │                                     │
    ▼                                     ▼
RequestField { Method, Url, Send,    render_request_panel()
               Headers, Body }            │
    │                                     ▼
    ▼                              frame.render_widget(&textarea, area)
Enter/i → AppMode::Editing
    │
    ▼
TextArea<'static> ── vim mode ──▶ active_editor()
    │
    ▼
send_request() ──url/headers/body strings──▶ http::send_request()
    │                                              │
    ▼                                              ▼
build_postman_request() ──▶ PostmanRequest { method, header, body, url }
```

### Target Architecture

```
User Input (keyboard)
    │
    ▼
AppMode::Navigation ──Shift+H/L──▶ RequestTab { Headers, Auth, Body }
    │                                     │
    ▼                                     ▼
RequestField { Method, Url, Send,    render_request_panel()
               Headers, Auth, Body }      │
    │                                     ├── Auth tab: render_auth_panel()
    ▼                                     │     ├── Auth type selector row
AuthField { AuthType, Token,              │     └── Dynamic fields per type
  Username, Password,                     │
  KeyName, KeyValue, KeyLocation }        ▼
    │                              frame.render_widget(&auth_textarea, area)
    ▼
Enter/i → AppMode::Editing (on auth TextAreas)
         or popup (on AuthType/KeyLocation selectors)
    │
    ▼
send_request() ──auth_config──▶ http::send_request(... auth)
    │                                    │
    ▼                                    ├── reqwest .bearer_auth(token)
build_postman_request()                  ├── reqwest .basic_auth(user, pass)
    │                                    └── inject header or modify URL
    ▼
PostmanRequest { method, header, body, url, auth: Option<PostmanAuth> }
```

### Key Files and Touchpoints

| File | Lines | What Changes |
|------|-------|-------------|
| `src/storage/postman.rs` | 32-41 | Add `PostmanAuth`, `PostmanAuthAttribute` structs; add `auth` field to `PostmanRequest` |
| `src/app.rs` | 66-71 | Add `Auth` variant to `RequestTab` enum |
| `src/app.rs` | 73-85 | Update `request_tab_from_str` / `request_tab_to_str` |
| `src/app.rs` | 244-252 | Add `Auth` variant to `RequestField` enum |
| `src/app.rs` | 364-369 | Add auth state + TextArea editors to `RequestState` |
| `src/app.rs` | 397-417 | Update `set_contents()` for auth |
| `src/app.rs` | 431-438 | Update `active_editor()` to return auth TextAreas |
| `src/app.rs` | 1124-1135 | Update `build_postman_request()` to serialize auth |
| `src/app.rs` | 1137-1157 | Update `open_request()` to load auth from storage |
| `src/app.rs` | 1929-1987 | Update `prepare_editors()` for auth field cursor/block styles |
| `src/app.rs` | 2649-2672 | Update `App::send_request()` to pass auth config |
| `src/app.rs` | 2681-2686 | Update `is_editable_field()` for auth fields |
| `src/app.rs` | 2729-2770 | Update `next_vertical()` / `prev_vertical()` for Auth tab |
| `src/app.rs` | 2773-2792 | Update `next_request_tab()` / `prev_request_tab()` for 3-tab cycle |
| `src/http.rs` | 7-70 | Extend `send_request()` signature with auth param; inject auth into reqwest builder |
| `src/ui/mod.rs` | 351-385 | Add `RequestTab::Auth` rendering branch |
| `src/ui/mod.rs` | 388-425 | Update `render_request_tab_bar()` with Auth tab |
| `src/ui/mod.rs` | 1069-1109 | Update status bar for Auth field hints |

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Auth field widgets | `TextArea<'static>` for token, username, password, key name, key value | Consistent with existing editing model. Full vim mode on all fields. Users expect the same editing experience everywhere. |
| Auth type selector | Popup (like method selector) | Proven interaction pattern. j/k + Enter. Familiar to users. |
| API Key location selector | Toggle with Enter (cycles Header ↔ Query Param) | Only 2 options — a popup is overkill. |
| Tab order | Headers → Auth → Body | Auth logically sits between "what headers to send" and "what body to send". |
| Tab cycling | Circular (Body → Headers wraps around) | Matches existing 2-tab toggle behavior extended to 3. |
| Auth vs manual header conflict | No conflict detection; both sent | Matches Postman behavior. User is responsible. Keeps MVP simple. |
| Auth data on type switch | Clear previous type's data | Matches Postman behavior. Simpler state model. Avoids lossy Postman roundtrip (v2.1 only stores active type). |
| Password masking | No masking | Developer tool in a terminal — same context as `curl -u user:pass`. Masking conflicts with vim visual mode and cursor positioning. |
| RequestField for auth | Single `RequestField::Auth` + separate `AuthField` sub-enum on `FocusState` | Minimizes changes to focus navigation. `AuthField` is cursor focus state (not request data), so it lives on `FocusState` alongside `panel` and `request_field`. |
| Auth injection layer | Extend `http::send_request()` signature with an `AuthConfig` enum param | Clean separation. Uses reqwest's built-in `.bearer_auth()` / `.basic_auth()` for correctness. Keeps auth data out of the visible headers text. |
| Unsupported Postman auth types | Ignore gracefully (default to NoAuth) | Postman import doesn't exist yet. When it ships, extend the auth structs to handle additional types. |
| Auth type in tab label | Show "Auth (Bearer)" / "Auth (Basic)" / etc. | At-a-glance visibility of configured auth without switching tabs. |
| Base64 encoding | Use reqwest's `.basic_auth()` | No need for a separate `base64` crate. reqwest handles encoding correctly per RFC 7617. |

---

## Implementation Phases

### Phase A: Postman-Compatible Auth Data Model

Add the serialization structs for auth in the Postman Collection v2.1 format.

**A.1: Add auth structs to `src/storage/postman.rs`**

- [x] Add `PostmanAuthAttribute` struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanAuthAttribute {
      pub key: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub value: Option<serde_json::Value>,
      #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
      pub attr_type: Option<String>,
  }
  ```

- [x] Add `PostmanAuth` struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct PostmanAuth {
      #[serde(rename = "type")]
      pub auth_type: String,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub bearer: Option<Vec<PostmanAuthAttribute>>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub basic: Option<Vec<PostmanAuthAttribute>>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub apikey: Option<Vec<PostmanAuthAttribute>>,
  }
  ```

- [x] Add `auth` field to `PostmanRequest` (`src/storage/postman.rs:32-41`):
  ```rust
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub auth: Option<PostmanAuth>,
  ```

- [x] Update `PostmanRequest::new()` to set `auth: None`

**A.2: Add helper methods on `PostmanAuth`**

- [x] `PostmanAuth::bearer(token: &str) -> PostmanAuth` — constructs a bearer auth object
- [x] `PostmanAuth::basic(username: &str, password: &str) -> PostmanAuth` — constructs a basic auth object
- [x] `PostmanAuth::apikey(key: &str, value: &str, location: &str) -> PostmanAuth` — constructs an apikey auth object
- [x] `PostmanAuth::get_bearer_token(&self) -> Option<&str>` — extracts token from bearer array
- [x] `PostmanAuth::get_basic_credentials(&self) -> Option<(&str, &str)>` — extracts username/password
- [x] `PostmanAuth::get_apikey(&self) -> Option<(&str, &str, &str)>` — extracts key, value, location

**A.3: Verify backward compatibility**

- [x] Compile — no other code changes needed (auth is `Option` with `serde(default)`)
- [x] Verify: existing collection JSON without `auth` field deserializes correctly (auth = None)
- [x] Verify: a JSON with an `auth` object round-trips correctly through serialize/deserialize

**Commit**: `feat(storage): add Postman v2.1 auth data model`

---

### Phase B: In-Memory Auth State on RequestState

Add the runtime auth state model with TextArea editors.

**B.1: Define auth enums in `src/app.rs`**

- [x] Add `AuthType` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum AuthType {
      #[default]
      NoAuth,
      Bearer,
      Basic,
      ApiKey,
  }
  ```

- [x] Add `ApiKeyLocation` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum ApiKeyLocation {
      #[default]
      Header,
      QueryParam,
  }
  ```

- [x] Add `AuthField` enum (tracks focused sub-field within Auth tab):
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
  pub enum AuthField {
      #[default]
      AuthType,       // The type selector row
      // Bearer fields
      Token,
      // Basic fields
      Username,
      Password,
      // API Key fields
      KeyName,
      KeyValue,
      KeyLocation,    // Header/QueryParam toggle
  }
  ```

- [x] Add constants on `AuthType`:
  - `AuthType::ALL: [AuthType; 4]` — for popup rendering
  - `AuthType::as_str(&self) -> &str` — "No Auth", "Bearer Token", "Basic Auth", "API Key"
  - `AuthType::from_index(usize) -> AuthType`
  - `AuthType::index(&self) -> usize`

**B.2: Add auth state to `RequestState` in `src/app.rs`**

- [x] Add fields to `RequestState` (after existing editors):
  ```rust
  pub auth_type: AuthType,
  pub api_key_location: ApiKeyLocation,
  // TextArea editors for auth fields (single-line: one row, no line wrapping)
  pub auth_token_editor: TextArea<'static>,
  pub auth_username_editor: TextArea<'static>,
  pub auth_password_editor: TextArea<'static>,
  pub auth_key_name_editor: TextArea<'static>,
  pub auth_key_value_editor: TextArea<'static>,
  ```

- [x] Add `auth_field: AuthField` to `FocusState` (not `RequestState` — it's cursor focus, not request data):
  ```rust
  pub struct FocusState {
      pub panel: Panel,
      pub request_field: RequestField,
      pub auth_field: AuthField,  // tracks focused sub-field within Auth tab
  }
  ```

- [x] Update `RequestState::new()` to initialize auth fields:
  - `auth_type: AuthType::NoAuth`
  - `api_key_location: ApiKeyLocation::Header`
  - All auth TextAreas: `TextArea::default()` — configure as single-line (disable line breaks in Insert mode)

- [x] Update `FocusState` default to include `auth_field: AuthField::AuthType`

- [x] Add text extraction methods:
  - `auth_token_text(&self) -> String`
  - `auth_username_text(&self) -> String`
  - `auth_password_text(&self) -> String`
  - `auth_key_name_text(&self) -> String`
  - `auth_key_value_text(&self) -> String`

**B.3: Compile and verify**

- [x] Compile — auth state exists but is not yet wired into UI or HTTP
- [x] All existing functionality works unchanged

**Commit**: `feat(app): add in-memory auth state model with TextArea editors`

---

### Phase C: Tab System + Navigation Extensions

Wire the Auth tab into the navigation model.

**C.1: Extend `RequestTab` enum (`src/app.rs:66-71`)**

- [x] Add `Auth` variant:
  ```rust
  pub enum RequestTab {
      #[default]
      Headers,
      Auth,
      Body,
  }
  ```

- [x] Update `request_tab_from_str()`:
  ```rust
  "Auth" => RequestTab::Auth,
  ```

- [x] Update `request_tab_to_str()`:
  ```rust
  RequestTab::Auth => "Auth",
  ```

**C.2: Extend `RequestField` enum (`src/app.rs:244-252`)**

- [x] Add `Auth` variant:
  ```rust
  pub enum RequestField {
      Method,
      Url,
      Send,
      Headers,
      Auth,
      Body,
  }
  ```

**C.3: Update tab cycling (`src/app.rs:2773-2792`)**

- [x] Rewrite `next_request_tab()` for 3-tab circular cycle:
  ```rust
  fn next_request_tab(&mut self) {
      self.request_tab = match self.request_tab {
          RequestTab::Headers => RequestTab::Auth,
          RequestTab::Auth => RequestTab::Body,
          RequestTab::Body => RequestTab::Headers,
      };
      self.focus.request_field = match self.request_tab {
          RequestTab::Headers => RequestField::Headers,
          RequestTab::Auth => RequestField::Auth,
          RequestTab::Body => RequestField::Body,
      };
  }
  ```

- [x] Rewrite `prev_request_tab()` for reverse cycle:
  ```rust
  fn prev_request_tab(&mut self) {
      self.request_tab = match self.request_tab {
          RequestTab::Headers => RequestTab::Body,
          RequestTab::Auth => RequestTab::Headers,
          RequestTab::Body => RequestTab::Auth,
      };
      // same focus update as next_request_tab
  }
  ```

**C.4: Update vertical navigation (`src/app.rs:2729-2770`)**

- [x] Add `Auth` arm to `next_vertical()` — navigating down from Url/Method/Send row:
  ```rust
  RequestTab::Auth => RequestField::Auth,
  ```

- [x] Add `Auth` arm to `prev_vertical()` — navigating up from Auth field:
  ```rust
  RequestField::Auth => RequestField::Url,
  ```

**C.5: Update session state**

- [x] `request_tab_from_str` already handles `"Auth"` (from C.1)
- [x] Verify: save session with Auth tab active, reload → Auth tab restored

**C.6: Compile and verify navigation**

- [x] Compile
- [x] Manual test: Shift+H/L cycles Headers → Auth → Body → Headers
- [x] Manual test: j/k navigates from URL row to Auth tab content and back
- [x] Auth tab content area is empty for now (will be filled in Phase D)

**Commit**: `feat(app): add Auth tab to request panel navigation`

---

### Phase D: Auth Tab Rendering

Render the auth tab content with type selector and per-type fields.

**D.1: Update tab bar (`src/ui/mod.rs:388-425`)**

- [x] Add Auth span to `render_request_tab_bar()`:
  ```rust
  // Dynamic label: "Auth" for NoAuth, "Auth (Bearer)" for Bearer, etc.
  let auth_label = match app.request.auth_type {
      AuthType::NoAuth => "Auth".to_string(),
      AuthType::Bearer => "Auth (Bearer)".to_string(),
      AuthType::Basic => "Auth (Basic)".to_string(),
      AuthType::ApiKey => "Auth (API Key)".to_string(),
  };
  ```
- [x] Tab bar order: `Headers | Auth | Body` (matching the enum order)
- [x] Active/inactive styling follows existing pattern

**D.2: Add `render_auth_panel()` in `src/ui/mod.rs`**

- [x] Create a new render function for auth tab content
- [x] Layout structure (vertical stack within the tab content area):
  ```
  ┌─────────────────────────────────┐
  │ Type: [Bearer Token       ▾]   │  ← Row 1: type selector (1 line)
  ├─────────────────────────────────┤
  │ Token:                         │  ← Row 2: field label (1 line)
  │ ┌─────────────────────────────┐│
  │ │ eyJhbGciOiJIUzI1NiIs...     ││  ← Row 3+: TextArea for value
  │ └─────────────────────────────┘│
  └─────────────────────────────────┘
  ```

- [x] **NoAuth layout**: Display centered "No authentication configured" message
- [x] **Bearer layout**: "Token:" label + token TextArea (uses remaining vertical space)
- [x] **Basic layout**: "Username:" label + username TextArea (3 lines) + "Password:" label + password TextArea (3 lines)
- [x] **API Key layout**: "Key:" label + key name TextArea (2 lines) + "Value:" label + key value TextArea (2 lines) + "Add to: [Header]" toggle row (1 line)

- [x] Use `Layout::vertical()` with constraints:
  - Type selector row: `Constraint::Length(1)`
  - Separator: `Constraint::Length(1)`
  - Content: `Constraint::Min(0)` (fills remaining space)

- [x] Highlight the currently focused auth sub-field:
  - If `app.focus.request_field == RequestField::Auth`, use `app.focus.auth_field` to determine which sub-field to highlight
  - Focused field: bordered block with accent color
  - Unfocused fields: bordered block with dim color

**D.3: Wire into `render_request_panel()` (`src/ui/mod.rs:351-385`)**

- [x] Add `RequestTab::Auth` match arm:
  ```rust
  RequestTab::Auth => {
      render_auth_panel(frame, app, layout.content_area);
  }
  ```

**D.4: Update `prepare_editors()` (`src/app.rs:1929-1987`)**

- [x] Add auth TextAreas to `prepare_editors()`:
  - Set block/cursor styles for `auth_token_editor`, `auth_username_editor`, `auth_password_editor`, `auth_key_name_editor`, `auth_key_value_editor`
  - Follow the same focus/unfocus pattern: focused editor gets accent border + visible cursor; unfocused editors get dim border + hidden cursor
  - Only prepare editors that are relevant to the current `auth_type` (e.g., skip token editor when auth type is Basic)

**D.5: Update status bar hints (`src/ui/mod.rs:1084-1109`)**

- [x] Add hint text for Auth tab:
  - On `AuthField::AuthType`: "Enter: change auth type"
  - On `AuthField::Token`: "i/Enter: edit token | Shift+H/L: switch tab"
  - On `AuthField::Username` / `AuthField::Password`: "i/Enter: edit | j/k: next/prev field"
  - On `AuthField::KeyName` / `AuthField::KeyValue`: "i/Enter: edit | j/k: next/prev field"
  - On `AuthField::KeyLocation`: "Enter: toggle Header/Query Param"

**D.6: Compile and verify rendering**

- [x] Compile
- [x] Manual test: switch to Auth tab — see "No authentication configured" (default NoAuth)
- [x] Tab bar shows "Auth" label
- [x] Status bar shows correct hints for Auth tab

**Commit**: `feat(ui): render auth tab with type selector and per-type field layout`

---

### Phase E: Auth Type Popup + Field Editing

Wire up interaction: selecting auth type, editing fields, navigating between sub-fields.

**E.1: Add auth type popup state to `App`**

- [x] Add fields to `App`:
  ```rust
  pub show_auth_type_popup: bool,
  pub auth_type_popup_index: usize,
  ```

- [x] Initialize in `App::new()`: `show_auth_type_popup: false`, `auth_type_popup_index: 0`

**E.2: Render auth type popup in `src/ui/mod.rs`**

- [x] Create `render_auth_type_popup()` — follows the pattern of `render_method_popup()`:
  - Popup options: "No Auth", "Bearer Token", "Basic Auth", "API Key"
  - j/k navigation with wrap-around
  - Enter selects, Esc cancels
  - Render as an overlay centered in the auth tab content area

**E.3: Handle auth type popup keys in `src/app.rs`**

- [x] In the main key handler, check `show_auth_type_popup` first (like `show_method_popup`):
  - `j` / `Down`: increment `auth_type_popup_index` (mod 4)
  - `k` / `Up`: decrement `auth_type_popup_index` (mod 4)
  - `Enter`: set `self.request.auth_type = AuthType::from_index(popup_index)`, close popup, clear previous auth data, set `request_dirty = true`
  - `Esc`: close popup without changing auth type

**E.4: Auth sub-field navigation in Navigation mode**

- [x] When `focus.request_field == RequestField::Auth` and in Navigation mode:
  - `j` / `Down`: move to next auth sub-field (within current auth type's fields)
  - `k` / `Up`: move to previous auth sub-field
  - `Enter` on `AuthField::AuthType`: open auth type popup
  - `Enter` on `AuthField::KeyLocation`: toggle `api_key_location` between Header and QueryParam
  - `Enter` / `i` on text fields: enter Editing mode on the corresponding TextArea

- [x] Define valid auth fields per auth type:
  - NoAuth: `[AuthType]` (only the type selector)
  - Bearer: `[AuthType, Token]`
  - Basic: `[AuthType, Username, Password]`
  - ApiKey: `[AuthType, KeyName, KeyValue, KeyLocation]`

- [x] `next_auth_field()` and `prev_auth_field()` methods navigate within valid fields for current type

**E.5: Wire auth TextAreas into editing mode**

- [x] Update `active_editor()` (`src/app.rs:431-438`). Note: `active_editor()` is on `App` (has access to both `self.request` and `self.focus`), so this works:
  ```rust
  RequestField::Auth => match self.focus.auth_field {
      AuthField::Token => Some(&mut self.request.auth_token_editor),
      AuthField::Username => Some(&mut self.request.auth_username_editor),
      AuthField::Password => Some(&mut self.request.auth_password_editor),
      AuthField::KeyName => Some(&mut self.request.auth_key_name_editor),
      AuthField::KeyValue => Some(&mut self.request.auth_key_value_editor),
      _ => None, // AuthType and KeyLocation are not TextAreas
  }
  ```

- [x] Update `is_editable_field()` (`src/app.rs:2681-2686`):
  - Return `true` for `RequestField::Auth` when `auth_field` is a text field (Token, Username, Password, KeyName, KeyValue)
  - Return `false` for `AuthField::AuthType` and `AuthField::KeyLocation` (these use popup/toggle, not text editing)

**E.6: Compile and verify interaction**

- [x] Compile
- [x] Manual test: Enter on auth type → popup appears → select Bearer → token field appears
- [x] Manual test: j/k navigates between auth sub-fields
- [x] Manual test: i on token field → vim Insert mode → type token → Esc → back to Normal → Esc → back to Navigation
- [x] Manual test: switch to Basic → username/password fields appear, token field gone
- [x] Manual test: Enter on API Key location → toggles Header/Query Param
- [x] Manual test: Shift+H/L still switches tabs from within Auth tab

**Commit**: `feat(app): add auth type popup and field editing interactions`

---

### Phase F: Auth Injection + Save/Load Integration

The core behavioral change: auth settings affect HTTP requests and persist to storage.

**F.1: Define `AuthConfig` enum for HTTP layer**

- [x] Add to `src/http.rs` (or a shared location):
  ```rust
  pub enum AuthConfig {
      NoAuth,
      Bearer { token: String },
      Basic { username: String, password: String },
      ApiKey { key: String, value: String, location: ApiKeyLocation },
  }
  ```

**F.2: Extend `send_request()` in `src/http.rs`**

- [x] Add `auth: &AuthConfig` parameter to the function signature
- [x] After building the reqwest request builder (method + URL), inject auth:
  ```rust
  let builder = match auth {
      AuthConfig::NoAuth => builder,
      AuthConfig::Bearer { token } => builder.bearer_auth(token),
      AuthConfig::Basic { username, password } => builder.basic_auth(username, Some(password)),
      AuthConfig::ApiKey { key, value, location } => match location {
          ApiKeyLocation::Header => builder.header(key, value),
          ApiKeyLocation::QueryParam => builder.query(&[(key, value)]),
      },
  };
  ```
- [x] Note: `reqwest::RequestBuilder::query()` appends query params to the URL. This correctly handles URLs that already have query parameters.

**F.3: Build `AuthConfig` from `RequestState` in `src/app.rs`**

- [x] Add `build_auth_config(&self) -> AuthConfig` method on `RequestState`:
  ```rust
  pub fn build_auth_config(&self) -> AuthConfig {
      match self.auth_type {
          AuthType::NoAuth => AuthConfig::NoAuth,
          AuthType::Bearer => AuthConfig::Bearer {
              token: self.auth_token_text(),
          },
          AuthType::Basic => AuthConfig::Basic {
              username: self.auth_username_text(),
              password: self.auth_password_text(),
          },
          AuthType::ApiKey => AuthConfig::ApiKey {
              key: self.auth_key_name_text(),
              value: self.auth_key_value_text(),
              location: self.api_key_location,
          },
      }
  }
  ```

**F.4: Update `App::send_request()` (`src/app.rs:2649-2672`)**

- [x] Extract auth config alongside existing data:
  ```rust
  let auth = self.request.build_auth_config();
  ```
- [x] Pass `&auth` to `http::send_request()` in the spawned task
- [x] **No conflict detection for MVP**: If the user has both auth configured and a manual `Authorization:` header in the Headers tab, both are sent. The user is responsible for conflicts. This matches Postman's behavior.

**F.5: Update `build_postman_request()` (`src/app.rs:1124-1135`)**

- [x] Serialize auth state to `PostmanAuth`:
  ```rust
  let auth = match self.request.auth_type {
      AuthType::NoAuth => None,
      AuthType::Bearer => Some(PostmanAuth::bearer(&self.request.auth_token_text())),
      AuthType::Basic => Some(PostmanAuth::basic(
          &self.request.auth_username_text(),
          &self.request.auth_password_text(),
      )),
      AuthType::ApiKey => Some(PostmanAuth::apikey(
          &self.request.auth_key_name_text(),
          &self.request.auth_key_value_text(),
          if self.request.api_key_location == ApiKeyLocation::Header { "header" } else { "query" },
      )),
  };
  ```
- [x] Set `postman_request.auth = auth`

**F.6: Update `open_request()` (`src/app.rs:1137-1157`)**

- [x] When loading a `PostmanItem`, extract auth and populate editors:
  ```rust
  if let Some(auth) = &postman_request.auth {
      match auth.auth_type.as_str() {
          "bearer" => {
              self.request.auth_type = AuthType::Bearer;
              if let Some(token) = auth.get_bearer_token() {
                  self.request.auth_token_editor = TextArea::new(vec![token.to_string()]);
              }
          }
          "basic" => {
              self.request.auth_type = AuthType::Basic;
              if let Some((username, password)) = auth.get_basic_credentials() {
                  self.request.auth_username_editor = TextArea::new(vec![username.to_string()]);
                  self.request.auth_password_editor = TextArea::new(vec![password.to_string()]);
              }
          }
          "apikey" => {
              self.request.auth_type = AuthType::ApiKey;
              if let Some((key, value, location)) = auth.get_apikey() {
                  self.request.auth_key_name_editor = TextArea::new(vec![key.to_string()]);
                  self.request.auth_key_value_editor = TextArea::new(vec![value.to_string()]);
                  self.request.api_key_location = match location {
                      "query" => ApiKeyLocation::QueryParam,
                      _ => ApiKeyLocation::Header,
                  };
              }
          }
          _ => {
              // Unsupported auth type — ignore, default to NoAuth
              self.request.auth_type = AuthType::NoAuth;
          }
      }
  } else {
      self.request.auth_type = AuthType::NoAuth;
  }
  ```

**F.7: Mark request dirty on auth changes**

- [x] Set `self.request_dirty = true` when:
  - Auth type changes (popup selection)
  - Any auth TextArea content changes (in editing mode)
  - API Key location toggles
- [x] This triggers auto-save on the next save cycle

**F.8: Compile and verify end-to-end**

- [x] Compile
- [x] Manual test: **Bearer Token flow**
  1. Select Auth tab → change type to Bearer → enter token → Ctrl+R to send
  2. Verify request includes `Authorization: Bearer <token>` header
  3. Save request, close, reopen → verify token is restored
- [x] Manual test: **Basic Auth flow**
  1. Change type to Basic → enter username "user" + password "pass" → send
  2. Verify request includes `Authorization: Basic dXNlcjpwYXNz` header
  3. Save/reload → verify credentials restored
- [x] Manual test: **API Key (Header) flow**
  1. Change type to API Key → key "X-API-Key", value "abc123", location Header → send
  2. Verify request includes `X-API-Key: abc123` header
- [x] Manual test: **API Key (Query Param) flow**
  1. Toggle location to Query Param → send to httpbin.org/get
  2. Verify URL has `?X-API-Key=abc123` appended (visible in response args)
- [x] Manual test: **Auth + manual header coexistence**
  1. Set Bearer token, then add `Authorization: Bearer old` in Headers tab
  2. Send → verify both headers are sent (no crash, no conflict resolution)
- [x] Manual test: **NoAuth default**
  1. New request → verify no auth headers injected
- [x] Manual test: **Roundtrip**
  1. Set Bearer auth, save collection, close Perseus, reopen → auth restored

**Commit**: `feat(http): inject auth into requests and persist to Postman collection`

---

## Acceptance Criteria

### Functional Requirements

- [x] Three auth types available: Bearer Token, Basic Auth, API Key
- [x] Auth type selector popup with j/k + Enter navigation
- [x] Bearer auth injects `Authorization: Bearer <token>` header
- [x] Basic auth injects `Authorization: Basic <base64>` header (encoded by reqwest)
- [x] API Key auth injects custom header OR query parameter based on location setting
- [x] API Key location toggles between Header and Query Param with Enter
- [x] Auth settings auto-inject at send time without modifying visible headers/URL text

### Data Integrity

- [x] Auth settings persist per-request in Postman Collection v2.1 format
- [x] Save → reload roundtrip preserves auth type, all field values, and API Key location
- [x] Collections without auth fields load correctly (default NoAuth) — backward compatible
- [x] Unsupported Postman auth types (OAuth, Digest, etc.) gracefully default to NoAuth

### UI/UX

- [x] Auth tab appears between Headers and Body in the tab bar
- [x] Tab label shows auth type: "Auth (Bearer)" / "Auth (Basic)" / "Auth (API Key)" / "Auth"
- [x] j/k navigates between auth sub-fields within the tab
- [x] Full vim editing (Normal/Insert/Visual) works on all auth text fields
- [x] Status bar hints update for Auth tab context
- [x] Shift+H/L cycles through all 3 tabs (Headers ↔ Auth ↔ Body)

### Edge Cases

- [x] Empty auth fields: sending with empty token/username/password sends the header with empty values (user responsibility)
- [x] Auth type switch clears previous type's data
- [x] API Key query param works with URLs that already have query parameters
- [x] No conflict detection: if user has both auth config and manual Authorization header, both are sent (user responsibility)

---

## Dependencies & Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Auth tab rendering complexity (dynamic fields per type) | Medium | Start with Bearer (simplest: 1 field), iterate. Vertical stack layout is straightforward. |
| 5 new TextArea editors on RequestState increase memory | Low | TextArea is lightweight. Only relevant editors are prepared per render cycle. |
| `prepare_editors()` grows complex with auth fields | Medium | Only prepare editors for the active auth type. Add a helper method to reduce branching. |
| Auth type popup conflicts with method popup | Low | Only one popup can be open at a time. Check `show_auth_type_popup` before `show_method_popup` in key handler priority. |
| Unsupported Postman auth types silently default to NoAuth | Low | Acceptable for MVP. When Postman import ships, extend auth structs to handle additional types. |
| API Key query param injection with malformed URLs | Low | reqwest's `.query()` handles URL encoding. Edge cases (fragment handling) deferred to reqwest's implementation. |

## References

- **Brainstorm**: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` — Phase 1.2
- **Postman Collection v2.1 Auth Schema**: `https://schema.postman.com/collection/json/v2.1.0/draft-07/collection.json`
- **reqwest Auth API**: `RequestBuilder::bearer_auth()`, `RequestBuilder::basic_auth()`, `RequestBuilder::query()`
- **RFC 7617**: HTTP Basic Authentication — Base64 encoding of `username:password`
- **RFC 6750**: Bearer Token Usage — `Authorization: Bearer <token>` format
- **Existing plan template**: `docs/plans/2026-02-15-feat-additional-http-methods-plan.md`
