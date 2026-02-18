---
status: done
priority: p2
issue_id: "006"
tags: [code-review, security, path-traversal]
dependencies: []
---

# Path Traversal in save_response_to_file

## Problem Statement

`save_response_to_file` at `src/app.rs:2326-2364` performs incomplete tilde expansion and does not canonicalize the path. A user-supplied filename like `../../etc/cron.d/malicious` could write outside the intended directory. While this is a local-only TUI app (the user controls input), it's still a defense-in-depth concern.

## Findings

- **Source**: Security Sentinel
- **Location**: `src/app.rs:2326-2364`
- **Severity**: HIGH - path traversal allows writing to arbitrary locations
- **Evidence**: Tilde expansion is partial, no `canonicalize()` or path prefix check

## Proposed Solutions

### Option A: Canonicalize and validate path (Recommended)
- Resolve the path with `std::fs::canonicalize()` or validate it starts with an expected prefix
- **Pros**: Prevents directory escape
- **Cons**: Minor additional code
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [x] File save rejects paths containing `..` traversal
- [x] Tilde expansion works correctly for `~/` prefix
- [x] User gets clear error message for invalid paths

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Always validate user-provided file paths |
| 2026-02-18 | Resolved: save_response_to_file rejects `..` via component check, full tilde expansion, error toasts | Defense-in-depth with component iteration |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs:2326-2364`
