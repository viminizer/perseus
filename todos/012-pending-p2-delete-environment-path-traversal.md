---
status: done
priority: p2
issue_id: "012"
tags: [code-review, security, path-traversal]
dependencies: []
---

# delete_environment_file Lacks Name Validation

## Problem Statement

`delete_environment_file()` at `src/storage/environment.rs:98-107` constructs a file path from a user-provided environment name without validating it. Names containing `../` could delete files outside the environments directory.

## Findings

- **Source**: Security Sentinel
- **Location**: `src/storage/environment.rs:98-107`
- **Severity**: IMPORTANT - path traversal in file deletion

## Proposed Solutions

### Option A: Validate environment name (Recommended)
- Reuse or extend `is_safe_env_name()` to reject names with path separators
- Canonicalize the final path and verify it's within the environments directory
- **Pros**: Prevents traversal, uses existing validation pattern
- **Cons**: None
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] Environment names with `../` or path separators are rejected
- [x] `delete_environment_file` only deletes files within the environments directory
- [x] Error message shown for invalid environment names

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Always validate names used in file paths |
| 2026-02-18 | Resolved: is_safe_env_name() rejects path separators, canonicalize() + starts_with() as defense-in-depth | Two-layer validation: name regex + canonical prefix check |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/storage/environment.rs:98-107`
