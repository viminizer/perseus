# Phase 1: Foundation

## Objective

Set up project with ratatui + tokio stack and basic application loop that starts and exits cleanly.

## Context

**Current state:**
- Cargo.toml has invalid edition "2024" (must fix to "2021")
- src/main.rs has placeholder "Hello, world!"
- No dependencies configured

**Target state:**
- Valid Cargo.toml with all dependencies
- Application struct with terminal setup/teardown
- Event loop that handles 'q' to quit
- Panic handler for clean terminal restoration

## Tasks

### Task 1: Configure Cargo.toml
**Files:** `Cargo.toml`

1. Change edition from "2024" to "2021"
2. Add dependencies:
   ```toml
   [dependencies]
   ratatui = "0.29"
   crossterm = "0.28"
   tokio = { version = "1", features = ["full"] }
   reqwest = { version = "0.12", features = ["json"] }
   anyhow = "1"
   ```
3. Run `cargo check` to verify dependencies resolve

**Verification:** `cargo check` succeeds

---

### Task 2: Create Application Structure
**Files:** `src/main.rs`, `src/app.rs`

1. Create `src/app.rs` with:
   - `App` struct holding application state
   - `App::new()` constructor
   - `App::run(&mut self)` method (empty for now)
   - `running: bool` field

2. Update `src/main.rs`:
   - Add `mod app;`
   - Create tokio async main with `#[tokio::main]`
   - Instantiate App and call run

**Verification:** `cargo build` succeeds

---

### Task 3: Terminal Setup and Event Loop
**Files:** `src/app.rs`

1. Add terminal setup in `App::run()`:
   - Enable raw mode (`crossterm::terminal::enable_raw_mode`)
   - Enter alternate screen
   - Create Terminal with CrosstermBackend

2. Add panic hook for clean terminal restoration:
   - Store original hook
   - Install custom hook that restores terminal then calls original

3. Add terminal teardown:
   - Disable raw mode
   - Leave alternate screen

4. Implement basic event loop:
   - Poll for crossterm events
   - Handle KeyCode::Char('q') to quit
   - Render empty frame (just clear)

**Verification:**
- App starts, shows blank terminal
- Press 'q' to quit cleanly
- Terminal restored to normal state

---

## Success Criteria

- [ ] `cargo build` succeeds with no warnings
- [ ] App starts and displays blank screen
- [ ] Press 'q' exits cleanly
- [ ] Terminal restored after exit (including panic)
- [ ] Code compiles without clippy warnings

## Output

```
src/
├── main.rs    # Entry point with tokio runtime
└── app.rs     # Application struct and event loop
```

---
*Plan created: 2026-02-04*
