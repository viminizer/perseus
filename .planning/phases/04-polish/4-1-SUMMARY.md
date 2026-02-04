# Summary: Plan 4-1 — UX Refinement

## Completed Tasks

### Task 1: Add Input Mode State
Added `InputMode` enum with `Normal` and `Insert` variants to `app.rs`. Added `input_mode` field to `App` struct initialized to `Normal`.

**Commit:** `a385523` feat(4-1): add InputMode enum for vim-style mode switching

---

### Task 2: Implement Vim-style Mode Switching
Refactored `handle_key` into `handle_normal_mode` and `handle_insert_mode`. In Normal mode: `i` enters Insert mode on editable fields, navigation keys work. In Insert mode: `Esc` returns to Normal, characters are typed, `Enter` adds newline (or exits Insert on URL field).

**Commit:** `f067460` feat(4-1): implement vim-style mode switching with i/Esc

---

### Task 3: Add Response Scroll State
Added `response_scroll: u16` field to `App`. Scroll resets to 0 when new response arrives. In Normal mode with Response panel focused, `j/k` control scroll position.

**Commit:** `f3c31a2` feat(4-1): add response scroll state with j/k navigation

---

### Task 4: Implement Response Scrolling in UI
Updated `render_response_content` to accept scroll offset and apply `.scroll((scroll_offset, 0))` to the body `Paragraph` widget.

**Commit:** `36c6af1` feat(4-1): implement response body scrolling in UI

---

### Task 5: Add Status Bar Layout
Modified `AppLayout` to include `status_bar: Rect`. Layout now reserves 1 row at bottom for status bar before splitting main area 50/50 for request/response.

**Commit:** `053f6e1` feat(4-1): add status bar area to layout

---

### Task 6: Render Status Bar
Added `render_status_bar` function displaying:
- Left: Mode indicator (`[NORMAL]` cyan, `[INSERT]` yellow)
- Center: Current panel and field info
- Right: Contextual key hints

**Commit:** `6eef1a7` feat(4-1): add status bar with mode indicator and key hints

---

## Files Modified

| File | Changes |
|------|---------|
| `src/app.rs` | InputMode enum, response_scroll, mode-aware key handling |
| `src/ui/mod.rs` | Status bar rendering, scroll support |
| `src/ui/layout.rs` | Status bar area |

## Deviations

None.

## Issues Discovered

None.

---
*Completed: 2026-02-04*
