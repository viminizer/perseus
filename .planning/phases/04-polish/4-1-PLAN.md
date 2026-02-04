# Plan 4-1: Polish — UX Refinement

## Objective

Complete the v1 milestone by adding vim-style insert mode, a status bar with keybinding hints, response scrolling for large bodies, and visual consistency improvements.

## Execution Context

**Files to modify:**
- `src/app.rs` — Add InputMode enum, modify key handling for insert/normal modes
- `src/ui/mod.rs` — Add status bar rendering, response scrolling
- `src/ui/layout.rs` — Adjust layout to include status bar area

**Current keybindings already implemented:**
- `j/k` — field navigation (works)
- `h/l` — method cycling when on Method field (works)
- `Tab` — panel switching (works)
- `Enter` — send request (works)
- `q/Esc` — quit (works)

**What's missing:**
- Insert mode toggle (typing vs navigation)
- Status bar showing current mode and key hints
- Response body scrolling for large responses

## Tasks

### Task 1: Add Input Mode State

Add `InputMode` enum to `app.rs` to distinguish between Normal mode (navigation) and Insert mode (typing).

**Changes to `src/app.rs`:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    #[default]
    Normal,
    Insert,
}
```

Add `input_mode: InputMode` field to `App` struct.

Update `App::new()` to initialize `input_mode: InputMode::Normal`.

**Acceptance:** App compiles with new field.

---

### Task 2: Implement Vim-style Mode Switching

Modify `handle_key` in `app.rs`:

- In Normal mode: `i` enters Insert mode (only when on editable field in Request panel)
- In Insert mode: `Esc` returns to Normal mode
- Character input only works in Insert mode
- Navigation keys (`j/k/h/l/Tab`) only work in Normal mode

**Key mapping:**
| Key | Normal Mode | Insert Mode |
|-----|-------------|-------------|
| `i` | Enter insert mode | Type 'i' |
| `Esc` | Quit | Exit to normal |
| `j/k` | Navigate fields | — |
| `h/l` | Cycle method | — |
| `Tab` | Switch panel | — |
| `q` | Quit | Type 'q' |
| chars | — | Insert into field |
| `Enter` | Send request | Add newline (headers/body) or send (url) |
| `Backspace` | — | Delete char |

**Acceptance:** Can toggle modes with `i`/`Esc`, typing only works in Insert mode.

---

### Task 3: Add Response Scroll State

Add scroll state to `App` for response body scrolling.

**Changes to `src/app.rs`:**

```rust
pub struct App {
    // ... existing fields
    pub response_scroll: u16,
}
```

Initialize to 0 in `App::new()`.

Reset to 0 when new response arrives (in `event_loop` when response status changes).

Add scroll handling in Normal mode when Response panel focused:
- `j` or `Down` — scroll down
- `k` or `Up` — scroll up

**Acceptance:** Scroll state tracked, reset on new response.

---

### Task 4: Implement Response Scrolling in UI

Modify `render_response_content` in `ui/mod.rs` to use scroll offset.

**Changes:**
- Pass scroll offset to response body rendering
- Use `.scroll((scroll_offset, 0))` on the body `Paragraph`
- Calculate max scroll based on body line count vs visible height

**Acceptance:** Large JSON responses can be scrolled with j/k when Response panel focused.

---

### Task 5: Add Status Bar Layout

Modify `layout.rs` to include a status bar area at bottom.

**Changes to `AppLayout`:**

```rust
pub struct AppLayout {
    pub request_area: Rect,
    pub response_area: Rect,
    pub status_bar: Rect,
}
```

Update `AppLayout::new()`:
- Reserve 1 row at bottom for status bar
- Split remaining space 50/50 for request/response

**Acceptance:** Layout includes status bar area.

---

### Task 6: Render Status Bar

Add `render_status_bar` function in `ui/mod.rs`.

**Content:**
- Left: Current mode indicator `[NORMAL]` or `[INSERT]`
- Center: Current field/panel info
- Right: Key hints based on context

**Mode-specific hints:**
- Normal mode: `i:insert  j/k:nav  Tab:panel  Enter:send  q:quit`
- Insert mode: `Esc:normal  Enter:newline`

**Styling:**
- Background: Dark gray
- Mode indicator: Yellow for Insert, Cyan for Normal
- Hints: White text

**Acceptance:** Status bar visible at bottom showing mode and contextual hints.

---

## Verification

After all tasks:

1. Run the app: `cargo run`
2. Verify starts in Normal mode (status bar shows `[NORMAL]`)
3. Press `j/k` to navigate between fields
4. Press `i` to enter Insert mode (status bar shows `[INSERT]`)
5. Type a URL, verify characters appear
6. Press `Esc` to return to Normal mode
7. Press `Tab` to switch to Response panel
8. Press `Enter` to send request (use `https://httpbin.org/json`)
9. Use `j/k` to scroll response body
10. Verify large responses scroll properly

## Success Criteria

- [ ] Input mode toggle works (i/Esc)
- [ ] Status bar shows current mode
- [ ] Status bar shows contextual key hints
- [ ] Response body scrollable with j/k
- [ ] Navigation only works in Normal mode
- [ ] Typing only works in Insert mode
- [ ] App feels keyboard-native and vim-like

## Output

Files modified:
- `src/app.rs` — InputMode, scroll state, mode-aware key handling
- `src/ui/mod.rs` — Status bar rendering, scroll support
- `src/ui/layout.rs` — Status bar area

---
*Created: 2026-02-04*
