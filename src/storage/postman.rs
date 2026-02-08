use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanCollection {
    pub info: PostmanInfo,
    #[serde(default)]
    pub item: Vec<PostmanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanInfo {
    pub name: String,
    #[serde(rename = "_postman_id")]
    pub postman_id: String,
    pub schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanItem {
    pub name: String,
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item: Vec<PostmanItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<PostmanRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub response: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanRequest {
    pub method: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<PostmanHeader>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
    #[serde(default)]
    pub url: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanHeader {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanBody {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

impl PostmanCollection {
    pub fn new(name: String) -> Self {
        Self {
            info: PostmanInfo {
                name,
                postman_id: new_id(),
                schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                    .to_string(),
            },
            item: Vec::new(),
        }
    }
}

impl PostmanItem {
    pub fn new_folder(name: String) -> Self {
        Self {
            name,
            id: new_id(),
            item: Vec::new(),
            request: None,
            response: Vec::new(),
        }
    }

    pub fn new_request(name: String, request: PostmanRequest) -> Self {
        Self {
            name,
            id: new_id(),
            item: Vec::new(),
            request: Some(request),
            response: Vec::new(),
        }
    }

    pub fn is_request(&self) -> bool {
        self.request.is_some()
    }
}

impl PostmanRequest {
    pub fn new(method: String, url: String, headers: Vec<PostmanHeader>, body: Option<String>) -> Self {
        let body = body.and_then(|raw| {
            if raw.trim().is_empty() {
                None
            } else {
                Some(PostmanBody {
                    mode: "raw".to_string(),
                    raw: Some(raw),
                })
            }
        });

        Self {
            method,
            header: headers,
            body,
            url: Value::String(url),
        }
    }
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
