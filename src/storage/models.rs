use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub url: String,
    pub method: String,
    pub headers: String,
    pub body: String,
}

impl SavedRequest {
    pub fn new(
        name: String,
        url: String,
        method: String,
        headers: String,
        body: String,
    ) -> Self {
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
            request.url_text(),
            request.method.as_str().to_string(),
            request.headers_text(),
            request.body_text(),
        )
    }
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
