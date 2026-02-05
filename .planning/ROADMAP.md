# Perseus Roadmap

## Completed Milestones

- [v1.0 — Core HTTP Client](milestones/v1.0-ROADMAP.md) (2026-02-04) — TUI HTTP client with vim-style keybindings

## Current Milestone: v1.1 — Collections & Workspaces

### Phase 5: UI Improvements
**Goal:** Better layouts, error display, visual polish

- Improve error message display and styling
- Add loading spinners/progress indicators
- Refine panel proportions and spacing
- Add help overlay (? key)
- Visual feedback for user actions

**Research:** None

---

### Phase 6: Persistence
**Goal:** File-based storage for requests and collections

- Design storage format (JSON)
- Create storage module with read/write operations
- Define data models for saved requests
- Handle file errors gracefully
- Storage location (~/.perseus/ or configurable)

**Research:** None

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

| Phase | Name | Goal |
|-------|------|------|
| 5 | UI Improvements | Better layouts, error display, polish |
| 6 | Persistence | File-based storage layer |
| 7 | Collections | Save/load/manage requests |
| 8 | Workspaces | Organize collections, import/export |

---
*Last updated: 2026-02-05*
