# Perseus Roadmap

## Milestone: v1.0 — Core HTTP Client

### Phase 1: Foundation
**Goal:** Project setup with ratatui + tokio, basic app loop running

- Fix Cargo.toml (edition 2021)
- Add dependencies: ratatui, crossterm, tokio, reqwest
- Create basic Application struct with event loop
- Terminal setup/teardown with panic handler
- Verify app starts and exits cleanly

**Research:** None — standard Rust TUI setup

---

### Phase 2: TUI Layout
**Goal:** Panel layout with request/response areas and navigation

- Define UI layout (request panel left/top, response panel right/bottom)
- Implement panel components with borders and titles
- Add input widgets: URL field, method selector, headers, body
- Keyboard navigation between panels and fields
- Focus indicators and visual feedback

**Research:** None — ratatui widget patterns

---

### Phase 3: HTTP Integration
**Goal:** Make requests and display responses beautifully

- Wire up reqwest client with tokio runtime
- Send request on user action (Enter key)
- Parse and display response: status, headers, body
- JSON syntax highlighting for response body
- Loading indicator during request
- Error display for failed requests

**Research:** None — reqwest + tokio standard patterns

---

### Phase 4: Polish
**Goal:** Smooth keyboard UX and visual refinement

- Vim-style keybindings (hjkl navigation, i for insert)
- Tab completion or method cycling
- Status bar with help hints
- Color scheme and visual consistency
- Edge case handling (large responses, timeouts)

**Research:** None — UX refinement

---

## Summary

| Phase | Name | Goal |
|-------|------|------|
| 1 | Foundation | Project setup, app loop running |
| 2 | TUI Layout | Panels, inputs, keyboard nav |
| 3 | HTTP Integration | Requests, responses, formatting |
| 4 | Polish | Vim keys, status bar, edge cases |

---
*Created: 2026-02-04*
