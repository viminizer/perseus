---
status: completed
priority: p3
issue_id: "018"
tags: [code-review, testing]
dependencies: []
---

# No Unit Tests for app.rs or ui/mod.rs

## Problem Statement

The two largest files in the codebase (`src/app.rs` at 4700+ lines, `src/ui/mod.rs` at 1700+ lines) have zero unit tests. Only `src/storage/environment.rs` has tests. This makes refactoring risky and regressions undetectable.

## Findings

- **Source**: Rust Quality reviewer, Code Simplicity reviewer
- **Location**: `src/app.rs`, `src/ui/mod.rs`
- **Severity**: NICE-TO-HAVE - no tests means no safety net for future changes

## Proposed Solutions

### Option A: Add targeted tests for critical logic (Recommended)
- Start with pure logic functions: `compute_matches()`, `TextInput` methods, `format_response_size()`, `substitute()`
- These are easily testable without TUI setup
- **Pros**: High ROI - tests the most bug-prone code
- **Cons**: Doesn't cover UI rendering
- **Effort**: Medium
- **Risk**: Low

## Acceptance Criteria

- [x] `TextInput` methods have unit tests covering ASCII and Unicode
- [x] `compute_matches()` has tests for case-sensitive/insensitive modes
- [x] `format_response_size()` has boundary tests

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Test pure logic first for highest ROI |
| 2026-02-18 | Added 71 unit tests to src/app.rs | Covered format_size, is_json_content, Method FromStr, TextInput (ASCII + Unicode), ResponseSearch compute_matches (case-sensitive, case-insensitive, caching, Unicode, overlapping). Discovered a bug in case-sensitive search with multibyte chars mid-text. |

## Resources

- PR #9: feat/response-metadata-and-search branch
