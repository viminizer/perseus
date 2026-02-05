# Plan 5-1: UI Improvements

## Objective

Enhance the visual experience with better error display, loading animation, help overlay, and refined proportions.

## Execution Context

**Files to modify:**
- `src/app.rs` — Add help overlay state, loading tick counter
- `src/ui/mod.rs` — Improved error rendering, loading animation, help overlay
- `src/ui/layout.rs` — Adjust panel proportions

**Current state:**
- Basic error display (red text)
- Static "Loading..." text
- No help overlay
- 50/50 panel split
- Status bar with basic key hints

## Tasks

### Task 1: Add Loading Animation State

Add a tick counter to App for animating the loading indicator.

**Changes to `src/app.rs`:**

```rust
pub struct App {
    // ... existing fields
    pub loading_tick: u8,
}
```

Initialize to 0 in `App::new()`.

In event_loop, increment tick on each frame when response is Loading:
```rust
if matches!(self.response, ResponseStatus::Loading) {
    self.loading_tick = self.loading_tick.wrapping_add(1);
}
```

**Acceptance:** Tick counter increments during loading.

---

### Task 2: Implement Loading Spinner

Replace static "Loading..." with animated spinner in response panel.

**Changes to `src/ui/mod.rs`:**

In `render_response_panel` for `ResponseStatus::Loading`:

```rust
ResponseStatus::Loading => {
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner_frames[(app.loading_tick as usize / 4) % spinner_frames.len()];
    let loading = Paragraph::new(format!("{} Sending request...", frame))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(loading, inner_area);
}
```

**Acceptance:** Spinner animates during request.

---

### Task 3: Improve Error Display

Enhance error messages with styled box and icon.

**Changes to `src/ui/mod.rs`:**

In `render_response_panel` for `ResponseStatus::Error`:

```rust
ResponseStatus::Error(msg) => {
    let error_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title("Error");
    let error_inner = error_block.inner(inner_area);
    frame.render_widget(error_block, inner_area);

    let error_lines = vec![
        Line::from(vec![
            Span::styled("✗ ", Style::default().fg(Color::Red)),
            Span::raw(msg.as_str()),
        ]),
    ];
    let error_text = Paragraph::new(error_lines)
        .style(Style::default().fg(Color::Red))
        .wrap(ratatui::widgets::Wrap { trim: true });
    frame.render_widget(error_text, error_inner);
}
```

Add `Wrap` to imports.

**Acceptance:** Errors display in red box with icon, text wraps.

---

### Task 4: Add Help Overlay State

Add state to track whether help overlay is visible.

**Changes to `src/app.rs`:**

```rust
pub struct App {
    // ... existing fields
    pub show_help: bool,
}
```

Initialize to `false` in `App::new()`.

In `handle_normal_mode`, add handler for `?`:
```rust
KeyCode::Char('?') => {
    self.show_help = !self.show_help;
}
```

**Acceptance:** `?` toggles help state.

---

### Task 5: Render Help Overlay

Create centered help overlay showing all keybindings.

**Changes to `src/ui/mod.rs`:**

Add new function:

```rust
fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();

    // Calculate centered area (60% width, 70% height)
    let width = (area.width as f32 * 0.6) as u16;
    let height = (area.height as f32 * 0.7) as u16;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let help_area = Rect::new(x, y, width, height);

    // Clear background
    frame.render_widget(ratatui::widgets::Clear, help_area);

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Help (press ? to close) ");

    let help_inner = help_block.inner(help_area);
    frame.render_widget(help_block, help_area);

    let help_text = vec![
        Line::from(Span::styled("Navigation", Style::default().fg(Color::Yellow))),
        Line::from("  j/k or ↑/↓  Move between fields"),
        Line::from("  h/l or ←/→  Cycle HTTP method"),
        Line::from("  Tab         Switch panel"),
        Line::from(""),
        Line::from(Span::styled("Modes", Style::default().fg(Color::Yellow))),
        Line::from("  i           Enter insert mode"),
        Line::from("  Esc         Return to normal mode"),
        Line::from(""),
        Line::from(Span::styled("Actions", Style::default().fg(Color::Yellow))),
        Line::from("  Enter       Send request"),
        Line::from("  ?           Toggle this help"),
        Line::from("  q           Quit"),
    ];

    let help_paragraph = Paragraph::new(help_text);
    frame.render_widget(help_paragraph, help_inner);
}
```

In `render()`, call after other panels if help is visible:
```rust
if app.show_help {
    render_help_overlay(frame);
}
```

Add `Clear` and `Rect` to imports.

**Acceptance:** `?` shows centered help overlay with all keybindings.

---

### Task 6: Refine Panel Proportions

Adjust layout for better visual balance.

**Changes to `src/ui/layout.rs`:**

Update `AppLayout::new()`:
- Change from 50/50 to 45/55 split (request panel slightly narrower)

```rust
let horizontal = Layout::horizontal([
    Constraint::Percentage(45),
    Constraint::Percentage(55),
])
.split(main_area);
```

Update `RequestLayout::new()`:
- Give more space to body area

```rust
let chunks = Layout::vertical([
    Constraint::Length(3),  // Method
    Constraint::Length(3),  // URL
    Constraint::Length(5),  // Headers (was Percentage(30))
    Constraint::Min(5),     // Body
])
.split(area);
```

**Acceptance:** Layout feels more balanced, body has more room.

---

## Verification

After all tasks:

1. Run the app: `cargo run`
2. Press `?` to open help overlay
3. Verify help shows all keybindings, press `?` to close
4. Enter a URL and press Enter
5. Verify spinner animates during loading
6. Enter invalid URL, verify error shows in red box with icon
7. Verify panel proportions look balanced

## Success Criteria

- [ ] Loading spinner animates
- [ ] Errors display in styled box with icon
- [ ] Help overlay shows with `?` key
- [ ] Help overlay closes with `?` key
- [ ] Panel proportions refined
- [ ] All existing functionality preserved

## Output

Files modified:
- `src/app.rs` — loading_tick, show_help state
- `src/ui/mod.rs` — spinner, error box, help overlay
- `src/ui/layout.rs` — refined proportions

---
*Created: 2026-02-05*
