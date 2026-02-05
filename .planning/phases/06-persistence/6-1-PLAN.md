# Plan 6-1: Persistence

## Objective

Implement project-local file-based storage for requests. Find project root, load existing requests on startup, and persist new requests to `.perseus/requests/` in the project folder.

## Execution Context

**New files:**
- `src/storage/mod.rs` — Public API and re-exports
- `src/storage/models.rs` — SavedRequest data model
- `src/storage/io.rs` — File operations (read/write/list/delete)
- `src/storage/project.rs` — Project root detection

**Files to modify:**
- `Cargo.toml` — Add serde dependencies
- `src/main.rs` — Add storage module declaration

**Storage location:** `<project_root>/.perseus/requests/`

**Project root detection:** Walk up from current directory, find first directory containing `.git`, `Cargo.toml`, `package.json`, or `.perseus` folder.

## Tasks

### Task 1: Add Dependencies

Add serde to Cargo.toml for serialization.

**Changes to `Cargo.toml`:**

```toml
[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
anyhow = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**Acceptance:** `cargo check` passes with new dependencies.

---

### Task 2: Create Storage Module Structure

Create the storage module with mod.rs as the public API.

**Create `src/storage/mod.rs`:**

```rust
mod io;
mod models;
mod project;

pub use io::{delete_request, list_requests, load_request, save_request};
pub use models::SavedRequest;
pub use project::{ensure_storage_dir, find_project_root, storage_dir};
```

**Changes to `src/main.rs`:**

Add module declaration after other mod statements:
```rust
mod storage;
```

**Acceptance:** `cargo check` passes.

---

### Task 3: Implement Project Root Detection

Create logic to find project root by walking up directories.

**Create `src/storage/project.rs`:**

```rust
use std::env;
use std::fs;
use std::path::PathBuf;

const PROJECT_MARKERS: &[&str] = &[".git", "Cargo.toml", "package.json", ".perseus"];

pub fn find_project_root() -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    let mut dir = current.as_path();

    loop {
        for marker in PROJECT_MARKERS {
            let marker_path = dir.join(marker);
            if marker_path.exists() {
                return Some(dir.to_path_buf());
            }
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

pub fn storage_dir() -> Option<PathBuf> {
    find_project_root().map(|root| root.join(".perseus").join("requests"))
}

pub fn ensure_storage_dir() -> Result<PathBuf, String> {
    let dir = storage_dir().ok_or("Could not find project root. Run from a directory with .git, Cargo.toml, package.json, or create a .perseus folder.")?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create storage directory: {}", e))?;
    Ok(dir)
}
```

**Acceptance:** `find_project_root()` returns correct path when run from project subdirectory.

---

### Task 4: Create SavedRequest Model

Define the data model for persisted requests with serde serialization.

**Create `src/storage/models.rs`:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl From<crate::app::HttpMethod> for HttpMethod {
    fn from(method: crate::app::HttpMethod) -> Self {
        match method {
            crate::app::HttpMethod::Get => HttpMethod::Get,
            crate::app::HttpMethod::Post => HttpMethod::Post,
            crate::app::HttpMethod::Put => HttpMethod::Put,
            crate::app::HttpMethod::Patch => HttpMethod::Patch,
            crate::app::HttpMethod::Delete => HttpMethod::Delete,
        }
    }
}

impl From<HttpMethod> for crate::app::HttpMethod {
    fn from(method: HttpMethod) -> Self {
        match method {
            HttpMethod::Get => crate::app::HttpMethod::Get,
            HttpMethod::Post => crate::app::HttpMethod::Post,
            HttpMethod::Put => crate::app::HttpMethod::Put,
            HttpMethod::Patch => crate::app::HttpMethod::Patch,
            HttpMethod::Delete => crate::app::HttpMethod::Delete,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: HttpMethod,
    pub headers: String,
    pub body: String,
}

impl SavedRequest {
    pub fn new(name: String, url: String, method: HttpMethod, headers: String, body: String) -> Self {
        let id = generate_id();
        Self {
            id,
            name,
            url,
            method,
            headers,
            body,
        }
    }

    pub fn from_request_state(name: String, request: &crate::app::RequestState) -> Self {
        Self::new(
            name,
            request.url.clone(),
            request.method.into(),
            request.headers.clone(),
            request.body.clone(),
        )
    }
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("req_{}", timestamp)
}
```

**Acceptance:** Model compiles, serializes to JSON correctly.

---

### Task 5: Implement File I/O Operations

Create the I/O module with CRUD operations for saved requests.

**Create `src/storage/io.rs`:**

```rust
use std::fs;

use crate::storage::models::SavedRequest;
use crate::storage::project::{ensure_storage_dir, storage_dir};

pub fn save_request(request: &SavedRequest) -> Result<(), String> {
    let dir = ensure_storage_dir()?;
    let path = dir.join(format!("{}.json", request.id));
    let json = serde_json::to_string_pretty(request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write request file: {}", e))?;
    Ok(())
}

pub fn load_request(id: &str) -> Result<SavedRequest, String> {
    let dir = storage_dir().ok_or("Could not find project root")?;
    let path = dir.join(format!("{}.json", id));
    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read request file: {}", e))?;
    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse request file: {}", e))
}

pub fn list_requests() -> Result<Vec<SavedRequest>, String> {
    let dir = match storage_dir() {
        Some(d) if d.exists() => d,
        _ => return Ok(Vec::new()),
    };

    let mut requests = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|e| format!("Failed to read storage directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(request) = serde_json::from_str::<SavedRequest>(&contents) {
                    requests.push(request);
                }
            }
        }
    }

    Ok(requests)
}

pub fn delete_request(id: &str) -> Result<(), String> {
    let dir = storage_dir().ok_or("Could not find project root")?;
    let path = dir.join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to delete request file: {}", e))?;
    }
    Ok(())
}
```

**Acceptance:** All CRUD operations work, files stored in `<project>/.perseus/requests/`.

---

### Task 6: Add Integration Test

Create a basic test to verify storage operations work end-to-end.

**Add test at bottom of `src/storage/io.rs`:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::HttpMethod;

    #[test]
    fn test_save_and_load_request() {
        // This test runs from project root, so storage_dir() will work
        let request = SavedRequest::new(
            "Test Request".to_string(),
            "https://api.example.com/test".to_string(),
            HttpMethod::Post,
            "Content-Type: application/json".to_string(),
            r#"{"key": "value"}"#.to_string(),
        );

        let id = request.id.clone();

        // Save
        save_request(&request).expect("Failed to save request");

        // Load
        let loaded = load_request(&id).expect("Failed to load request");
        assert_eq!(loaded.name, "Test Request");
        assert_eq!(loaded.url, "https://api.example.com/test");
        assert_eq!(loaded.method, HttpMethod::Post);

        // List
        let all = list_requests().expect("Failed to list requests");
        assert!(all.iter().any(|r| r.id == id));

        // Delete
        delete_request(&id).expect("Failed to delete request");
        assert!(load_request(&id).is_err());
    }
}
```

**Acceptance:** `cargo test` passes for storage module.

---

## Verification

After all tasks:

1. Run `cargo check` — no errors
2. Run `cargo test` — storage tests pass
3. Verify project detection:
   - `cd` into a subdirectory
   - Run perseus, save a request
   - Check `<project_root>/.perseus/requests/` contains the file

## Success Criteria

- [ ] serde dependencies added
- [ ] storage module created with mod.rs, models.rs, io.rs, project.rs
- [ ] find_project_root() walks up to find .git/Cargo.toml/package.json/.perseus
- [ ] SavedRequest model with JSON serialization
- [ ] save_request() writes JSON to <project>/.perseus/requests/{id}.json
- [ ] load_request() reads and deserializes request
- [ ] list_requests() returns all saved requests from project
- [ ] delete_request() removes request file
- [ ] Integration test passes

## Output

Files created:
- `src/storage/mod.rs` — Public API
- `src/storage/models.rs` — SavedRequest, HttpMethod (serializable)
- `src/storage/io.rs` — File operations with tests
- `src/storage/project.rs` — Project root detection

Files modified:
- `Cargo.toml` — serde, serde_json dependencies
- `src/main.rs` — mod storage declaration

Storage format (example `<project>/.perseus/requests/req_1738764800000.json`):
```json
{
  "id": "req_1738764800000",
  "name": "Get Users",
  "url": "https://api.example.com/users",
  "method": "GET",
  "headers": "Authorization: Bearer token",
  "body": ""
}
```

---
*Created: 2026-02-05*
