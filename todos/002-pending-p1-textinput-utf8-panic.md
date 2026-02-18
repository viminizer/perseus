---
status: completed
priority: p1
issue_id: "002"
tags: [code-review, bug, crash, unicode]
dependencies: []
---

# TextInput Panics on Multi-byte UTF-8 Characters

## Problem Statement

The `TextInput` struct at `src/app.rs:448-492` tracks cursor position by byte offset but manipulates it as if it were a character index. `insert_char` increments cursor by 1 instead of `ch.len_utf8()`, and `backspace`/`move_left` step back by 1 byte instead of 1 char boundary. This causes panics when users type non-ASCII characters (accented letters, CJK, emoji).

## Findings

- **Source**: Security Sentinel, Rust Quality reviewer
- **Location**: `src/app.rs:448-492` (TextInput struct methods)
- **Severity**: CRITICAL - panic/crash on non-ASCII input
- **Evidence**: `insert_char` does `self.cursor += 1` instead of `self.cursor += ch.len_utf8()`. `backspace` does `self.cursor -= 1` which can land in the middle of a multi-byte sequence, causing `String::remove` to panic at a non-char-boundary.

## Proposed Solutions

### Option A: Track cursor as char index (Recommended)
- Store cursor as character count, convert to byte offset only when needed for string operations
- Use `char_indices()` for navigation
- **Pros**: Clean mental model, prevents all byte/char confusion
- **Cons**: O(n) conversion for each operation (negligible for URL/header lengths)
- **Effort**: Medium
- **Risk**: Low

### Option B: Track cursor as byte offset correctly
- Fix all arithmetic to use `ch.len_utf8()` for insertion, `floor_char_boundary` for movement
- **Pros**: Efficient for long strings
- **Cons**: Easy to introduce new bugs, every operation must be careful
- **Effort**: Medium
- **Risk**: Medium

## Acceptance Criteria

- [ ] Typing multi-byte characters (e.g., `e`, emoji, CJK) does not panic
- [ ] Cursor navigates correctly through mixed ASCII/non-ASCII text
- [ ] Backspace deletes one character (not one byte)
- [ ] Text content remains valid UTF-8 after all operations

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Byte vs char cursor tracking is a common Rust string bug |
| 2026-02-18 | Verified fixed — cursor tracked as char index with `byte_offset()` conversion | Option A implemented at lines 456-516 |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs:448-492`
