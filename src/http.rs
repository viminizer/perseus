use std::time::Instant;

use reqwest::Client;

use crate::app::{HttpMethod, Method, ResponseData};

pub async fn send_request(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &str,
    body: &str,
) -> Result<ResponseData, String> {
    let start = Instant::now();

    let mut builder = match method {
        Method::Standard(m) => match m {
            HttpMethod::Get => client.get(url),
            HttpMethod::Post => client.post(url),
            HttpMethod::Put => client.put(url),
            HttpMethod::Patch => client.patch(url),
            HttpMethod::Delete => client.delete(url),
            HttpMethod::Head => client.head(url),
            HttpMethod::Options => client.request(reqwest::Method::OPTIONS, url),
        },
        Method::Custom(s) => {
            let method = reqwest::Method::from_bytes(s.as_bytes())
                .map_err(|e| format!("Invalid HTTP method '{}': {}", s, e))?;
            client.request(method, url)
        }
    };

    for line in headers.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            builder = builder.header(key.trim(), value.trim());
        } else {
            return Err(format!(
                "Invalid header format: '{}' (expected 'Key: Value')",
                line
            ));
        }
    }

    let sends_body = match method {
        Method::Standard(m) => matches!(
            m,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Delete
        ),
        Method::Custom(_) => true,
    };

    if !body.is_empty() && sends_body {
        builder = builder.body(body.to_string());
    }

    let response = builder.send().await.map_err(format_request_error)?;

    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("").to_string();

    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let response_body = response.text().await.map_err(|e| e.to_string())?;

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ResponseData {
        status: status_code,
        status_text,
        headers: response_headers,
        body: response_body,
        duration_ms,
    })
}

fn format_request_error(err: reqwest::Error) -> String {
    if err.is_timeout() {
        return "Request timed out".to_string();
    }
    if err.is_connect() {
        if let Some(url) = err.url() {
            if let Some(host) = url.host_str() {
                return format!("Connection failed: {}", host);
            }
        }
        return "Connection failed".to_string();
    }
    if err.is_builder() {
        let msg = err.to_string();
        if msg.contains("relative URL without a base") {
            return "Invalid URL: missing scheme (try https://)".to_string();
        }
        return format!("Invalid URL: {}", msg);
    }
    if err.is_redirect() {
        return "Too many redirects".to_string();
    }
    if err.is_decode() {
        return "Failed to decode response body".to_string();
    }
    format!("Request failed: {}", err)
}
