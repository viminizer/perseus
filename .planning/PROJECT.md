# Perseus

A terminal-based HTTP client with Postman-level ambitions. Built in Rust with ratatui and tokio.

## Vision

Postman for the terminal — a sophisticated TUI for making HTTP requests, viewing responses, and managing API workflows. Start lean, grow into full feature parity.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] Make HTTP requests (GET, POST, PUT, DELETE, etc.) to any URL
- [ ] Display responses with beautiful formatting (syntax highlighting, headers, status)
- [ ] Keyboard-driven navigation throughout the UI
- [ ] Input fields for URL, method, headers, and body
- [ ] Clean visual separation of request and response panels

### Out of Scope

- Collections/workspaces — deferred to future version
- Auth handling (OAuth, API keys, bearer tokens) — deferred to future version
- Environment variables (dev/staging/prod configs) — deferred to future version
- Request history/saving — deferred to future version

## Constraints

| Constraint | Rationale |
|------------|-----------|
| ratatui for TUI | User requirement, modern Rust TUI standard |
| tokio for async | User requirement, de facto async runtime |
| Rust 2021 edition | Current stable (fix from 2024 in Cargo.toml) |

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| ratatui + tokio stack | User specified, industry standard choices | — Pending |
| v1 focuses on request/response + UX | Build solid foundation before features | — Pending |
| No persistence in v1 | Keep scope minimal, validate core flow first | — Pending |

## Success Criteria

**v1 is successful when:**
1. User can input a URL and method
2. User can add headers and body
3. Request is sent and response displayed beautifully
4. Navigation feels smooth and keyboard-native
5. Works reliably for basic HTTP workflows

---
*Last updated: 2026-02-04 after initialization*
