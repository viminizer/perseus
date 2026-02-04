# Phase 2: TUI Layout

## Objective

Create panel-based UI layout with request/response areas, input widgets, and keyboard navigation between panels and fields.

## Execution Context

**Files to read first:**
- `src/app.rs` — Current app structure and event loop
- `Cargo.toml` — Dependencies available

**Dependencies available:**
- ratatui 0.29 (widgets, layout, Frame, Style)
- crossterm 0.28 (keyboard events)

## Context

**Current state:**
- Basic app with terminal setup/teardown
- Event loop polling for key events
- Only handles 'q' to quit
- Renders empty/clear frame

**Target state:**
- Split layout: request panel (left/top), response panel (right/bottom)
- Request panel contains: URL field, method selector, headers area, body area
- Response panel shows placeholder for response display
- Tab/arrow keys navigate between panels and fields
- Visual focus indicators show current selection
- Esc or 'q' quits application

## Tasks

### Task 1: Define UI Module Structure
**Files:** `src/ui.rs`, `src/ui/mod.rs`

1. Create `src/ui/` directory with module structure:
   - `mod.rs` — Exports and main render function
   - `layout.rs` — Layout definitions
   - `widgets.rs` — Custom widget wrappers

2. Add `mod ui;` to `src/main.rs`

3. Define basic `render(frame: &mut Frame, app: &App)` function in `ui/mod.rs`

**Verification:** `cargo check` passes

---

### Task 2: Implement Panel Layout
**Files:** `src/ui/layout.rs`, `src/ui/mod.rs`

1. Create horizontal split layout (request left 50%, response right 50%)

2. Define layout structure:
   ```rust
   pub struct AppLayout {
       pub request_area: Rect,
       pub response_area: Rect,
   }

   impl AppLayout {
       pub fn new(area: Rect) -> Self { ... }
   }
   ```

3. Use ratatui `Layout::horizontal()` with `Constraint::Percentage(50)`

4. Render bordered blocks with titles:
   - Left: "Request"
   - Right: "Response"

5. Update `ui::render()` to use layout and draw bordered panels

6. Call `ui::render()` from `App::event_loop()` instead of Clear widget

**Verification:** App shows two bordered panels side by side

---

### Task 3: Add Request Panel Fields
**Files:** `src/ui/widgets.rs`, `src/ui/mod.rs`, `src/app.rs`

1. Add request state to `App`:
   ```rust
   pub struct RequestState {
       pub url: String,
       pub method: HttpMethod,
       pub headers: String,
       pub body: String,
   }

   pub enum HttpMethod {
       Get, Post, Put, Patch, Delete,
   }
   ```

2. Split request area into sub-regions:
   - Method selector (1 line)
   - URL input (1 line)
   - Headers area (30%)
   - Body area (remaining)

3. Render each field with ratatui widgets:
   - Method: styled text showing current method
   - URL: Paragraph with input text
   - Headers: Paragraph with "Key: Value" lines
   - Body: Paragraph with body text

4. Add visual separators between fields

**Verification:** Request panel shows all input fields with placeholder content

---

### Task 4: Implement Focus Management
**Files:** `src/app.rs`, `src/ui/mod.rs`

1. Define focus tracking in App:
   ```rust
   pub enum Panel {
       Request,
       Response,
   }

   pub enum RequestField {
       Method,
       Url,
       Headers,
       Body,
   }

   pub struct FocusState {
       pub panel: Panel,
       pub request_field: RequestField,
   }
   ```

2. Add `focus: FocusState` to `App`

3. Update render to highlight focused element:
   - Focused panel: brighter border color
   - Focused field: inverted colors or distinct highlight

4. Use ratatui `Style::default().fg(Color::Yellow)` for focus indicators

**Verification:** App shows visual distinction for focused panel/field

---

### Task 5: Implement Keyboard Navigation
**Files:** `src/app.rs`

1. Add key handling for navigation:
   - `Tab` — Cycle between panels (Request <-> Response)
   - `Up/Down` or `j/k` — Move between fields within request panel
   - `q` or `Esc` — Quit application

2. Update event loop to dispatch key events:
   ```rust
   fn handle_key(&mut self, key: KeyEvent) {
       match key.code {
           KeyCode::Tab => self.cycle_panel(),
           KeyCode::Up | KeyCode::Char('k') => self.prev_field(),
           KeyCode::Down | KeyCode::Char('j') => self.next_field(),
           KeyCode::Char('q') | KeyCode::Esc => self.running = false,
           _ => {}
       }
   }
   ```

3. Implement navigation methods:
   - `cycle_panel()` — Toggle between Request and Response
   - `next_field()` / `prev_field()` — Cycle through RequestField variants

**Verification:**
- Tab switches panel focus
- Arrow keys move between request fields
- 'q' or Esc quits

---

### Task 6: Add Text Input Handling
**Files:** `src/app.rs`

1. Handle character input when focused on editable fields:
   - Method field: Left/Right arrows or `h/l` to cycle methods
   - URL field: Accept character input, Backspace to delete
   - Headers field: Accept character input with newlines
   - Body field: Accept character input with newlines

2. Add input mode detection:
   ```rust
   fn handle_key(&mut self, key: KeyEvent) {
       // Navigation keys work in all modes
       // Character input goes to focused field
       match key.code {
           KeyCode::Char(c) if self.is_editable_field() => {
               self.insert_char(c);
           }
           KeyCode::Backspace if self.is_editable_field() => {
               self.delete_char();
           }
           // ... navigation keys
       }
   }
   ```

3. Implement cursor position tracking for URL field (optional for headers/body in this phase)

**Verification:**
- Can type in URL field
- Can cycle through HTTP methods
- Can enter text in headers and body
- Backspace deletes characters

---

## Verification

Run the app and verify:
1. `cargo run` — App starts without errors
2. Two panels visible with borders and titles
3. Request panel shows method, URL, headers, body fields
4. Tab key switches focus between panels
5. Arrow keys navigate between request fields
6. Visual indicators show focused element
7. Can type text in URL field
8. Can cycle HTTP methods
9. 'q' or Esc quits cleanly

## Success Criteria

- [ ] `cargo build` succeeds with no warnings
- [ ] Two-panel layout renders correctly
- [ ] All request input fields visible
- [ ] Keyboard navigation works between panels and fields
- [ ] Focus indicators clearly show current selection
- [ ] Text input works for URL field
- [ ] Method selector cycles through HTTP methods
- [ ] App exits cleanly on 'q' or Esc

## Output

```
src/
├── main.rs          # Entry point (add mod ui)
├── app.rs           # App struct with focus state and input handling
└── ui/
    ├── mod.rs       # Main render function
    ├── layout.rs    # Layout calculations
    └── widgets.rs   # Field widgets
```

---
*Plan created: 2026-02-04*
