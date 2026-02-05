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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::HttpMethod;

    #[test]
    fn test_save_and_load_request() {
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
