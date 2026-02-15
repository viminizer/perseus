use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::storage::find_project_root;

// ---------------------------------------------------------------------------
// Top-level Config — all fields have defaults, unknown keys silently ignored.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub http: HttpConfig,
    pub proxy: ProxyConfig,
    pub ssl: SslConfig,
    pub ui: UiConfig,
    pub editor: EditorConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HttpConfig {
    /// Timeout in seconds. 0 = no timeout.
    pub timeout: u64,
    pub follow_redirects: bool,
    pub max_redirects: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    pub url: Option<String>,
    pub no_proxy: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SslConfig {
    pub verify: bool,
    pub ca_cert: Option<PathBuf>,
    pub client_cert: Option<PathBuf>,
    pub client_key: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub sidebar_width: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    pub tab_size: u8,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

impl Default for Config {
    fn default() -> Self {
        Self {
            http: HttpConfig::default(),
            proxy: ProxyConfig::default(),
            ssl: SslConfig::default(),
            ui: UiConfig::default(),
            editor: EditorConfig::default(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: 30,
            follow_redirects: true,
            max_redirects: 10,
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            url: None,
            no_proxy: None,
        }
    }
}

impl Default for SslConfig {
    fn default() -> Self {
        Self {
            verify: true,
            ca_cert: None,
            client_cert: None,
            client_key: None,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { sidebar_width: 32 }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self { tab_size: 2 }
    }
}

// ---------------------------------------------------------------------------
// Overlay config — partial deserialization for field-level merging.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlayConfig {
    http: OverlayHttpConfig,
    proxy: OverlayProxyConfig,
    ssl: OverlaySslConfig,
    ui: OverlayUiConfig,
    editor: OverlayEditorConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlayHttpConfig {
    timeout: Option<u64>,
    follow_redirects: Option<bool>,
    max_redirects: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlayProxyConfig {
    url: Option<String>,
    no_proxy: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlaySslConfig {
    verify: Option<bool>,
    ca_cert: Option<PathBuf>,
    client_cert: Option<PathBuf>,
    client_key: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlayUiConfig {
    sidebar_width: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct OverlayEditorConfig {
    tab_size: Option<u8>,
}

impl Config {
    /// Apply overlay values over self. Only `Some` fields are overridden.
    fn merge(mut self, overlay: OverlayConfig) -> Self {
        if let Some(v) = overlay.http.timeout {
            self.http.timeout = v;
        }
        if let Some(v) = overlay.http.follow_redirects {
            self.http.follow_redirects = v;
        }
        if let Some(v) = overlay.http.max_redirects {
            self.http.max_redirects = v;
        }
        if let Some(v) = overlay.proxy.url {
            self.proxy.url = Some(v);
        }
        if let Some(v) = overlay.proxy.no_proxy {
            self.proxy.no_proxy = Some(v);
        }
        if let Some(v) = overlay.ssl.verify {
            self.ssl.verify = v;
        }
        if let Some(v) = overlay.ssl.ca_cert {
            self.ssl.ca_cert = Some(v);
        }
        if let Some(v) = overlay.ssl.client_cert {
            self.ssl.client_cert = Some(v);
        }
        if let Some(v) = overlay.ssl.client_key {
            self.ssl.client_key = Some(v);
        }
        if let Some(v) = overlay.ui.sidebar_width {
            self.ui.sidebar_width = v;
        }
        if let Some(v) = overlay.editor.tab_size {
            self.editor.tab_size = v;
        }
        self
    }
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

const CONFIG_DIR_NAME: &str = "perseus";
const CONFIG_FILE_NAME: &str = "config.toml";

fn global_config_path() -> Option<PathBuf> {
    if let Ok(dir) = env::var("XDG_CONFIG_HOME") {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir).join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME));
        }
    }
    let home = env::var("HOME").ok()?;
    if home.trim().is_empty() {
        return None;
    }
    Some(
        PathBuf::from(home)
            .join(".config")
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME),
    )
}

fn project_config_path() -> Option<PathBuf> {
    let root = find_project_root()?;
    let path = root.join(".perseus").join(CONFIG_FILE_NAME);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tilde expansion
// ---------------------------------------------------------------------------

fn expand_tilde(path: &PathBuf) -> PathBuf {
    if let Some(s) = path.to_str() {
        if let Some(rest) = s.strip_prefix('~') {
            if let Ok(home) = env::var("HOME") {
                return PathBuf::from(home).join(rest.strip_prefix('/').unwrap_or(rest));
            }
        }
    }
    path.clone()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ConfigError {
    pub messages: Vec<String>,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for msg in &self.messages {
            writeln!(f, "{}", msg)?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();

        if self.http.timeout > 600 {
            errors.push(format!(
                "config error: http.timeout = {} is out of range (0..=600)",
                self.http.timeout
            ));
        }
        if self.http.max_redirects > 100 {
            errors.push(format!(
                "config error: http.max_redirects = {} is out of range (0..=100)",
                self.http.max_redirects
            ));
        }
        if !(28..=60).contains(&self.ui.sidebar_width) {
            errors.push(format!(
                "config error: ui.sidebar_width = {} is out of range (28..=60)",
                self.ui.sidebar_width
            ));
        }
        if !(1..=8).contains(&self.editor.tab_size) {
            errors.push(format!(
                "config error: editor.tab_size = {} is out of range (1..=8)",
                self.editor.tab_size
            ));
        }

        if let Some(ref url) = self.proxy.url {
            if reqwest::Url::parse(url).is_err() {
                errors.push(format!(
                    "config error: proxy.url = \"{}\" is not a valid URL",
                    url
                ));
            }
        }

        if let Some(ref path) = self.ssl.ca_cert {
            let expanded = expand_tilde(path);
            if !expanded.exists() {
                errors.push(format!(
                    "config error: ssl.ca_cert = \"{}\" — file not found",
                    expanded.display()
                ));
            }
        }
        if let Some(ref path) = self.ssl.client_cert {
            let expanded = expand_tilde(path);
            if !expanded.exists() {
                errors.push(format!(
                    "config error: ssl.client_cert = \"{}\" — file not found",
                    expanded.display()
                ));
            }
        }
        if let Some(ref path) = self.ssl.client_key {
            let expanded = expand_tilde(path);
            if !expanded.exists() {
                errors.push(format!(
                    "config error: ssl.client_key = \"{}\" — file not found",
                    expanded.display()
                ));
            }
        }

        let has_cert = self.ssl.client_cert.is_some();
        let has_key = self.ssl.client_key.is_some();
        if has_cert != has_key {
            errors.push(
                "config error: ssl.client_cert and ssl.client_key must both be set or both be unset"
                    .to_string(),
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError { messages: errors })
        }
    }

    /// Expand tilde in all path fields. Called after merging, before validation.
    fn expand_paths(&mut self) {
        if let Some(ref path) = self.ssl.ca_cert {
            self.ssl.ca_cert = Some(expand_tilde(path));
        }
        if let Some(ref path) = self.ssl.client_cert {
            self.ssl.client_cert = Some(expand_tilde(path));
        }
        if let Some(ref path) = self.ssl.client_key {
            self.ssl.client_key = Some(expand_tilde(path));
        }
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

fn load_overlay(path: &PathBuf) -> Result<OverlayConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "config error: could not read \"{}\": {}",
            path.display(),
            e
        )
    })?;
    toml::from_str(&content).map_err(|e| {
        format!(
            "config error: failed to parse \"{}\": {}",
            path.display(),
            e
        )
    })
}

/// Load configuration from global and project config files.
/// Missing files are silently skipped (all defaults apply).
/// Parse or validation errors are returned as `Err`.
pub fn load_config() -> Result<Config, String> {
    let mut config = Config::default();

    // Global config layer
    if let Some(path) = global_config_path() {
        if path.exists() {
            let overlay = load_overlay(&path)?;
            config = config.merge(overlay);
        }
    }

    // Project config layer
    if let Some(path) = project_config_path() {
        let overlay = load_overlay(&path)?;
        config = config.merge(overlay);
    }

    config.expand_paths();
    config.validate().map_err(|e| e.to_string())?;

    Ok(config)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = Config::default();
        assert_eq!(config.http.timeout, 30);
        assert!(config.http.follow_redirects);
        assert_eq!(config.http.max_redirects, 10);
        assert!(config.proxy.url.is_none());
        assert!(config.proxy.no_proxy.is_none());
        assert!(config.ssl.verify);
        assert!(config.ssl.ca_cert.is_none());
        assert!(config.ssl.client_cert.is_none());
        assert!(config.ssl.client_key.is_none());
        assert_eq!(config.ui.sidebar_width, 32);
        assert_eq!(config.editor.tab_size, 2);
    }

    #[test]
    fn test_parse_valid_toml() {
        let toml_str = r#"
[http]
timeout = 10
follow_redirects = false
max_redirects = 5

[proxy]
url = "http://proxy.corp:8080"
no_proxy = "localhost,127.0.0.1"

[ssl]
verify = false

[ui]
sidebar_width = 36

[editor]
tab_size = 4
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.http.timeout, 10);
        assert!(!config.http.follow_redirects);
        assert_eq!(config.http.max_redirects, 5);
        assert_eq!(config.proxy.url.as_deref(), Some("http://proxy.corp:8080"));
        assert_eq!(config.proxy.no_proxy.as_deref(), Some("localhost,127.0.0.1"));
        assert!(!config.ssl.verify);
        assert_eq!(config.ui.sidebar_width, 36);
        assert_eq!(config.editor.tab_size, 4);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
[http]
timeout = 5
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.http.timeout, 5);
        // All other fields retain defaults
        assert!(config.http.follow_redirects);
        assert_eq!(config.http.max_redirects, 10);
        assert_eq!(config.ui.sidebar_width, 32);
    }

    #[test]
    fn test_parse_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.http.timeout, 30);
        assert_eq!(config.ui.sidebar_width, 32);
    }

    #[test]
    fn test_unknown_keys_ignored() {
        let toml_str = r#"
[http]
timeout = 15
unknown_field = "hello"

[unknown_section]
key = "value"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.http.timeout, 15);
    }

    // -- Merge tests --

    #[test]
    fn test_merge_empty_overlay() {
        let base = Config::default();
        let overlay = OverlayConfig::default();
        let merged = base.merge(overlay);
        assert_eq!(merged.http.timeout, 30);
        assert!(merged.http.follow_redirects);
        assert_eq!(merged.ui.sidebar_width, 32);
    }

    #[test]
    fn test_merge_partial_overlay() {
        let base = Config::default();
        let overlay_str = r#"
[http]
timeout = 60
"#;
        let overlay: OverlayConfig = toml::from_str(overlay_str).unwrap();
        let merged = base.merge(overlay);
        assert_eq!(merged.http.timeout, 60);
        // Other fields unchanged
        assert!(merged.http.follow_redirects);
        assert_eq!(merged.http.max_redirects, 10);
        assert_eq!(merged.ui.sidebar_width, 32);
    }

    #[test]
    fn test_merge_proxy_field_level() {
        let mut base = Config::default();
        base.proxy.url = Some("http://global-proxy:8080".into());
        base.proxy.no_proxy = Some("localhost".into());

        // Project overlay only overrides url, not no_proxy
        let overlay_str = r#"
[proxy]
url = "http://project-proxy:9090"
"#;
        let overlay: OverlayConfig = toml::from_str(overlay_str).unwrap();
        let merged = base.merge(overlay);

        assert_eq!(merged.proxy.url.as_deref(), Some("http://project-proxy:9090"));
        // no_proxy survives from the global layer
        assert_eq!(merged.proxy.no_proxy.as_deref(), Some("localhost"));
    }

    // -- Validation tests --

    #[test]
    fn test_validate_defaults_pass() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_timeout_out_of_range() {
        let mut config = Config::default();
        config.http.timeout = 999;
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("http.timeout"));
        assert!(err.messages[0].contains("999"));
    }

    #[test]
    fn test_validate_max_redirects_out_of_range() {
        let mut config = Config::default();
        config.http.max_redirects = 200;
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("http.max_redirects"));
    }

    #[test]
    fn test_validate_sidebar_width_out_of_range() {
        let mut config = Config::default();
        config.ui.sidebar_width = 999;
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("ui.sidebar_width"));
    }

    #[test]
    fn test_validate_tab_size_out_of_range() {
        let mut config = Config::default();
        config.editor.tab_size = 0;
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("editor.tab_size"));
    }

    #[test]
    fn test_validate_invalid_proxy_url() {
        let mut config = Config::default();
        config.proxy.url = Some("not a url".into());
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("proxy.url"));
    }

    #[test]
    fn test_validate_cert_key_mismatch() {
        let mut config = Config::default();
        config.ssl.client_cert = Some(PathBuf::from("/tmp/cert.pem"));
        // client_key is None — mismatch
        let err = config.validate().unwrap_err();
        let all = err.messages.join("\n");
        assert!(all.contains("both be set or both be unset"));
    }

    #[test]
    fn test_validate_nonexistent_ca_cert() {
        let mut config = Config::default();
        config.ssl.ca_cert = Some(PathBuf::from("/nonexistent/ca.pem"));
        let err = config.validate().unwrap_err();
        assert!(err.messages[0].contains("ssl.ca_cert"));
        assert!(err.messages[0].contains("file not found"));
    }

    #[test]
    fn test_validate_zero_timeout_valid() {
        let mut config = Config::default();
        config.http.timeout = 0;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_boundary_values() {
        let mut config = Config::default();
        config.http.timeout = 600;
        config.http.max_redirects = 100;
        config.ui.sidebar_width = 28;
        config.editor.tab_size = 1;
        assert!(config.validate().is_ok());

        config.ui.sidebar_width = 60;
        config.editor.tab_size = 8;
        assert!(config.validate().is_ok());
    }

    // -- Tilde expansion tests --

    #[test]
    fn test_expand_tilde() {
        let home = env::var("HOME").unwrap_or_else(|_| "/home/test".into());
        let path = PathBuf::from("~/certs/ca.pem");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, PathBuf::from(format!("{}/certs/ca.pem", home)));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = PathBuf::from("/absolute/path/ca.pem");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, path);
    }
}
