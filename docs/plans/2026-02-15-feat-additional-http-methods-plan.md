---
title: "feat: Add HEAD, OPTIONS, and custom HTTP method support"
type: feat
date: 2026-02-15
---

# feat: Add HEAD, OPTIONS, and Custom HTTP Method Support

## Overview

Extend Perseus to support HEAD, OPTIONS as first-class HTTP methods, plus allow users to type arbitrary custom method strings (e.g., PURGE, PROPFIND, REPORT). This fills a gap where Perseus only supports 5 methods (GET, POST, PUT, PATCH, DELETE), making it unable to handle common API workflows like health checks (HEAD), CORS preflight inspection (OPTIONS), or WebDAV/custom protocol methods.

## Problem Statement

Perseus currently supports only 5 HTTP methods via a closed `HttpMethod` enum:

| Gap | Impact |
|-----|--------|
| No HEAD method | Cannot perform lightweight endpoint health checks or check response headers without downloading body |
| No OPTIONS method | Cannot inspect CORS configuration or test preflight requests |
| No custom methods | Cannot work with WebDAV (PROPFIND, MKCOL, COPY, MOVE), cache purging (PURGE), or other extension methods |
| `from_str()` maps unknowns to GET | **Data loss bug** — loading a Postman collection containing HEAD/OPTIONS/custom methods silently converts them to GET. Saving overwrites the original method permanently |

## Proposed Solution

A two-phase implementation (each phase is a separate commit; ship as one PR or two — Phase A can stand alone):

1. **Phase A**: Add HEAD and OPTIONS as first-class enum variants. Zero architectural risk — same pattern as existing methods, preserves `Copy` trait.
2. **Phase B**: Add custom method support via a wrapper type that preserves `Copy` for standard methods while allowing arbitrary strings.

## Technical Approach

### Current Architecture

```
User Input (popup)
    │
    ▼
HttpMethod enum (app.rs) ──Copy──▶ RequestState.method
    │                                     │
    ▼                                     ▼
method_color() (ui/mod.rs)         send_request() (http.rs)
    │                                     │
    ▼                                     ▼
render_method_popup()              reqwest::Client match arms
render_sidebar()                   body gating: POST|PUT|PATCH only
    │
    ▼
build_postman_request() ──.as_str()──▶ PostmanRequest.method (String)
    │
    ▼
open_request() ──from_str()──▶ HttpMethod (LOSSY: unknown → GET)
```

### Key Files and Touchpoints

| File | Lines | What Changes |
|------|-------|-------------|
| `src/app.rs` | 125-177 | `HttpMethod` enum: variants, `ALL`, `as_str()`, `index()`, `from_index()`, `from_str()` |
| `src/app.rs` | 309 | `RequestState.method` field type |
| `src/app.rs` | 466-467 | `show_method_popup`, `method_popup_index` state |
| `src/app.rs` | 993-1003 | `build_postman_request()` — method → String for storage |
| `src/app.rs` | 1010-1018 | `open_request()` — String → method from storage |
| `src/app.rs` | 2002-2024 | Popup key handling (j/k/Enter/Esc) |
| `src/app.rs` | 2158-2162 | Method field Enter handler (opens popup) |
| `src/app.rs` | 2464-2487 | `send_request()` — copies method for HTTP call |
| `src/http.rs` | 16-22 | reqwest builder match (method → client.get/post/etc.) |
| `src/http.rs` | 36-39 | Body attachment gating |
| `src/ui/mod.rs` | 264-298 | `render_method_popup()` |
| `src/ui/mod.rs` | 304-312 | `method_color()` |
| `src/ui/layout.rs` | 60-61 | Method area width: `Constraint::Length(10)` |
| `src/storage/models.rs` | 3-36 | Storage `HttpMethod` enum + `From` conversions |

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Enum strategy for custom methods | Wrapper type: `Method` enum with `Standard(HttpMethod)` + `Custom(String)` | Preserves `Copy` on `HttpMethod` for the 7 standard methods. Isolates String allocation to custom methods only. Avoids massive refactor of every `method` usage site. |
| Storage serialization for custom methods | `PostmanRequest.method` is already `String` — no change needed. Remove/simplify `storage::models::HttpMethod` since Postman format is the primary format. | Postman collections already store methods as plain strings. The typed storage enum adds complexity without value. |
| `from_str()` fix | Return `Method::Custom(original)` for unrecognized strings instead of defaulting to GET | Fixes the data loss bug. Any valid HTTP method token is preserved through save/load roundtrips. |
| Body gating for custom methods | Allow body for POST, PUT, PATCH, DELETE, and any custom method. Drop body for GET, HEAD, OPTIONS. | WebDAV methods (PROPFIND, REPORT) require bodies. Users choosing custom methods should get full control. DELETE included since some APIs use DELETE with body. |
| Custom method validation | Auto-uppercase input. Reject empty strings and non-ASCII characters. Max 20 characters. | HTTP methods are case-sensitive per RFC 7230 but convention is uppercase. Length limit prevents layout overflow. |
| Method colors | HEAD=Cyan, OPTIONS=White, Custom=DarkGray | Cyan and White are unused in the current palette. DarkGray signals "non-standard" for custom methods. |
| Method area width | Keep `Constraint::Length(10)` — truncate with ellipsis for methods > 8 chars | OPTIONS (7 chars) fits. Custom methods that overflow get visual truncation. Avoids layout disruption. |
| Custom method popup UX | "Custom..." entry at bottom of popup list → inline text input on selection | Consistent with the existing popup interaction model. Low discoverability cost since custom methods are a power-user feature. |

## Implementation Phases

### Phase A: Add HEAD and OPTIONS (First-Class Variants)

Self-contained, zero-risk change. All modifications follow the existing pattern.

**A.1: Extend `HttpMethod` enum in `src/app.rs`**

- [x] Add `Head` and `Options` variants to `HttpMethod` enum (`src/app.rs:126-133`)
- [x] Update `ALL` array to `[HttpMethod; 7]` with Head and Options (`src/app.rs:136-141`)
- [x] Add `as_str()` arms: `Head => "HEAD"`, `Options => "OPTIONS"` (`src/app.rs:144-151`)
- [x] Add `index()` arms: `Head => 5`, `Options => 6` (`src/app.rs:154-161`)
- [x] Update `from_index()` — already uses `ALL[index % ALL.len()]`, works automatically
- [x] Add `from_str()` arms: `"HEAD" => Head`, `"OPTIONS" => Options` (`src/app.rs:168-175`)

**A.2: Extend `storage::models::HttpMethod` in `src/storage/models.rs`**

- [x] Add `Head` and `Options` variants (`src/storage/models.rs:3-12`)
- [x] Update both `From` implementations for bidirectional conversion (`src/storage/models.rs:14-36`)

**A.3: Update HTTP execution in `src/http.rs`**

- [x] Add match arms: `Head => client.head(url)`, `Options => client.request(reqwest::Method::OPTIONS, url)` (`src/http.rs:16-22`)
- [x] Update body gating (`src/http.rs:36-39`)
  - **Note:** This intentionally adds DELETE to body-sending methods. Current code only sends body for POST/PUT/PATCH. Adding DELETE is a behavioral change — some REST APIs use DELETE with a body (e.g., bulk delete with IDs). HEAD and OPTIONS are excluded.
  ```rust
  // Before:
  if !body.is_empty() && matches!(method, Post | Put | Patch) { ... }
  // After:
  if !body.is_empty() && matches!(method, Post | Put | Patch | Delete) { ... }
  ```

**A.4: Update UI rendering in `src/ui/mod.rs`**

- [x] Add `method_color()` arms: `Head => Color::Cyan`, `Options => Color::White` (`src/ui/mod.rs:304-312`)
- [x] Popup height auto-adjusts via `HttpMethod::ALL.len()` — verify renders correctly with 7 items

**A.5: Verify and test**

- [x] Compile and fix any exhaustive match warnings
- [ ] Manual test: open popup, select HEAD, send request to httpbin.org/get — verify empty body response
- [ ] Manual test: select OPTIONS, send to any endpoint — verify response headers
- [ ] Manual test: save HEAD/OPTIONS requests to collection, reload, verify method persists
- [ ] Verify sidebar displays HEAD/OPTIONS with correct colors

**Commit**: `feat(http): add HEAD and OPTIONS method support`

---

### Phase B: Custom Method Support

Adds the ability to type arbitrary HTTP method strings.

**B.1: Create `Method` wrapper type in `src/app.rs`**

- [x] Define new type alongside `HttpMethod`:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum Method {
      Standard(HttpMethod),
      Custom(String),
  }
  ```

- [x] Implement `Method`:
  - `as_str(&self) -> &str` — delegates to `HttpMethod::as_str()` for Standard, returns `&self.0` for Custom
  - `From<HttpMethod>` for `Method`
  - `Default` → `Method::Standard(HttpMethod::Get)`

- [x] Implement `from_str(s: &str) -> Method`:
  - Try matching against all 7 standard methods first
  - If no match, return `Method::Custom(s.to_uppercase())`
  - This fixes the data loss bug

**B.2: Update `RequestState` to use `Method`**

- [x] Change `RequestState.method` from `HttpMethod` to `Method` (`src/app.rs:309`)
- [x] Update `set_contents()` to accept `Method`
- [x] Update `build_postman_request()` to use `method.as_str()` (already returns `&str`)
- [x] Update `open_request()` to use `Method::from_str()` instead of `HttpMethod::from_str()`
- [x] Update `send_request()` to pass `Method` to `http::send_request()`

**B.3: Update HTTP execution for custom methods**

- [x] Change `send_request()` signature in `src/http.rs` to accept `&Method` instead of `HttpMethod`
- [x] Add custom method handling:

  ```rust
  Method::Standard(m) => match m {
      Get => client.get(url),
      Post => client.post(url),
      // ... existing arms
  },
  Method::Custom(s) => {
      let method = reqwest::Method::from_bytes(s.as_bytes())
          .map_err(|e| format!("Invalid HTTP method '{}': {}", s, e))?;
      client.request(method, url)
  }
  ```

- [x] Update body gating to allow body for custom methods:

  ```rust
  let sends_body = match method {
      Method::Standard(m) => matches!(m, Post | Put | Patch | Delete),
      Method::Custom(_) => true, // Custom methods may need body (WebDAV, etc.)
  };
  ```

**B.4: Add custom method popup UX**

- [x] Add state fields to `App`:
  - `method_custom_input: String` — buffer for custom method text
  - `method_popup_custom_mode: bool` — whether popup is in text input mode
- [x] **Fix popup index navigation for 8 entries**: The popup uses `method_popup_index % HttpMethod::ALL.len()` for wrapping. With 7 standard methods + "Custom...", the total is 8 entries. Change modulo to `HttpMethod::ALL.len() + 1` (or extract as a `popup_item_count()` constant). Index 7 = "Custom..." entry. `from_index()` is only called when Enter is pressed on indices 0-6; index 7 triggers custom input mode instead.
- [x] Add "Custom..." entry to popup rendering below the 7 standard methods
  - Render with DarkGray color and italic style
  - When selected (Enter on index 7), switch to text input mode instead of calling `from_index()`
- [x] Implement text input mode in popup:
  - Render a single-line text input where the "Custom..." entry was
  - Character keys append to `method_custom_input` (auto-uppercased)
  - Backspace removes last character
  - Enter confirms: validate → set `self.request.method = Method::Custom(input)` → close popup
  - Esc cancels: clear input → close popup entirely (consistent with Esc behavior elsewhere)
- [x] Validation on confirm:
  - Reject empty string — no-op (Enter does nothing, input stays open)
  - Reject strings containing whitespace or non-ASCII — no-op
  - Enforce max 20 characters (stop accepting input at limit)
- [x] When re-opening popup with a custom method already selected:
  - Show standard list with "Custom..." highlighted at bottom
  - Pre-fill `method_custom_input` with current custom method string

**B.5: Update method display for custom methods**

- [x] Extend `method_color()` free function to accept `&Method` instead of `HttpMethod`:
  - `Method::Standard(m)` → delegates to existing color logic
  - `Method::Custom(_)` → returns `Color::DarkGray`
- [x] Update method area rendering to handle `Method`:
  - Truncate display to 8 chars + ellipsis if method string exceeds width
- [x] Update sidebar rendering:
  - Extract method from `Method::as_str()` for display
  - Use updated `method_color()` for consistent coloring
- [x] Update `SidebarLine.method` type from `Option<HttpMethod>` to `Option<Method>`

**B.6: Simplify storage layer**

- [x] Remove `storage::models::HttpMethod` enum and its `From` conversions
  - `PostmanRequest.method` is already `String` — the typed storage enum adds no value now that `Method::from_str()` handles all strings
  - If `SavedRequest` still references the storage enum, change its `method` field to `String` and use `#[serde(default)]` for backward compatibility
- [x] Verify save/load roundtrip:
  - Save a custom method request → verify JSON contains the exact method string
  - Reload → verify `Method::Custom("PURGE")` is restored, not `Method::Standard(Get)`

**B.7: Verify and test**

- [x] Compile and fix all type errors from `HttpMethod` → `Method` migration
- [ ] Manual test: open popup, select "Custom...", type "PURGE", confirm — verify display and send
- [ ] Manual test: type lowercase "purge" → verify auto-uppercased to "PURGE"
- [ ] Manual test: try empty string → verify rejection
- [ ] Manual test: save custom method request, reload → verify persistence
- [ ] Manual test: load a Postman collection containing "PROPFIND" method → verify it loads as Custom, not GET
- [ ] Test edge case: very long method name (20 chars) — verify truncation in method area and sidebar

**Commit**: `feat(http): add custom HTTP method support with popup input`

---

## Acceptance Criteria

### Functional Requirements

- [ ] HEAD and OPTIONS appear in the method selector popup and can be selected
- [ ] HEAD requests execute correctly (empty response body, headers present)
- [ ] OPTIONS requests execute correctly (Allow headers visible)
- [ ] Users can type arbitrary method strings via "Custom..." popup entry
- [ ] Custom methods are auto-uppercased on input
- [ ] Custom methods execute via reqwest with correct method token
- [ ] Body is attached for POST, PUT, PATCH, DELETE, and custom methods
- [ ] Body is dropped for GET, HEAD, OPTIONS
- [ ] Invalid custom methods (empty, non-ASCII) are rejected with feedback

### Data Integrity

- [ ] HEAD/OPTIONS/custom methods survive save → reload roundtrip without data loss
- [ ] Loading existing Postman collections with unknown methods preserves the original method string (fixes current data loss bug)
- [ ] `from_str()` no longer silently maps unknown methods to GET

### UI/UX

- [ ] HEAD displays in Cyan, OPTIONS in White, custom methods in DarkGray
- [ ] Method popup correctly sizes for 7 standard methods + "Custom..." entry
- [ ] Custom method input mode shows text field with auto-uppercase
- [ ] Sidebar tree displays all method types with correct colors
- [ ] Method area truncates long custom methods with ellipsis

## Dependencies & Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Phase B `Method` type change touches many files | Medium | Phase A is self-contained and can ship independently. Phase B changes are mechanical (type substitution). |
| Custom popup text input adds UI complexity | Low | Reuse patterns from existing vim text input. Single-line input is simpler than multi-line TextArea. |
| `reqwest::Method::from_bytes()` rejects valid methods | Low | HTTP method tokens are well-defined (RFC 7230). Validation before send catches issues early. |
| Existing collections with non-standard methods break on load | Already happening | Phase A + `from_str()` fix resolves this. Shipping Phase A first reduces the window. |

## References

- **Brainstorm**: `docs/brainstorms/2026-02-15-production-ready-features-brainstorm.md` — Phase 1.4
- **RFC 7231** (HTTP/1.1 Semantics): HEAD and OPTIONS method definitions
- **RFC 7230** (HTTP/1.1 Message Syntax): Method token definition (`token = 1*tchar`)
- **reqwest API**: `Client::head()`, `Client::request()`, `Method::from_bytes()`
- **Postman Collection v2.1**: `request.method` is a plain string field — already supports arbitrary methods
