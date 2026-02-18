# Brainstorm: Production-Ready Feature Set for Perseus

**Date:** 2026-02-15
**Status:** Draft
**Participants:** Kevin, Claude

---

## What We're Building

A comprehensive feature set to transform Perseus from a functional TUI HTTP client into a production-ready tool that can serve as a full Postman/Insomnia alternative for any developer who prefers working in the terminal.

**Target user:** All developers — backend/API developers, full-stack developers, and terminal power users.

**Core philosophy:** Keyboard-driven, vim-modal, fast, and terminal-native. No electron, no browser, no GUI overhead.

---

## Current State

Perseus today has:
- Full vim-modal editing (Normal/Insert/Visual/Operator-pending)
- 5 HTTP methods (GET, POST, PUT, PATCH, DELETE)
- Raw text request bodies only
- Postman Collection v2.1 storage format
- Rich sidebar tree management (CRUD, search, move, indent/outdent)
- System clipboard integration
- Session restoration per project
- JSON syntax highlighting in responses
- Event-driven render loop with performance caching

---

## Feature Roadmap

### Phase 1: Foundation (Core HTTP & Data)

These features fill the most critical gaps — without them, Perseus can't handle standard API workflows.

#### 1.1 Configuration File ✅ DONE
- Location: `~/.config/perseus/config.toml`
- Settings:
  - `http.timeout` (default: 30s)
  - `http.follow_redirects` (default: true)
  - `http.max_redirects` (default: 10)
  - `proxy.url`, `proxy.no_proxy`
  - `ssl.ca_cert`, `ssl.client_cert`, `ssl.verify`
  - `ui.sidebar_width` (default range)
  - `history.max_entries` (default: 500)
  - `editor.tab_size` (default: 2)
- TOML format for readability
- Layered: global config < project config (`.perseus/config.toml`) < per-request overrides
- **Rationale for Phase 1:** Multiple later features (proxy, SSL, timeout, redirects, history) depend on config infrastructure. Building it first avoids retrofitting.

#### 1.2 Authentication Support 🔄 IN PROGRESS (plan exists)
- **Bearer Token**: Dedicated auth tab/section, token field, auto-injects `Authorization: Bearer <token>` header
- **Basic Auth**: Username + password fields, auto-encodes to Base64 `Authorization: Basic <encoded>` header
- **API Key**: Key name + value + location (header or query param), auto-injects into the right place
- Auth settings stored per-request in the Postman collection format (which already supports `auth` field)
- Auth tab in the request panel alongside Headers and Body

#### 1.3 Environment Variables
- Named environments (dev, staging, production, custom)
- Key-value variable pairs per environment
- `{{variable}}` substitution in URL, headers, body, and auth fields
- Quick-switch hotkey (needs binding — see Open Questions)
- Environment stored in `.perseus/environments.json` (or per-env files)
- Visual indicator of active environment in status bar
- Global variables that apply across all environments
- Environment-specific overrides layer on top of globals

#### 1.4 Additional HTTP Methods ✅ DONE
- Add HEAD, OPTIONS
- Custom method input: allow users to type any arbitrary method string
- Update method selector popup to include new methods

#### 1.5 Request Body Types
- **JSON**: Syntax validation, auto-set Content-Type header, pretty-format on paste
- **Form URL-encoded**: Key-value editor UI, auto-encode, auto-set Content-Type
- **Multipart Form Data**: Key-value + file attachment fields, file picker (path input), auto-set Content-Type with boundary
- **XML**: Syntax mode indicator, auto-set Content-Type
- **Binary**: File path input, read and send as binary body
- **Raw** (existing): Keep current plain text mode
- Body type selector (dropdown/tabs) that switches the editor mode

#### 1.6 Query Parameter Editor
- Dedicated key-value UI for URL query parameters
- Bidirectional sync: editing params updates the URL, editing the URL updates params
- Toggle individual params on/off without deleting them
- Tab in request panel: Params | Auth | Headers | Body

#### 1.7 Response Metadata & Search (Quick Wins)
- Display response size (bytes/KB/MB) in status line alongside status code and duration
- One-key copy of entire response body to clipboard (e.g., `Y` in response panel)
- Save response body to file (hotkey, prompted for path)
- `/` key in response panel triggers search mode
- Incremental search with highlighting
- `n`/`N` for next/previous match
- Case-sensitive/insensitive toggle

---

### Phase 2: Import/Export & Interop

Enable users to bring existing work in and share requests out.

#### 2.1 Curl Import
- Parse curl command strings into Perseus requests
- Support common curl flags: -X, -H, -d, --data, -u, -b, -k, --compressed, -F
- Import via: paste into a popup, or CLI argument (`perseus --import-curl "curl ..."`)
- Handle quoted strings, multi-line curl commands

#### 2.2 Curl Export
- Generate curl command from current request
- Copy to clipboard with one key
- Include auth, headers, body, method, URL
- Handle special characters and quoting properly

#### 2.3 Postman Collection Import
- Import Postman Collection v2.1 JSON files
- Map Postman's folder/request structure to Perseus tree
- Import auth, headers, body, URL, method
- Preserve folder hierarchy
- CLI: `perseus --import-postman path/to/collection.json`

#### 2.4 OpenAPI/Swagger Import
- Parse OpenAPI 3.x and Swagger 2.0 specs (JSON and YAML)
- Generate requests for each endpoint with example parameters
- Organize by tags into folders
- Import path parameters, query parameters, headers, request body schemas
- CLI: `perseus --import-openapi path/to/spec.yaml`

#### 2.5 Code Generation
- Generate code snippets from the current request
- Languages: curl, Python (requests), JavaScript (fetch), Go (net/http), Rust (reqwest)
- Copy to clipboard or display in a popup

#### 2.6 Request History
- Log of all sent requests with timestamp, method, URL, status, duration
- Browsable history list (popup or sidebar tab)
- Replay any historical request
- Persistent storage in `.perseus/history.json`
- Configurable history limit via config file

#### 2.7 Request Notes/Documentation
- Markdown description field per request
- Stored in Postman collection format (which supports `description` field)
- Viewable/editable from request panel (new tab: Docs)
- Useful for team context when sharing collections

---

### Phase 3: Advanced HTTP & Networking

Features needed for working with real-world APIs in corporate and complex environments.

#### 3.1 Proxy Configuration
- HTTP and SOCKS5 proxy support
- Per-request or global proxy setting (via config file)
- Proxy authentication (username/password)
- No-proxy list for bypassing

#### 3.2 SSL/TLS Certificate Management
- Custom CA certificate paths
- Client certificate + key for mutual TLS
- Option to disable SSL verification (with warning)
- Settings in config file

#### 3.3 Cookie Jar
- Automatic cookie storage and sending across requests
- Per-collection or per-environment cookie jar
- View/edit/delete cookies in a dedicated panel
- Option to disable cookie handling per request
- Persistent storage in `.perseus/cookies.json`

#### 3.4 Redirect & Timeout Control
- Per-request timeout setting (override config default)
- Follow/no-follow redirects toggle
- Max redirects limit
- Show redirect chain in response

---

### Phase 4: Major Extensions

These are architecturally significant features that extend Perseus beyond standard HTTP.

#### 4.1 Multi-Tab Workspace
- Open multiple requests in tabs
- Tab bar showing request names
- Switch between tabs with hotkeys (e.g., gt/gT vim-style, or Ctrl+1-9)
- Each tab has independent request/response state
- Close tab, reorder tabs
- **Complexity note:** This is a major architectural change. The current `App` struct holds a single `RequestState`. Multi-tab requires refactoring to a `Vec<RequestState>` or similar, affecting the entire state machine, keybinding dispatch, and rendering pipeline. Plan carefully.

#### 4.2 GraphQL Support
- Dedicated GraphQL mode (auto-detected or manually toggled)
- Query editor with syntax awareness
- Variables editor (JSON)
- Schema introspection (fetch schema from endpoint)
- Operation name support
- Auto-set Content-Type to application/json and wrap query in proper JSON structure

#### 4.3 WebSocket Support
- Connect to WebSocket endpoints
- Send and receive messages
- Message history view
- Connection status indicator
- Dedicated WebSocket mode (separate from HTTP request mode)
- Support text and binary frames
- **Complexity note:** Requires a persistent async connection model, distinct from the fire-and-wait HTTP pattern. Needs its own UI layout (send input + scrolling message log).

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Authentication types | Bearer, Basic, API Key | Covers ~90% of REST API use cases without OAuth complexity |
| Environment system | Full named environments with layered variables | Standard approach, user expectation from Postman/Bruno |
| Import formats | Curl, Postman Collection, OpenAPI/Swagger | Maximum onboarding paths; curl is universal, Postman for migration, OpenAPI for API-first teams |
| Body types | Raw, JSON, Form URL-encoded, Multipart, XML, Binary | Full coverage for all common API body formats |
| HTTP methods | 5 existing + HEAD, OPTIONS, custom | HEAD/OPTIONS are commonly needed; custom allows niche use |
| Scripting/automation | Deferred | Keep Perseus focused as a manual testing tool; scripting adds significant complexity |
| Config format | TOML, Phase 1 | Readable, standard for Rust CLI tools; moved to Phase 1 because Phases 2-4 depend on config infrastructure |
| WebSocket | Included | Increasingly needed; fits the HTTP client scope |
| GraphQL | Included | Growing portion of API development; dedicated mode adds real value |
| Multi-tab | Included | Essential for comparing requests/responses and working with multiple endpoints |
| Theming | Deferred | Config file for functional settings only; themes can come later |

---

## Open Questions

1. **Environment file format**: Should environments be stored in a single JSON file or individual files per environment (like Bruno does)? Individual files are more git-friendly.
2. **OpenAPI import depth**: Should we parse full request body schemas and generate example bodies, or just extract endpoint paths and methods?
3. **WebSocket UI**: Should WebSocket be a separate mode/panel or integrated into the existing request/response layout?
4. **Multi-tab limit**: Should there be a max number of open tabs, or let memory be the limit?
5. **GraphQL schema caching**: Cache introspected schemas to disk, or fetch fresh each time?
6. **Config file precedence**: If a project `.perseus/config.toml` conflicts with global `~/.config/perseus/config.toml`, which wins? (Proposed: project overrides global)
7. **History scope**: Should history be global or per-project?
8. **Hotkey allocation**: Several new features need keybindings (environment switch, curl import, code gen, history). Ctrl+E is already used for sidebar toggle. Need a comprehensive keybinding audit before Phase 1 planning to avoid conflicts.
9. **Request panel tab overflow**: With Params, Auth, Headers, Body, and Docs tabs, the request panel may overflow in narrow terminals. Consider a scrollable tab bar or abbreviated labels.
10. **Multi-tab timing**: Should multi-tab be built before or after environment variables? Environments apply per-tab, so the interaction model matters.

---

## Phase Priority Summary

| Phase | Focus | Impact | Complexity |
|-------|-------|--------|------------|
| Phase 1 | Config, Auth, Environments, Body Types, Methods, Query Params, Response Search/Metadata | **Critical** — unblocks standard API workflows | Medium-High |
| Phase 2 | Curl/Postman/OpenAPI Import, Curl Export, Code Gen, History, Request Notes | **High** — enables adoption and sharing | Medium |
| Phase 3 | Proxy, SSL, Cookies, Redirect/Timeout Control | **Medium** — needed for enterprise/advanced use | Low-Medium |
| Phase 4 | Multi-Tab, GraphQL, WebSocket | **Medium** — major extensions beyond standard HTTP | **High** — architectural changes |

---

## Implementation Strategy

**One plan per feature.** Each feature gets its own plan file in `docs/plans/` with:
- Multiple implementation phases (small, verifiable steps)
- Each phase verified (compiles, tests pass, manual check) and committed before moving to the next
- Plan file naming: `docs/plans/YYYY-MM-DD-<feature-name>.md`

This keeps PRs focused, makes rollbacks clean, and allows features to be developed independently or in parallel.

**Suggested implementation order** (within Phase 1):
1. Configuration file — foundation for later features
2. Additional HTTP methods — smallest scope, quick win
3. Authentication support — highest user-facing impact
4. Request body types — unlocks standard API workflows
5. Environment variables — enables multi-environment workflows
6. Query parameter editor — UI enhancement, depends on URL parsing
7. Response metadata & search — quick wins, high daily use

## Next Steps

Pick a feature and run `/workflows:plan` to create its implementation plan.
