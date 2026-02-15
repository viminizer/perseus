use std::fs;

use crate::storage::models::{HttpMethod, SavedRequest};
use crate::storage::postman::{PostmanCollection, PostmanItem, PostmanRequest};
use crate::storage::project::requests_dir;

pub fn load_legacy_requests() -> Result<Vec<SavedRequest>, String> {
    let dir = match requests_dir() {
        Some(path) if path.exists() => path,
        _ => return Ok(Vec::new()),
    };

    let mut requests = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|e| format!("Failed to read legacy requests: {}", e))?;

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

pub fn migrate_legacy(
    collection_name: String,
    project_name: String,
    requests: Vec<SavedRequest>,
) -> PostmanCollection {
    let mut collection = PostmanCollection::new(collection_name);
    let mut project = PostmanItem::new_folder(project_name);

    for request in requests {
        let method = match request.method {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
        .to_string();

        let headers = parse_headers(&request.headers);
        let body = if request.body.trim().is_empty() {
            None
        } else {
            Some(request.body.clone())
        };
        let postman_request = PostmanRequest::new(method, request.url.clone(), headers, body);
        let item = PostmanItem::new_request(request.name.clone(), postman_request);
        project.item.push(item);
    }

    collection.item.push(project);
    collection
}

fn parse_headers(raw: &str) -> Vec<crate::storage::postman::PostmanHeader> {
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut parts = trimmed.splitn(2, ':');
            let key = parts.next()?.trim();
            let value = parts.next().unwrap_or("").trim();
            if key.is_empty() {
                return None;
            }
            Some(crate::storage::postman::PostmanHeader {
                key: key.to_string(),
                value: value.to_string(),
                disabled: None,
            })
        })
        .collect()
}
