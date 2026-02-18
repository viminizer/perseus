---
status: completed
priority: p3
issue_id: "017"
tags: [code-review, quality, encapsulation]
dependencies: []
---

# Excessive pub Visibility on App Fields

## Problem Statement

Most fields on the `App` struct are `pub`, exposing internal state directly. This makes it difficult to enforce invariants and creates a wide API surface that's hard to maintain.

## Findings

- **Source**: Rust Quality reviewer, Architecture Strategist
- **Location**: `src/app.rs` (App struct definition)
- **Severity**: NICE-TO-HAVE - encapsulation concern

## Proposed Solutions

### Option A: Reduce visibility incrementally
- Make fields `pub(crate)` where cross-module access is needed
- Make fields private where only App methods use them
- Add accessor methods where needed
- **Pros**: Better encapsulation, clearer API
- **Cons**: Requires auditing each field's usage
- **Effort**: Medium
- **Risk**: Low

## Acceptance Criteria

- [x] New fields added to App should be private by default
- [x] Existing fields can be reduced to `pub(crate)` when touching related code

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Default to private, expose as needed |
| 2026-02-18 | Audited all field usage across codebase and reduced visibility | Fields accessed from `src/ui/mod.rs` set to `pub(crate)`; fields only used within `src/app.rs` made private; 5 fields made private (`config`, `client`, `collection`, `current_request_id`, `request_dirty`), 33 fields changed from `pub` to `pub(crate)` |

## Resources

- PR #9: feat/response-metadata-and-search branch
