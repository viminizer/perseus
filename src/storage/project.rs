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

pub fn project_root_key() -> Option<String> {
    let root = find_project_root()?;
    let canonical = fs::canonicalize(root).ok()?;
    Some(canonical.to_string_lossy().to_string())
}

pub fn storage_dir() -> Option<PathBuf> {
    find_project_root().map(|root| root.join(".perseus"))
}

pub fn ensure_storage_dir() -> Result<PathBuf, String> {
    let dir = storage_dir().ok_or(
        "Could not find project root. Run from a directory with .git, Cargo.toml, package.json, or create a .perseus folder.",
    )?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create storage directory: {}", e))?;
    Ok(dir)
}

pub fn collection_path() -> Option<PathBuf> {
    storage_dir().map(|root| root.join("collection.json"))
}

pub fn requests_dir() -> Option<PathBuf> {
    storage_dir().map(|root| root.join("requests"))
}

pub fn ui_state_path() -> Option<PathBuf> {
    storage_dir().map(|root| root.join("ui.json"))
}
