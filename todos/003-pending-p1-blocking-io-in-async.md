---
status: completed
priority: p1
issue_id: "003"
tags: [code-review, bug, async, performance]
dependencies: []
---

# Blocking `std::fs::read` Inside Async Context

## Problem Statement

`src/http.rs` uses `std::fs::read()` at lines 146 and 167 inside the async `send_request()` function. This blocks the tokio runtime thread, which can cause the entire TUI event loop to freeze while reading large files (multipart uploads or binary body).

## Findings

- **Source**: Performance Oracle, Rust Quality reviewer
- **Location**: `src/http.rs:146` (multipart file read), `src/http.rs:167` (binary body file read)
- **Severity**: CRITICAL - blocks async runtime, freezes UI
- **Evidence**: `std::fs::read(path)` is synchronous and will block the tokio worker thread

## Proposed Solutions

### Option A: Use `tokio::fs::read` (Recommended)
- Replace `std::fs::read` with `tokio::fs::read().await`
- **Pros**: Non-blocking, idiomatic async Rust
- **Cons**: Adds tokio fs dependency (likely already available)
- **Effort**: Small
- **Risk**: Low

### Option B: Use `tokio::task::spawn_blocking`
- Wrap the `std::fs::read` call in `spawn_blocking`
- **Pros**: Works without changing the API
- **Cons**: More boilerplate
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [ ] File reads in `send_request` are non-blocking
- [ ] TUI remains responsive during file upload
- [ ] Large file uploads don't freeze the event loop

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Always use tokio::fs in async context |
| 2026-02-18 | Verified fixed — `tokio::fs::read()` used at http.rs lines 255 and 281 | Option A implemented |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/http.rs:146,167`
