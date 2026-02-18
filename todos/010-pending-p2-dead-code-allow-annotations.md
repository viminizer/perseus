---
status: done
priority: p2
issue_id: "010"
tags: [code-review, quality, dead-code]
dependencies: []
---

# Dead Code Hidden by #[allow(dead_code)] and #![allow(unused)]

## Problem Statement

Multiple `#[allow(dead_code)]` annotations exist on functions like `build_body_content` and `build_auth_config` in `src/app.rs`. Additionally, `src/storage/mod.rs` has a module-wide `#![allow(unused)]` that suppresses all unused warnings. These hide genuinely unused code that should either be used or removed.

## Findings

- **Source**: Pattern Recognition, Code Simplicity reviewer
- **Location**: `src/app.rs` (build_body_content, build_auth_config), `src/storage/mod.rs`
- **Severity**: IMPORTANT - dead code increases maintenance burden and hides real issues

## Proposed Solutions

### Option A: Remove dead code and allow annotations (Recommended)
- Delete genuinely unused functions
- Remove `#![allow(unused)]` from storage/mod.rs
- Wire up functions that should be used but aren't yet
- **Pros**: Cleaner codebase, compiler catches future dead code
- **Cons**: None
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] No `#[allow(dead_code)]` annotations on unused functions
- [x] No module-wide `#![allow(unused)]`
- [x] `cargo clippy` remains clean after removal

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Allow annotations mask real issues |
| 2026-02-18 | Resolved: all #[allow(dead_code)] and #![allow(unused)] removed, dead functions deleted | Compiler now catches future dead code |

## Resources

- PR #9: feat/response-metadata-and-search branch
