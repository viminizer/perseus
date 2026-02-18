---
status: done
priority: p2
issue_id: "011"
tags: [code-review, quality, duplication]
dependencies: []
---

# Duplicated JSON Detection Functions

## Problem Statement

`is_json_like()` in `src/app.rs` and `is_json_response()` in `src/ui/mod.rs` both detect whether content is JSON but use different heuristics. This duplication can lead to inconsistent behavior where one detects JSON but the other doesn't.

## Findings

- **Source**: Pattern Recognition, Code Simplicity reviewer
- **Location**: `src/app.rs` (is_json_like), `src/ui/mod.rs` (is_json_response)
- **Severity**: IMPORTANT - inconsistent behavior, maintenance burden

## Proposed Solutions

### Option A: Consolidate into single function (Recommended)
- Keep one canonical `is_json()` function, remove the other
- Place it in a shared location (e.g., a utils module or on ResponseData)
- **Pros**: Single source of truth, consistent behavior
- **Cons**: Minor refactor
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] Only one JSON detection function exists
- [x] All call sites use the consolidated function
- [x] JSON detection behavior is consistent across app and UI

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | DRY - consolidate detection heuristics |
| 2026-02-18 | Resolved: single is_json_content() in app.rs, imported by ui/mod.rs | Header check + structural sniffing in one place |

## Resources

- PR #9: feat/response-metadata-and-search branch
