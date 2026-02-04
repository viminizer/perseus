# Structure

## Directory Layout

```
perseus/
├── Cargo.toml          # Project manifest
├── Cargo.lock          # Dependency lock
├── .gitignore          # Git ignore rules
├── src/
│   └── main.rs         # Binary entry point
└── target/             # Build artifacts (gitignored)
```

## Module Hierarchy

```
crate (perseus)
└── main.rs             # Root binary module
```

No sub-modules or library structure defined.

## File Organization

**Current:** Minimal Rust binary structure
- Single source file
- No feature-based organization
- No layer separation

## Naming Conventions

- **Files:** lowercase with underscores (Rust standard)
- **Binary:** named after package (`perseus`)

## Key Files

| File | Purpose | Lines |
|------|---------|-------|
| src/main.rs | Application entry point | ~3 |
| Cargo.toml | Project configuration | ~7 |

---
*Generated: 2026-02-04*
