---
status: completed
priority: p1
issue_id: "004"
tags: [code-review, bug, search]
dependencies: []
---

# Search Byte-Offset Mismatch in Case-Insensitive Mode

## Problem Statement

`compute_matches()` at `src/app.rs:590-634` lowercases both the haystack and needle for case-insensitive search, then records byte offsets from the lowercased text. However, `.to_lowercase()` can change byte lengths (e.g., German `` (2 bytes) becomes `ss` (2 bytes, but different), and some Unicode characters change byte width when lowercased). The byte offsets from the lowercased copy are then applied to highlight the original (non-lowercased) text, causing incorrect highlighting or potential panics at non-char boundaries.

## Findings

- **Source**: Performance Oracle, Rust Quality reviewer
- **Location**: `src/app.rs:590-634` (compute_matches / ResponseSearch)
- **Severity**: CRITICAL - can cause panics or garbled highlights with certain Unicode input
- **Evidence**: Offsets computed on `text.to_lowercase()` are used to index into the original `text`

## Proposed Solutions

### Option A: Use char-aware case-insensitive matching (Recommended)
- Iterate through the original text using `char_indices()` and compare chars case-insensitively
- Record byte offsets from the original text directly
- **Pros**: Correct for all Unicode, offsets always valid
- **Cons**: Slightly more complex implementation
- **Effort**: Medium
- **Risk**: Low

### Option B: Use a regex with case-insensitive flag
- Use `regex::Regex` with `(?i)` flag, which handles Unicode correctly
- Match positions are from the original text
- **Pros**: Well-tested Unicode handling, simple API
- **Cons**: Adds regex dependency, need to escape user input
- **Effort**: Small
- **Risk**: Low

## Acceptance Criteria

- [ ] Case-insensitive search produces correct highlight positions on original text
- [ ] No panics with Unicode text containing characters that change byte-width when lowercased
- [ ] Search highlights visually match the found text

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | to_lowercase() can change byte lengths |
| 2026-02-18 | Verified fixed — char-aware matching records offsets from original text at lines 685-722 | Option A implemented |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/app.rs:590-634`
