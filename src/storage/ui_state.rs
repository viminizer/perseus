use serde::{Deserialize, Serialize};
use std::fs;

use crate::storage::project::{ensure_storage_dir, ui_state_path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiState {
    pub active_project_id: String,
    pub sidebar_width: u16,
}

impl UiState {
    pub fn new(active_project_id: String, sidebar_width: u16) -> Self {
        Self {
            active_project_id,
            sidebar_width,
        }
    }
}

pub fn load_ui_state() -> Result<Option<UiState>, String> {
    let path = match ui_state_path() {
        Some(path) if path.exists() => path,
        _ => return Ok(None),
    };

    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read UI state: {}", e))?;
    let state: UiState =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse UI state: {}", e))?;
    Ok(Some(state))
}

pub fn save_ui_state(state: &UiState) -> Result<(), String> {
    let _ = ensure_storage_dir()?;
    let path = ui_state_path().ok_or("Could not find project root")?;
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize UI state: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write UI state: {}", e))?;
    Ok(())
}
