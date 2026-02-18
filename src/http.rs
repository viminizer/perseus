use std::path::{Path, PathBuf};
use std::time::Instant;

use futures_util::StreamExt;
use reqwest::Client;

use crate::app::{ApiKeyLocation, HttpMethod, Method, ResponseData};

/// Maximum response body size in bytes (50 MB).
/// Responses larger than this will be truncated to prevent out-of-memory conditions.
const MAX_RESPONSE_BODY_SIZE: usize = 50 * 1024 * 1024;

/// Maximum file size allowed for upload (100 MB).
const MAX_UPLOAD_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Well-known sensitive directories relative to the user's home directory.
/// Paths are checked after canonicalization, so symlink tricks are neutralized.
const SENSITIVE_HOME_PREFIXES: &[&str] = &[
    "/.ssh",
    "/.gnupg",
    "/.gpg",
    "/.aws",
    "/.config/gcloud",
    "/.azure",
    "/.kube",
];

/// Well-known sensitive absolute paths that do not depend on a home directory.
const SENSITIVE_ABSOLUTE_PATHS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/ssl/private",
    "/etc/sudoers",
];

/// Validate and resolve a file path before reading it for upload.
///
/// Returns the canonicalized [`PathBuf`] on success, or a descriptive error.
///
/// Checks performed:
///   1. The path is canonicalized to resolve `..`, `.`, and symlinks. This also
///      verifies the path exists on disk.
///   2. The resolved target must be a regular file (not a directory, device, etc.).
///   3. The file must not reside in a known sensitive directory.
///   4. The file size must not exceed [`MAX_UPLOAD_FILE_SIZE`].
fn validate_file_path(raw_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(raw_path);

    // 1. Canonicalize -- resolves symlinks, `..`, `.` and verifies existence.
    let canonical = path.canonicalize().map_err(|e| {
        format!("Cannot resolve file path '{}': {}", raw_path, e)
    })?;

    // 2. Must be a regular file after symlink resolution.
    let metadata = std::fs::metadata(&canonical).map_err(|e| {
        format!(
            "Cannot read file metadata for '{}': {}",
            canonical.display(),
            e
        )
    })?;

    if !metadata.is_file() {
        return Err(format!(
            "Path '{}' is not a regular file",
            canonical.display()
        ));
    }

    // 3. Reject files inside sensitive directories.
    let canonical_str = canonical.to_string_lossy();

    if let Some(home) = home_dir_prefix() {
        for prefix in SENSITIVE_HOME_PREFIXES {
            let sensitive = format!("{}{}", home, prefix);
            if canonical_str.starts_with(&sensitive) {
                return Err(format!(
                    "Refusing to read file in sensitive directory: {}",
                    canonical.display()
                ));
            }
        }
    }

    for sensitive_path in SENSITIVE_ABSOLUTE_PATHS {
        if canonical_str.starts_with(sensitive_path) {
            return Err(format!(
                "Refusing to read file in sensitive location: {}",
                canonical.display()
            ));
        }
    }

    // 4. Enforce maximum file size.
    let size = metadata.len();
    if size > MAX_UPLOAD_FILE_SIZE {
        return Err(format!(
            "File '{}' is too large ({:.1} MB). Maximum allowed size is {:.0} MB.",
            canonical.display(),
            size as f64 / (1024.0 * 1024.0),
            MAX_UPLOAD_FILE_SIZE as f64 / (1024.0 * 1024.0),
        ));
    }

    Ok(canonical)
}

/// Return the user's home directory path as a [`String`], if available.
fn home_dir_prefix() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
}

pub enum AuthConfig {
    NoAuth,
    Bearer { token: String },
    Basic { username: String, password: String },
    ApiKey { key: String, value: String, location: ApiKeyLocation },
}

pub enum BodyContent {
    None,
    Raw(String),
    Json(String),
    Xml(String),
    FormUrlEncoded(Vec<(String, String)>),
    Multipart(Vec<MultipartPart>),
    Binary(String),
}

pub struct MultipartPart {
    pub key: String,
    pub value: String,
    pub field_type: MultipartPartType,
}

pub enum MultipartPartType {
    Text,
    File,
}

pub async fn send_request(
    client: &Client,
    method: &Method,
    url: &str,
    headers: &str,
    body: BodyContent,
    auth: &AuthConfig,
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

    // Inject authentication
    builder = match auth {
        AuthConfig::NoAuth => builder,
        AuthConfig::Bearer { token } => builder.bearer_auth(token),
        AuthConfig::Basic { username, password } => builder.basic_auth(username, Some(password)),
        AuthConfig::ApiKey { key, value, location } => match location {
            ApiKeyLocation::Header => builder.header(key.as_str(), value.as_str()),
            ApiKeyLocation::QueryParam => builder.query(&[(key.as_str(), value.as_str())]),
        },
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

    let has_manual_content_type = headers
        .lines()
        .any(|line| line.trim().to_lowercase().starts_with("content-type"));

    builder = match body {
        BodyContent::None => builder,
        BodyContent::Raw(text) => {
            if !text.is_empty() && sends_body {
                builder.body(text)
            } else {
                builder
            }
        }
        BodyContent::Json(text) => {
            let mut b = builder;
            if !has_manual_content_type {
                b = b.header("Content-Type", "application/json");
            }
            if !text.is_empty() && sends_body {
                b = b.body(text);
            }
            b
        }
        BodyContent::Xml(text) => {
            let mut b = builder;
            if !has_manual_content_type {
                b = b.header("Content-Type", "application/xml");
            }
            if !text.is_empty() && sends_body {
                b = b.body(text);
            }
            b
        }
        BodyContent::FormUrlEncoded(pairs) => {
            if !pairs.is_empty() && sends_body {
                builder.form(&pairs)
            } else {
                builder
            }
        }
        BodyContent::Multipart(parts) => {
            if !parts.is_empty() && sends_body {
                let mut form = reqwest::multipart::Form::new();
                for part in parts {
                    match part.field_type {
                        MultipartPartType::Text => {
                            form = form.text(part.key, part.value);
                        }
                        MultipartPartType::File => {
                            let validated_path = validate_file_path(&part.value)?;
                            let file_bytes =
                                tokio::fs::read(&validated_path).await.map_err(|e| {
                                    format!(
                                        "Failed to read file '{}': {}",
                                        validated_path.display(),
                                        e
                                    )
                                })?;
                            let file_name = validated_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("file")
                                .to_string();
                            let file_part =
                                reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
                            form = form.part(part.key, file_part);
                        }
                    }
                }
                builder.multipart(form)
            } else {
                builder
            }
        }
        BodyContent::Binary(path) => {
            if !path.is_empty() && sends_body {
                let validated_path = validate_file_path(&path)?;
                let bytes = tokio::fs::read(&validated_path).await.map_err(|e| {
                    format!("Failed to read file '{}': {}", validated_path.display(), e)
                })?;
                let mut b = builder;
                if !has_manual_content_type {
                    b = b.header("Content-Type", "application/octet-stream");
                }
                b.body(bytes)
            } else {
                builder
            }
        }
    };

    let response = builder.send().await.map_err(format_request_error)?;

    let status = response.status();
    let status_code = status.as_u16();
    let status_text = status.canonical_reason().unwrap_or("").to_string();

    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Read the response body in chunks, enforcing a size limit to prevent OOM
    // from malicious or misconfigured servers returning unbounded data.
    let mut body_bytes: Vec<u8> = Vec::new();
    let mut truncated = false;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| e.to_string())?;
        let remaining = MAX_RESPONSE_BODY_SIZE.saturating_sub(body_bytes.len());
        if remaining == 0 {
            truncated = true;
            break;
        }
        if chunk.len() > remaining {
            body_bytes.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body_bytes.extend_from_slice(&chunk);
    }

    let body_size_bytes = body_bytes.len();
    let mut response_body = String::from_utf8_lossy(&body_bytes).into_owned();

    if truncated {
        response_body.push_str(&format!(
            "\n\n--- Response truncated at {} bytes (limit: {} bytes) ---",
            body_size_bytes,
            MAX_RESPONSE_BODY_SIZE,
        ));
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(ResponseData {
        status: status_code,
        status_text,
        headers: response_headers,
        body: response_body,
        body_size_bytes,
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
