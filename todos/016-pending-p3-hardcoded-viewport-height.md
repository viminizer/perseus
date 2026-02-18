---
status: completed
priority: p3
issue_id: "016"
tags: [code-review, bug, ui]
dependencies: []
---

# Hardcoded Viewport Height of 20 in scroll_to_search_match

## Problem Statement

`scroll_to_search_match` uses a hardcoded viewport height of 20 lines instead of using the actual terminal height. This means search scroll-to-match behavior may not center the match correctly on terminals of different sizes.

## Findings

- **Source**: Performance Oracle
- **Location**: `src/app.rs` (scroll_to_search_match)
- **Severity**: NICE-TO-HAVE - cosmetic issue, search still works

## Proposed Solutions

### Option A: Pass actual viewport height (Recommended)
- Thread the actual content area height from the layout into the scroll calculation
- **Pros**: Correct centering on all terminal sizes
- **Cons**: Requires passing layout info to the scroll function
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] Search match scrolling uses actual viewport height
- [x] Match is centered in the visible area regardless of terminal size

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Avoid hardcoded dimensions in TUI apps |
| 2026-02-18 | Already implemented: response_viewport_height updated from layout each frame in ui/mod.rs:1189 | Dynamic viewport height via render-frame updates |

## Resources

- PR #9: feat/response-metadata-and-search branch
