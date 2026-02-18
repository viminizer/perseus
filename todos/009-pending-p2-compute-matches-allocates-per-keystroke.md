---
status: done
priority: p2
issue_id: "009"
tags: [code-review, performance, search]
dependencies: []
---

# compute_matches() Allocates Full Body Copy Per Keystroke

## Problem Statement

`compute_matches()` at `src/app.rs:590-634` creates a full copy of the response body (via `to_lowercase()`) on every keystroke during search. For large responses, this causes noticeable input lag.

## Findings

- **Source**: Performance Oracle
- **Location**: `src/app.rs:590-634`
- **Severity**: HIGH - input lag with large responses during search
- **Evidence**: `text.to_lowercase()` allocates a new string on every call

## Proposed Solutions

### Option A: Cache the lowercased body (Recommended)
- Store the lowercased version when the response body changes, reuse it for searches
- **Pros**: Single allocation, fast search
- **Cons**: Doubles memory for response body
- **Effort**: Small
- **Risk**: Low

### Option B: Debounce search computation
- Only recompute matches after a brief pause in typing (e.g., 100ms)
- **Pros**: Reduces frequency of expensive operation
- **Cons**: Delayed search results
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] Search typing is responsive even with large response bodies (1MB+)
- [x] No redundant string allocation per keystroke

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Cache derived data, don't recompute per frame |
| 2026-02-18 | Resolved: compute_matches caches body_generation/query/case_sensitive, early-returns on no-change | Char-aware matching avoids full to_lowercase() allocation |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs:590-634`
