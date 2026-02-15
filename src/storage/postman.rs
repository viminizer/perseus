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
pub struct PostmanAuthAttribute {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub attr_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmanAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer: Option<Vec<PostmanAuthAttribute>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic: Option<Vec<PostmanAuthAttribute>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apikey: Option<Vec<PostmanAuthAttribute>>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
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
            auth: None,
        }
    }
}

impl PostmanAuth {
    pub fn bearer(token: &str) -> Self {
        Self {
            auth_type: "bearer".to_string(),
            bearer: Some(vec![PostmanAuthAttribute {
                key: "token".to_string(),
                value: Some(serde_json::Value::String(token.to_string())),
                attr_type: Some("string".to_string()),
            }]),
            basic: None,
            apikey: None,
        }
    }

    pub fn basic(username: &str, password: &str) -> Self {
        Self {
            auth_type: "basic".to_string(),
            bearer: None,
            basic: Some(vec![
                PostmanAuthAttribute {
                    key: "username".to_string(),
                    value: Some(serde_json::Value::String(username.to_string())),
                    attr_type: Some("string".to_string()),
                },
                PostmanAuthAttribute {
                    key: "password".to_string(),
                    value: Some(serde_json::Value::String(password.to_string())),
                    attr_type: Some("string".to_string()),
                },
            ]),
            apikey: None,
        }
    }

    pub fn apikey(key: &str, value: &str, location: &str) -> Self {
        Self {
            auth_type: "apikey".to_string(),
            bearer: None,
            basic: None,
            apikey: Some(vec![
                PostmanAuthAttribute {
                    key: "key".to_string(),
                    value: Some(serde_json::Value::String(key.to_string())),
                    attr_type: Some("string".to_string()),
                },
                PostmanAuthAttribute {
                    key: "value".to_string(),
                    value: Some(serde_json::Value::String(value.to_string())),
                    attr_type: Some("string".to_string()),
                },
                PostmanAuthAttribute {
                    key: "in".to_string(),
                    value: Some(serde_json::Value::String(location.to_string())),
                    attr_type: Some("string".to_string()),
                },
            ]),
        }
    }

    pub fn get_bearer_token(&self) -> Option<&str> {
        self.bearer.as_ref()?.iter().find(|a| a.key == "token").and_then(|a| {
            a.value.as_ref().and_then(|v| v.as_str())
        })
    }

    pub fn get_basic_credentials(&self) -> Option<(&str, &str)> {
        let attrs = self.basic.as_ref()?;
        let username = attrs.iter().find(|a| a.key == "username")
            .and_then(|a| a.value.as_ref().and_then(|v| v.as_str()))?;
        let password = attrs.iter().find(|a| a.key == "password")
            .and_then(|a| a.value.as_ref().and_then(|v| v.as_str()))?;
        Some((username, password))
    }

    pub fn get_apikey(&self) -> Option<(&str, &str, &str)> {
        let attrs = self.apikey.as_ref()?;
        let key = attrs.iter().find(|a| a.key == "key")
            .and_then(|a| a.value.as_ref().and_then(|v| v.as_str()))?;
        let value = attrs.iter().find(|a| a.key == "value")
            .and_then(|a| a.value.as_ref().and_then(|v| v.as_str()))?;
        let location = attrs.iter().find(|a| a.key == "in")
            .and_then(|a| a.value.as_ref().and_then(|v| v.as_str()))
            .unwrap_or("header");
        Some((key, value, location))
    }
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
