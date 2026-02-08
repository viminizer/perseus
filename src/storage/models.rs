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
    pub fn new(
        name: String,
        url: String,
        method: HttpMethod,
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
            request.method.into(),
            request.headers_text(),
            request.body_text(),
        )
    }
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
