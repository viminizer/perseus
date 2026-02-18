---
status: done
priority: p2
issue_id: "007"
tags: [code-review, security, file-access]
dependencies: []
---

# Arbitrary File Read via Multipart/Binary Body

## Problem Statement

`src/http.rs:144-177` reads arbitrary files from disk for multipart and binary body types. There is no path validation, so a user could inadvertently (or via a loaded collection) send sensitive files like `~/.ssh/id_rsa` as request bodies. While the user controls input, imported Postman collections could contain malicious file paths.

## Findings

- **Source**: Security Sentinel
- **Location**: `src/http.rs:144-177`
- **Severity**: HIGH - arbitrary file read exfiltrated over HTTP
- **Evidence**: `std::fs::read(path)` with no validation on what files can be read

## Proposed Solutions

### Option A: Add user confirmation for file paths (Recommended)
- Show the full resolved path and file size before sending
- Require explicit confirmation for sensitive directories
- **Pros**: User stays informed, prevents accidental exfiltration
- **Cons**: Extra UX step
- **Effort**: Medium
- **Risk**: Low

### Option B: Restrict to working directory
- Only allow file paths within the current working directory or project root
- **Pros**: Strong containment
- **Cons**: May be too restrictive for legitimate use cases
- **Effort**: Small
- **Risk**: Medium (usability impact)

## Acceptance Criteria

- [x] User is informed what file will be sent before request executes
- [x] File paths are resolved and displayed as absolute paths

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-02-18 | Identified during PR #9 review | Imported collections can contain arbitrary file paths |
| 2026-02-18 | Implemented validate_file_path() in src/http.rs | Path canonicalization, regular-file check, sensitive-dir blocklist, 100 MB size cap |

## Resources

- PR #9: feat/response-metadata-and-search branch
- File: `src/http.rs:144-177`
