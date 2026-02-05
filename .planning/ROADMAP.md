# Perseus Roadmap

## Completed Milestones

- [v1.0 — Core HTTP Client](milestones/v1.0-ROADMAP.md) (2026-02-04) — TUI HTTP client with vim-style keybindings

## Current Milestone: v1.1 — Collections & Workspaces

### Phase 5: UI Improvements ✓
**Goal:** Better layouts, error display, visual polish

- [x] Improve error message display and styling
- [x] Add loading spinners/progress indicators
- [x] Refine panel proportions and spacing
- [x] Add help overlay (? key)

**Plans:** 1 completed

---

### Phase 6: Persistence ✓
**Goal:** File-based storage for requests and collections

- [x] Design storage format (JSON)
- [x] Create storage module with read/write operations
- [x] Define data models for saved requests
- [x] Handle file errors gracefully
- [x] Storage location (project-local: `<project>/.perseus/requests/`)

**Plans:** 1 completed

---

### Phase 7: Collections
**Goal:** Save requests to named collections, list/select/delete

- Create collection data model
- Add "Save Request" action (Ctrl+S or similar)
- Collection browser panel/modal
- Load saved request into editor
- Delete requests from collections

**Research:** None

---

### Phase 8: Workspaces
**Goal:** Group collections, switch between workspaces, import/export

- Workspace data model (contains collections)
- Workspace switcher UI
- Create/rename/delete workspaces
- Import/export workspace as JSON
- Default workspace on startup

**Research:** None

---

## Summary

| Phase | Name | Status |
|-------|------|--------|
| 5 | UI Improvements | Complete |
| 6 | Persistence | Complete |
| 7 | Collections | Pending |
| 8 | Workspaces | Pending |

---
*Last updated: 2026-02-05*
