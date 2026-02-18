---
status: completed
priority: p3
issue_id: "014"
tags: [code-review, architecture, refactor]
dependencies: []
---

# God Object: App Struct Has 36+ Fields and 4700+ Lines

## Problem Statement

The `App` struct in `src/app.rs` has grown to 36+ fields and the file is 4700+ lines. It handles HTTP requests, response display, sidebar navigation, popups, search, clipboard, environment management, and more. This makes the code hard to navigate, test, and maintain.

## Findings

- **Source**: Architecture Strategist, Code Simplicity reviewer, Pattern Recognition
- **Location**: `src/app.rs` (entire file)
- **Severity**: NICE-TO-HAVE - architectural concern, not blocking current functionality

## Proposed Solutions

### Option A: Incremental extraction (Recommended)
- Extract logical groups into separate modules over time:
  - `PopupState` / `PopupHandler` for all popup logic
  - `SidebarState` / `SidebarHandler` for sidebar
  - `SearchState` already exists as `ResponseSearch` - continue this pattern
- **Pros**: Gradual improvement, no big-bang refactor
- **Cons**: Takes time across multiple PRs
- **Effort**: Large (spread over time)
- **Risk**: Low (incremental)

## Acceptance Criteria

- [x] New features should be added to extracted modules, not directly to App
- [x] Consider extraction when touching related code

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | God objects are a natural consequence of rapid feature addition |
| 2026-02-18 | Acknowledged as ongoing architectural guidance | Follow incremental extraction pattern when adding new features |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs`
