# Perseus

A terminal-based HTTP client with Postman-level ambitions. Built in Rust with ratatui and tokio.

## Current State

**Shipped:** v1.0 — Core HTTP Client (2026-02-04)

A fully functional TUI HTTP client featuring:
- Request/response split-panel layout
- HTTP methods: GET, POST, PUT, PATCH, DELETE
- Headers and body input
- JSON syntax highlighting
- Vim-style keybindings (i/Esc mode switching, hjkl navigation)
- Status bar with mode indicator and key hints
- Response body scrolling

## Next Milestone Goals

*Not yet defined — run `/gsd:new-milestone` to plan v1.1*

Potential directions:
- Request history/saving
- Collections/workspaces
- Environment variables
- Auth handling (OAuth, API keys)

## Vision

Postman for the terminal — a sophisticated TUI for making HTTP requests, viewing responses, and managing API workflows. Start lean, grow into full feature parity.

## Requirements

### Validated (v1.0)

- [x] Make HTTP requests (GET, POST, PUT, DELETE, etc.) to any URL
- [x] Display responses with beautiful formatting (syntax highlighting, headers, status)
- [x] Keyboard-driven navigation throughout the UI
- [x] Input fields for URL, method, headers, and body
- [x] Clean visual separation of request and response panels

### Out of Scope (Future)

- Collections/workspaces
- Auth handling (OAuth, API keys, bearer tokens)
- Environment variables (dev/staging/prod configs)
- Request history/saving

## Constraints

| Constraint | Rationale |
|------------|-----------|
| ratatui for TUI | User requirement, modern Rust TUI standard |
| tokio for async | User requirement, de facto async runtime |
| Rust 2021 edition | Current stable |

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| ratatui + tokio stack | User specified, industry standard choices | Shipped v1.0 |
| v1 focuses on request/response + UX | Build solid foundation before features | Shipped v1.0 |
| No persistence in v1 | Keep scope minimal, validate core flow first | Deferred to v1.1+ |
| Vim-style mode switching | Familiar to power users, clean separation of navigation/editing | Shipped v1.0 |

---
*Last updated: 2026-02-04*
