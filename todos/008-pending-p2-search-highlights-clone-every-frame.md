---
status: done
priority: p2
issue_id: "008"
tags: [code-review, performance, rendering]
dependencies: []
---

# apply_search_highlights() Clones All Lines Every Frame

## Problem Statement

`apply_search_highlights()` in `src/ui/mod.rs` (around line 1371-1375) clones ALL response body lines on every render frame to apply search highlighting. For large responses (e.g., 10MB JSON), this creates significant allocation pressure and GC-like behavior, causing visible UI stutter.

## Findings

- **Source**: Performance Oracle, Code Simplicity reviewer
- **Location**: `src/ui/mod.rs:1371-1375`
- **Severity**: HIGH - performance degradation with large responses
- **Evidence**: Full clone of lines vector on every frame, even when search matches haven't changed

## Proposed Solutions

### Option A: Cache highlighted lines (Recommended)
- Only recompute highlights when search query or matches change (use generation counter)
- Store highlighted `Vec<Line>` in a cache invalidated by search state changes
- **Pros**: Eliminates per-frame allocation, leverages existing cache pattern
- **Cons**: Additional cache management
- **Effort**: Medium
- **Risk**: Low

### Option B: Apply highlights in-place
- Modify spans in-place instead of cloning the entire line vector
- **Pros**: Zero allocation
- **Cons**: More complex lifetime management
- **Effort**: Medium
- **Risk**: Medium

## Acceptance Criteria

- [x] Search highlights don't cause per-frame allocation of the entire response body
- [x] Large responses (1MB+) with active search remain responsive
- [x] Highlights update correctly when search query changes

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Cache highlight results using generation counters |
| 2026-02-18 | Resolved: highlight_search_gen cache + generation counter prevents per-frame cloning | Cached highlighted_lines reused until search state changes |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/ui/mod.rs:1371-1375`
