---
status: done
priority: p2
issue_id: "013"
tags: [code-review, quality, duplication]
dependencies: []
---

# Triplicated Clipboard and Environment Popup Logic

## Problem Statement

The Ctrl+N environment popup toggle logic is repeated 3 times across different input handling modes. Clipboard paste/copy logic is also triplicated. This violates DRY and makes behavior changes require updates in 3 places.

## Findings

- **Source**: Pattern Recognition, Code Simplicity reviewer
- **Location**: Multiple locations in `src/app.rs` (handle_navigation_mode, handle_editing_mode, handle_sidebar_mode)
- **Severity**: IMPORTANT - maintenance burden, risk of inconsistent behavior

## Proposed Solutions

### Option A: Extract shared handler methods (Recommended)
- Create `toggle_env_popup()`, `handle_clipboard_paste()`, `handle_clipboard_copy()` methods
- Call from each mode handler
- **Pros**: DRY, single point of change
- **Cons**: Minor refactor
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] Environment popup toggle code exists in one place
- [x] Clipboard logic exists in one place
- [x] All modes call the shared methods
- [x] Behavior remains identical in all modes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Extract shared logic across modal handlers |
| 2026-02-18 | Resolved: extracted toggle_env_popup() and handle_env_popup_input() methods | Clipboard methods already existed as shared methods; env popup was the true triplication |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs`
