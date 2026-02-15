---
title: "feat: Add environment variable system with named environments and {{variable}} substitution"
type: feat
date: 2026-02-15
---

# feat: Add Environment Variable System

## Overview

Add named environments (dev, staging, production, custom) with key-value variable pairs and `{{variable}}` substitution in all request fields (URL, headers, body, auth). Users create environment files as JSON, switch between them with `Ctrl+N`, and see the active environment in the status bar.

## Problem Statement

Perseus currently has no way to parameterize requests. Users who work with multiple API environments must:

| Gap | Impact |
|-----|--------|
| No environment concept | Users manually edit URLs between `localhost:3000` and `api.staging.example.com` |
| No variable substitution | Base URLs, API keys, tokens, and common values are duplicated across requests |
| No quick environment switching | Changing from dev to production requires editing every request |

This is the single most requested feature class for any HTTP client tool — it's table-stakes for daily API development workflows.

## Proposed Solution

A four-phase implementation, each phase independently compilable and committable:

1. **Phase A**: Environment data model and file I/O (storage layer)
2. **Phase B**: Substitution engine (pure functions, no UI dependency)
3. **Phase C**: App state integration (load environments at startup, track active)
4. **Phase D**: Quick-switch popup + status bar indicator + wire substitution into send

## Technical Approach

### Current Architecture (Send Flow)

```
User presses Ctrl+R
    |
    v
send_request() ------------------------------------------------> tokio::spawn
    |                                                              |
    |-- url = self.request.url_text()                              |
    |-- headers = self.request.headers_text()                      v
    |-- body = self.request.body_text()               http::send_request(&client,
    |-- auth = self.request.build_auth_config()             &method, &url,
    |                                                       &headers, &body, &auth)
    +-- (no transformation step)
```

### Target Architecture (With Environment Substitution)

```
User presses Ctrl+R
    |
    v
send_request()
    |
    |-- url = self.request.url_text()
    |-- headers = self.request.headers_text()
    |-- body = self.request.body_text()
    |-- auth fields = self.request.auth_*_text()
    |
    v
+-------------------------------------+
|  resolve_variables(active_env)      |
|  -> HashMap<String, String>         |
|                                     |
|  substitute("{{base_url}}/users")   |
|  -> "https://api.dev.example.com/users"|
+-------------------------------------+
    |
    v
tokio::spawn --> http::send_request(&client, &method,
                      &resolved_url, &resolved_headers,
                      &resolved_body, &resolved_auth)
```

### Storage Layout

```
.perseus/
|-- collection.json          # Existing -- unchanged
|-- config.toml              # Existing -- unchanged
|-- environments/            # NEW -- one file per environment
|   |-- dev.json
|   |-- staging.json
|   +-- production.json
+-- ui.json                  # Existing -- unchanged
```

Individual files per environment because:
- Simpler to implement: `load_all_environments()` reads every `.json` in a directory — filesystem is the index
- More git-friendly: each environment is a separate file diff
- Easier to share specific environments (e.g., `dev.json` for the team, `local.json` for personal)

### Key Files and Touchpoints

| File | What Changes |
|------|-------------|
| `src/storage/environment.rs` | **NEW** -- `Environment`, `EnvironmentVariable` data models; file I/O; `substitute()` and `resolve_variables()` functions |
| `src/storage/mod.rs` | Re-export environment module |
| `src/storage/project.rs` | Add `environments_dir()` and `ensure_environments_dir()` helpers |
| `src/app.rs` | Add `environments`/`active_environment_name` to `App`; integrate substitution into `send_request()`; add env popup state; add `Ctrl+N` handler |
| `src/ui/mod.rs` | Render environment popup; render env indicator in status bar |

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage format | Individual JSON files per environment | Filesystem is the index. Simpler than managing an array in a single file. |
| File location | `.perseus/environments/*.json` | Consistent with existing `.perseus/` convention. Per-project. |
| JSON structure | Postman-compatible `{name, values: [{key, value, enabled, type}]}` | Future-proofs Postman environment import (Phase 2.3). Established schema. |
| Environment identifier | Name (= filename stem) | Natural key. No UUID indirection. Filesystem enforces uniqueness. |
| Variable syntax | `{{variable_name}}` (double curly braces) | Industry standard (Postman, Bruno, Insomnia). Unambiguous in URLs/headers/JSON. |
| Substitution timing | At send time only | Variables are replaced when Ctrl+R is pressed, not in the editor. The editor always shows raw `{{var}}` templates. |
| Substitution scope | URL, headers, body, auth fields | All user-editable text fields. Method and response are excluded. |
| Quick-switch hotkey | `Ctrl+N` | Available (Ctrl+E/P/S/R are taken). Mnemonic: eNvironment. |
| Missing variable handling | Leave `{{var}}` as literal | Matches Postman behavior. Non-blocking -- request still sends. User sees unresolved vars in the URL/response. |
| Environment management | Users edit JSON files directly | Terminal developers are comfortable editing small JSON files. In-app CRUD deferred to v2 after the feature proves its value. |
| Global variables | Deferred | Users create a named "globals" or "shared" environment. Same effect, no separate concept. Add proper globals in v2 if users request cross-environment defaults. |
| Session persistence of active env | Deferred | Users press Ctrl+N once after launch. Avoids touching SessionState. Add in v2. |
| Nested substitution | Not supported | `{{a}}` values are not re-scanned for `{{b}}`. Single pass. |
| Escaping `{{` syntax | No escape mechanism in v1 | If `{{name}}` has no matching variable, it's left as literal text. |
| Clipboard behavior | Copy unresolved (raw) text | Users see and copy `{{base_url}}/api` -- the template. Resolved values are for sending only. |
| Variable value types | Strings only | Consistent with Postman format. Simple and sufficient. |
| Naming restrictions | Alphanumeric + underscore + hyphen for env names | Safe for filenames across all OS. |

---

## Implementation Phases

### Phase A: Environment Data Model and File I/O

Add the data structures and file persistence for environments.

**A.1: Create `src/storage/environment.rs`**

- [x] Define `EnvironmentVariable` struct (Postman-compatible):
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct EnvironmentVariable {
      pub key: String,
      pub value: String,
      #[serde(default = "default_true")]
      pub enabled: bool,
      #[serde(rename = "type", default = "default_type")]
      pub var_type: String,
  }

  fn default_true() -> bool { true }
  fn default_type() -> String { "default".to_string() }
  ```

- [x] Define `Environment` struct:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Environment {
      pub name: String,
      #[serde(default)]
      pub values: Vec<EnvironmentVariable>,
  }
  ```

- [x] Constructor: `EnvironmentVariable::new(key: &str, value: &str) -> EnvironmentVariable` -- `enabled: true`, `var_type: "default"`

**A.2: Add file I/O functions**

- [x] `environments_dir() -> Option<PathBuf>` in `src/storage/project.rs` -- returns `.perseus/environments/` (follows the pattern of existing `storage_dir()`)
- [x] `ensure_environments_dir() -> Result<PathBuf, String>` in `src/storage/project.rs` -- creates the dir if missing (follows the pattern of existing `ensure_storage_dir()`)
- [x] `load_environment(path: &Path) -> Result<Environment, String>` -- reads and deserializes one env file
- [x] `save_environment(env: &Environment) -> Result<(), String>` -- serializes to `.perseus/environments/<name>.json`. Validate that `env.name` is a safe filename (alphanumeric + underscore + hyphen, non-empty) before writing; return `Err` otherwise.
- [x] `load_all_environments() -> Result<Vec<Environment>, String>` -- reads all `.json` files from environments dir, returns empty Vec if dir doesn't exist
- [x] `delete_environment_file(name: &str) -> Result<(), String>` -- removes a specific env file

**A.3: Wire into storage module**

- [x] Add `mod environment;` to `src/storage/mod.rs`
- [x] Re-export public items:
  ```rust
  pub use environment::{
      Environment, EnvironmentVariable,
      load_all_environments, save_environment, delete_environment_file,
  };
  ```
- [x] Add `environments_dir` and `ensure_environments_dir` to the existing `pub use project::{...}` block in `src/storage/mod.rs`

**A.4: Verify**

- [x] Compile -- no other code touches environment structs yet
- [x] Write a test: create an Environment, serialize to JSON, verify Postman-compatible format
- [x] Write a test: load/save roundtrip

**Commit**: `feat(storage): add environment variable data model and file I/O`

---

### Phase B: Substitution Engine

Pure functions that replace `{{variable}}` patterns with resolved values. Lives in the same `src/storage/environment.rs` file alongside the data model.

**B.1: Implement substitution functions**

- [x] Add `substitute(template: &str, variables: &HashMap<String, String>) -> (String, Vec<String>)`:
  - Returns `(resolved_text, unresolved_variable_names)`
  - Use a simple state-machine scan (not regex, to avoid the `regex` dependency):
    - Scan for `{{`
    - Capture chars until `}}`
    - Look up captured name in `variables` HashMap
    - If found: replace with value
    - If not found: leave `{{name}}` as-is, add name to unresolved Vec
  - Handle edge cases:
    - Empty variable name: `{{}}` -- leave as-is
    - Unclosed braces: `{{name` -- leave as-is (no closing `}}`)
    - Adjacent variables: `{{a}}{{b}}` -- both resolved
    - Variable in URL path: `{{base_url}}/api/{{version}}/users`

  ```rust
  pub fn substitute(template: &str, variables: &HashMap<String, String>) -> (String, Vec<String>) {
      let mut result = String::with_capacity(template.len());
      let mut unresolved = Vec::new();
      let mut chars = template.chars().peekable();

      while let Some(c) = chars.next() {
          if c == '{' && chars.peek() == Some(&'{') {
              chars.next(); // consume second '{'
              let mut name = String::new();
              let mut closed = false;
              while let Some(nc) = chars.next() {
                  if nc == '}' && chars.peek() == Some(&'}') {
                      chars.next();
                      closed = true;
                      break;
                  }
                  name.push(nc);
              }
              if closed && !name.is_empty() {
                  if let Some(val) = variables.get(&name) {
                      result.push_str(val);
                  } else {
                      result.push_str("{{");
                      result.push_str(&name);
                      result.push_str("}}");
                      unresolved.push(name);
                  }
              } else {
                  result.push_str("{{");
                  result.push_str(&name);
                  if !closed { /* unclosed -- already consumed */ }
              }
          } else {
              result.push(c);
          }
      }
      (result, unresolved)
  }
  ```

- [x] Implement `resolve_variables(env: Option<&Environment>) -> HashMap<String, String>`:
  - Collect only enabled variables from the environment into a HashMap
  - If `env` is None, return empty HashMap

  ```rust
  pub fn resolve_variables(env: Option<&Environment>) -> HashMap<String, String> {
      let mut vars = HashMap::new();
      if let Some(env) = env {
          for var in &env.values {
              if var.enabled {
                  vars.insert(var.key.clone(), var.value.clone());
              }
          }
      }
      vars
  }
  ```

**B.2: Unit tests**

- [x] Test basic substitution: `"{{host}}/api"` -> `"localhost:3000/api"`
- [x] Test multiple variables: `"{{scheme}}://{{host}}:{{port}}"`
- [x] Test unresolved variable: `"{{missing}}"` stays as `"{{missing}}"`, appears in unresolved
- [x] Test empty template: `""` -> `""`
- [x] Test no variables in template: `"https://example.com"` -> unchanged
- [x] Test only enabled variables are used: disabled var is skipped
- [x] Test edge cases: unclosed braces, empty name `{{}}`
- [x] Test adjacent variables: `"{{a}}{{b}}"` -> both resolved

**Commit**: `feat(env): add {{variable}} substitution engine`

---

### Phase C: App State Integration

Load environments at startup and track the active environment in the `App` struct.

**C.1: Add environment state to `App` struct in `src/app.rs`**

- [x] Add fields to `App`:
  ```rust
  pub environments: Vec<Environment>,
  pub active_environment_name: Option<String>,  // None = "No Environment"
  ```

- [x] Add import: `use crate::storage::environment::{self, Environment};`

**C.2: Load environments in `App::new()`**

- [x] In `App::new()`, after `CollectionStore::load_or_init()` (line ~694 of `app.rs`) and before the `Self { ... }` struct literal (line ~812), load environments:
  ```rust
  let environments = storage::load_all_environments().unwrap_or_default();
  ```
- [x] Initialize `active_environment_name: None`

**C.3: Add helper method on `App`**

- [x] `active_environment(&self) -> Option<&Environment>` -- finds the env matching `active_environment_name`:
  ```rust
  fn active_environment(&self) -> Option<&Environment> {
      self.active_environment_name.as_ref()
          .and_then(|name| self.environments.iter().find(|e| e.name == *name))
  }
  ```

**C.4: Verify**

- [x] Compile with empty environments dir -- app starts normally, `active_environment_name = None`
- [x] Manually create a `.perseus/environments/dev.json` file, verify it loads on startup

**Commit**: `feat(app): load environments at startup and track active environment`

---

### Phase D: Quick-Switch Popup + Status Bar + Send Substitution

Add `Ctrl+N` to open an environment switcher popup, display the active environment in the status bar, and wire substitution into the send flow.

**D.1: Add popup state to `App`**

- [x] Add fields to `App`:
  ```rust
  pub show_env_popup: bool,
  pub env_popup_index: usize,
  ```

**D.2: Add `Ctrl+N` keybinding in `handle_navigation_mode()` (`src/app.rs`)**

- [x] Add handler alongside existing Ctrl+E/P/S/R:
  ```rust
  if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
      // Close any other open popups first (mutual exclusion)
      self.show_method_popup = false;
      self.show_auth_type_popup = false;
      self.show_env_popup = !self.show_env_popup;
      if self.show_env_popup {
          // Index: 0 = "No Environment", 1..N = environments
          self.env_popup_index = self.active_environment_name
              .as_ref()
              .and_then(|name| self.environments.iter().position(|e| e.name == *name))
              .map(|i| i + 1)
              .unwrap_or(0);
      }
      self.dirty = true;
      return;
  }
  ```

- [x] Add `Ctrl+N` in `handle_editing_mode()` as well (same pattern as Ctrl+R -- works from any mode)
- [x] Add `Ctrl+N` in `handle_sidebar_mode()` -- there are three `AppMode` variants: `Navigation`, `Editing`, and `Sidebar`

**D.3: Handle popup keys**

- [x] Add popup key handling in `handle_navigation_mode()` after the `show_help` check but before `show_auth_type_popup` and `show_method_popup` checks (matching the existing popup priority chain at lines ~2384-2398 of `app.rs`):
  ```rust
  if self.show_env_popup {
      match key.code {
          KeyCode::Char('j') | KeyCode::Down => {
              let count = self.environments.len() + 1; // +1 for "No Environment"
              self.env_popup_index = (self.env_popup_index + 1) % count;
          }
          KeyCode::Char('k') | KeyCode::Up => {
              let count = self.environments.len() + 1;
              self.env_popup_index = (self.env_popup_index + count - 1) % count;
          }
          KeyCode::Enter => {
              self.active_environment_name = if self.env_popup_index == 0 {
                  None
              } else {
                  Some(self.environments[self.env_popup_index - 1].name.clone())
              };
              self.show_env_popup = false;
          }
          KeyCode::Esc | KeyCode::Char('q') => {
              self.show_env_popup = false;
          }
          _ => {}
      }
      self.dirty = true;
      return;
  }
  ```

**D.4: Render environment popup in `src/ui/mod.rs`**

- [x] Add `render_env_popup(frame: &mut Frame, app: &App)` function:
  - Follow the same pattern as `render_method_popup()` / `render_auth_type_popup()`
  - Items: `["No Environment", env1.name, env2.name, ...]`
  - Highlight current selection with inverted colors
  - Show checkmark next to active environment
  - Position: centered overlay (like help) since this is a global action

- [x] Wire into `render()` in `src/ui/mod.rs`. Add after `show_auth_type_popup` check and before `show_help` check (help overlay should always render on top of everything):
  ```rust
  if app.show_env_popup {
      render_env_popup(frame, app);
  }
  ```

**D.5: Add environment indicator to status bar**

- [x] In `render_status_bar()`, add an environment indicator span. The current code builds `status_spans` as a single `vec![mode, "  ", panel_info, "  |  ", hints]`. Insert the env indicator after the initial vec construction, before the clipboard toast check (before the `if let Some(msg) = app.clipboard_toast_message()` block):
  ```rust
  // After status_spans vec construction, before clipboard toast:
  if let Some(env_name) = app.active_environment_name.as_deref() {
      status_spans.push(Span::raw("  |  "));
      status_spans.push(Span::styled(
          format!(" {} ", env_name),
          Style::default()
              .fg(Color::Black)
              .bg(Color::Blue)
              .add_modifier(Modifier::BOLD),
      ));
  }
  ```

**D.6: Wire substitution into `send_request()`**

- [x] After extracting raw text, apply substitution:
  ```rust
  fn send_request(&mut self, tx: mpsc::Sender<Result<ResponseData, String>>) {
      let raw_url = self.request.url_text();
      if raw_url.is_empty() {
          self.response = ResponseStatus::Error("URL is required".to_string());
          return;
      }

      if matches!(self.response, ResponseStatus::Loading) {
          return;
      }

      // Resolve variables from active environment
      let variables = environment::resolve_variables(self.active_environment());

      let (url, _) = environment::substitute(&raw_url, &variables);
      let (headers, _) = environment::substitute(&self.request.headers_text(), &variables);
      let (body, _) = environment::substitute(&self.request.body_text(), &variables);

      // Build auth config with substituted variables
      let auth = self.build_resolved_auth_config(&variables);

      self.response = ResponseStatus::Loading;

      let client = self.client.clone();
      let method = self.request.method.clone();

      let handle = tokio::spawn(async move {
          let result = http::send_request(&client, &method, &url, &headers, &body, &auth).await;
          let _ = tx.send(result).await;
      });
      self.request_handle = Some(handle.abort_handle());
  }
  ```

**D.7: Substitute in auth fields too**

- [x] Auth is already implemented (`build_auth_config()` in `RequestState`, `AuthConfig` enum in `http.rs`). Apply substitution to auth field values before building the `AuthConfig`:
  - Bearer token: `substitute(auth_token_text, &variables)`
  - Basic auth username/password: substitute both
  - API key name/value: substitute both
  - This ensures `{{api_token}}` in a Bearer token field resolves correctly
- [x] Implement `build_resolved_auth_config(&self, variables: &HashMap<String, String>) -> AuthConfig` on `App` that substitutes variables in auth fields before constructing the `AuthConfig`.

**D.8: Update help overlay**

- [x] Add `Ctrl+n  Switch environment` to the help text
- [x] Update navigation hints in status bar to include `Ctrl+n:env` (existing hints use lowercase, e.g., `Ctrl+r:send`, `Ctrl+s:save`)

**D.9: Verify**

- [x] Create `.perseus/environments/dev.json` with `{"name":"dev","values":[{"key":"base_url","value":"http://localhost:3000","enabled":true,"type":"default"}]}`
- [x] Select "dev" environment via Ctrl+N
- [x] Enter URL: `{{base_url}}/api/users`
- [x] Send request -- verify URL resolves to `http://localhost:3000/api/users`
- [x] Verify status bar shows "dev" indicator
- [x] Enter URL: `{{base_url}}/{{missing}}` -- verify `{{missing}}` is left as literal in the sent request

**Commit**: `feat(env): add Ctrl+N environment switcher popup, status bar indicator, and send-time substitution`

---

## Acceptance Criteria

### Functional Requirements

- [x] `{{variable}}` syntax is substituted in URL, headers, body, and auth fields at send time
- [x] Disabled variables are excluded from substitution
- [x] Unresolved variables are left as literal `{{name}}` in the sent request
- [x] `Ctrl+N` opens environment quick-switch popup from any mode
- [x] Active environment name displayed in status bar
- [x] Environment data stored in `.perseus/environments/*.json` (Postman-compatible format)
- [x] App starts normally with no environments (empty `.perseus/environments/` or missing dir)

### Non-Functional Requirements

- [x] Substitution engine handles large templates without performance issues (no regex, simple scan)
- [x] File I/O errors (corrupt JSON, permission denied) produce user-visible error messages
- [x] No new crate dependencies (substitution uses state-machine scan, not regex)

### Quality Gates

- [x] All phases compile independently with `cargo check`
- [x] Unit tests for substitution engine (edge cases)
- [x] Unit tests for environment file I/O (roundtrip serialization)
- [x] Manual test: full workflow (create env file -> switch env -> send request -> verify substitution)

---

## Risk Analysis & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Environment popup conflicts with other popups | UI bugs | Follow existing pattern: check `show_env_popup` before other popup handlers. Mutual exclusion -- close other popups when opening env popup. |
| Variable substitution in malformed headers | Request fails | Substitution happens on raw text before header parsing. If a variable resolves to something with colons or newlines, header parsing in `http.rs` already handles malformed lines gracefully (returns error). |
| Race between environment edit and send | Stale data | All operations are synchronous on the main thread. The substitution reads the current in-memory state. No race possible with the current architecture. |

---

## Deferred to v2

These features were considered but intentionally deferred to keep v1 minimal:

| Feature | Rationale | Trigger to Add |
|---------|-----------|----------------|
| **Environment management popup** (CRUD UI) | ~500 LOC. Terminal devs can edit JSON files. Phase D (quick-switch) + substitution deliver 90% of value. | Users report friction with file editing |
| **Global variables** (`globals.json`) | A named "globals" environment achieves the same thing without a separate concept. | Users request cross-environment defaults |
| **Session persistence of active env** | Users press Ctrl+N once after launch. Avoids touching SessionState. | User demand |
| **Unresolved variable count in status bar** | Unresolved vars are left as literal `{{name}}` -- visible in the URL/response. | User demand |
| **Secret variable masking** | Terminal devs are comfortable with visible values. | User demand |
| **Dynamic variables** (`{{$timestamp}}`, `{{$randomUUID}}`) | Postman supports these. Could be added as a follow-up. | User demand |
| **Variable autocomplete** | Typing `{{` could show a completion popup. Significant complexity (requires intercepting tui-textarea input). | User demand |

---

## Future Considerations

- **Import Postman environments**: Straightforward once this feature is built -- the storage format is already Postman-compatible. Part of Phase 2.3.
- **Pre-request scripts**: Variable values computed from previous responses. Deferred per brainstorm decision -- keeps Perseus focused as a manual testing tool.
- **Environment file encryption**: For sensitive values. Defer until there's user demand.

---

## References

### Internal References

- Auth plan: `docs/plans/2026-02-15-feat-authentication-support-plan.md` -- same phased approach, reference for pattern
- Config system: `src/config.rs` -- layered config pattern (global + project overlay)
- Storage models: `src/storage/postman.rs` -- Postman v2.1 format reference
- Session state: `src/storage/session_state.rs` -- per-project session persistence pattern
- Project detection: `src/storage/project.rs` -- `.perseus/` directory resolution
- Brainstorm: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` -- Phase 1.3

### External References

- [Postman environment schema](https://github.com/postmanlabs/postman-validator/blob/master/json-schemas/environment.schema.json) -- JSON schema for environment files
- [Postman Collection Format v2.1.0](https://schema.postman.com/collection/json/v2.1.0/draft-07/docs/index.html) -- collection schema reference
