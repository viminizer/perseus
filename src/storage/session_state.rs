use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const SESSION_VERSION: u32 = 1;
const SESSION_DIR_NAME: &str = "perseus";
const SESSION_FILE_NAME: &str = "session.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub active_project_id: String,
    pub sidebar_width: u16,
    pub sidebar_visible: bool,
    pub selection_id: Option<String>,
    pub current_request_id: Option<String>,
    pub expanded: Vec<String>,
    pub request_tab: String,
    pub response_tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub version: u32,
    pub sessions: HashMap<String, SessionState>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self {
            version: SESSION_VERSION,
            sessions: HashMap::new(),
        }
    }
}

fn state_dir() -> Option<PathBuf> {
    if let Ok(dir) = env::var("XDG_STATE_HOME") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    let home = env::var("HOME").ok()?;
    if home.trim().is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(".local").join("state"))
}

fn session_dir() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join(SESSION_DIR_NAME))
}

fn session_store_path() -> Option<PathBuf> {
    session_dir().map(|dir| dir.join(SESSION_FILE_NAME))
}

fn ensure_session_dir() -> Result<PathBuf, String> {
    let dir = session_dir().ok_or("Could not resolve session state directory")?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create session directory: {}", e))?;
    Ok(dir)
}

pub fn load_sessions() -> Result<SessionStore, String> {
    let path = match session_store_path() {
        Some(path) if path.exists() => path,
        _ => return Ok(SessionStore::default()),
    };

    let contents =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read session store: {}", e))?;
    let store: SessionStore =
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse session store: {}", e))?;
    if store.version != SESSION_VERSION {
        return Err(format!(
            "Unsupported session store version: {}",
            store.version
        ));
    }
    Ok(store)
}

pub fn save_sessions(store: &SessionStore) -> Result<(), String> {
    let _ = ensure_session_dir()?;
    let path = session_store_path().ok_or("Could not resolve session store path")?;
    let json = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize session store: {}", e))?;
    fs::write(path, json).map_err(|e| format!("Failed to write session store: {}", e))?;
    Ok(())
}

pub fn load_session_for_root(root_key: &str) -> Result<Option<SessionState>, String> {
    if root_key.trim().is_empty() {
        return Ok(None);
    }
    let store = load_sessions()?;
    Ok(store.sessions.get(root_key).cloned())
}

pub fn save_session_for_root(root_key: &str, session: SessionState) -> Result<(), String> {
    if root_key.trim().is_empty() {
        return Err("Session root key is empty".to_string());
    }
    let mut store = load_sessions()?;
    store.version = SESSION_VERSION;
    store.sessions.insert(root_key.to_string(), session);
    save_sessions(&store)
}
