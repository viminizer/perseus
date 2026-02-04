# Phase 3: HTTP Integration

## Objective

Wire up reqwest HTTP client to make requests from TUI input, display responses with status, headers, and body, and add loading/error states.

## Execution Context

**Files to read first:**
- `src/app.rs` — Current App struct with RequestState
- `src/ui/mod.rs` — Current render functions
- `Cargo.toml` — reqwest 0.12 already available

**Dependencies available:**
- reqwest 0.12 (with json feature)
- tokio 1 (full features, async runtime)
- anyhow 1 (error handling)

## Context

**Current state:**
- TUI with request/response panels
- RequestState holds: url, method, headers, body
- HttpMethod enum with Get/Post/Put/Patch/Delete
- Response panel is empty placeholder
- No HTTP client wired up

**Target state:**
- Enter key triggers HTTP request
- Loading indicator while request in progress
- Response displays: status code, headers, body
- JSON body gets syntax highlighting
- Errors display clearly in response panel

## Tasks

### Task 1: Add Response State
**Files:** `src/app.rs`

1. Add ResponseState struct:
   ```rust
   pub enum ResponseStatus {
       Empty,
       Loading,
       Success(ResponseData),
       Error(String),
   }

   pub struct ResponseData {
       pub status: u16,
       pub status_text: String,
       pub headers: Vec<(String, String)>,
       pub body: String,
       pub duration_ms: u64,
   }
   ```

2. Add `response: ResponseStatus` field to `App`

3. Initialize as `ResponseStatus::Empty` in `App::new()`

**Verification:** `cargo check` passes

---

### Task 2: Implement HTTP Client
**Files:** `src/http.rs`, `src/main.rs`

1. Create `src/http.rs` module

2. Add `mod http;` to `src/main.rs`

3. Implement async request function:
   ```rust
   use reqwest::Client;
   use crate::app::{HttpMethod, ResponseData};

   pub async fn send_request(
       client: &Client,
       method: HttpMethod,
       url: &str,
       headers: &str,
       body: &str,
   ) -> Result<ResponseData, String> {
       // Parse headers from "Key: Value\n" format
       // Build request with method, headers, body
       // Execute and capture response
       // Return ResponseData or error string
   }
   ```

4. Add `client: reqwest::Client` to `App` struct

5. Initialize client in `App::new()` with `Client::new()`

**Verification:** `cargo check` passes

---

### Task 3: Wire Enter Key to Send Request
**Files:** `src/app.rs`

1. Handle Enter key in `handle_key()`:
   - Only when focused on request panel
   - Only when not already loading
   - Validate URL is not empty

2. Create `send_request()` method on App:
   ```rust
   async fn send_request(&mut self) {
       if self.request.url.is_empty() {
           self.response = ResponseStatus::Error("URL is required".into());
           return;
       }
       self.response = ResponseStatus::Loading;
       // Actual request happens in event loop
   }
   ```

3. Modify event loop to handle async request:
   - Use tokio::spawn for non-blocking request
   - Use channel or flag to signal completion
   - Update response state when done

**Verification:** Enter key triggers loading state (visible in UI)

---

### Task 4: Render Response Panel Content
**Files:** `src/ui/mod.rs`, `src/ui/layout.rs`

1. Add ResponseLayout for response panel areas:
   ```rust
   pub struct ResponseLayout {
       pub status_area: Rect,
       pub headers_area: Rect,
       pub body_area: Rect,
   }
   ```

2. Update `render_response_panel()` to show response:
   - Empty: "Press Enter to send request"
   - Loading: "Loading..." with spinner (optional)
   - Success: Status line, headers list, body text
   - Error: Red error message

3. Format status line: "200 OK (123ms)"

4. Format headers as scrollable list

**Verification:** Response panel shows content based on state

---

### Task 5: Add JSON Syntax Highlighting
**Files:** `src/ui/mod.rs`

1. Detect JSON response (Content-Type header or valid parse)

2. Add simple JSON colorization:
   - Keys: cyan
   - Strings: green
   - Numbers: yellow
   - Booleans/null: magenta
   - Brackets/braces: white

3. Implement `colorize_json()` function that returns styled spans

4. Apply to body text when response is JSON

**Verification:** JSON responses display with colors

---

### Task 6: Handle Request Errors
**Files:** `src/http.rs`, `src/app.rs`

1. Catch all reqwest errors and convert to user-friendly messages:
   - Invalid URL: "Invalid URL: {details}"
   - Connection error: "Connection failed: {host}"
   - Timeout: "Request timed out"
   - DNS error: "Could not resolve: {host}"
   - Other: "Request failed: {error}"

2. Set request timeout (default 30 seconds)

3. Handle parse errors for headers input

4. Display errors in response panel with red styling

**Verification:**
- Invalid URL shows error
- Unreachable host shows connection error
- Malformed headers show parse error

---

## Verification

Run the app and verify:
1. `cargo run` — App starts without errors
2. Type URL: `https://httpbin.org/get`
3. Press Enter — Shows "Loading..."
4. Response appears with status, headers, body
5. JSON body has syntax highlighting
6. Try invalid URL — Shows error message
7. Try unreachable host — Shows connection error
8. 'q' or Esc quits cleanly

## Success Criteria

- [ ] `cargo build` succeeds with no warnings
- [ ] Enter key sends HTTP request
- [ ] Loading state displays during request
- [ ] Response shows status code and duration
- [ ] Response headers display correctly
- [ ] Response body displays with JSON highlighting
- [ ] Errors display with clear messages
- [ ] App remains responsive during request

## Output

```
src/
├── main.rs          # Entry point (add mod http)
├── app.rs           # App with ResponseStatus state
├── http.rs          # HTTP client wrapper
└── ui/
    ├── mod.rs       # Response rendering with JSON highlighting
    ├── layout.rs    # ResponseLayout added
    └── widgets.rs   # (unchanged)
```

---
*Plan created: 2026-02-04*
