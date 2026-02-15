use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::project;

// --- Data model (Postman-compatible) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(rename = "type", default = "default_type")]
    pub var_type: String,
}

fn default_true() -> bool {
    true
}

fn default_type() -> String {
    "default".to_string()
}

impl EnvironmentVariable {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            enabled: true,
            var_type: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(default)]
    pub values: Vec<EnvironmentVariable>,
}

// --- File I/O ---

pub fn load_environment(path: &Path) -> Result<Environment, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

pub fn save_environment(env: &Environment) -> Result<(), String> {
    if !is_safe_env_name(&env.name) {
        return Err(format!(
            "Invalid environment name '{}': must be non-empty and contain only alphanumeric, underscore, or hyphen characters",
            env.name
        ));
    }
    let dir = project::ensure_environments_dir()?;
    let path = dir.join(format!("{}.json", env.name));
    let json = serde_json::to_string_pretty(env)
        .map_err(|e| format!("Failed to serialize environment: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

pub fn load_all_environments() -> Result<Vec<Environment>, String> {
    let dir = match project::environments_dir() {
        Some(d) => d,
        None => return Ok(Vec::new()),
    };
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut environments = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|e| format!("Failed to read environments dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            match load_environment(&path) {
                Ok(env) => environments.push(env),
                Err(err) => eprintln!("Warning: skipping environment file: {}", err),
            }
        }
    }

    environments.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(environments)
}

pub fn delete_environment_file(name: &str) -> Result<(), String> {
    let dir = project::environments_dir()
        .ok_or("Could not find environments directory")?;
    let path = dir.join(format!("{}.json", name));
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete {}: {}", path.display(), e))?;
    }
    Ok(())
}

fn is_safe_env_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

// --- Substitution engine ---

/// Replace `{{variable}}` patterns with values from the given map.
/// Returns `(resolved_text, unresolved_variable_names)`.
pub fn substitute(template: &str, variables: &HashMap<String, String>) -> (String, Vec<String>) {
    let mut result = String::with_capacity(template.len());
    let mut unresolved = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'{') {
            chars.next(); // consume second '{'
            let mut name = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == '}' && chars.peek() == Some(&'}') {
                    chars.next(); // consume second '}'
                    closed = true;
                    break;
                }
                name.push(nc);
            }
            if closed && !name.is_empty() {
                if let Some(val) = variables.get(&name) {
                    result.push_str(val);
                } else {
                    result.push_str("{{");
                    result.push_str(&name);
                    result.push_str("}}");
                    unresolved.push(name);
                }
            } else {
                // Unclosed braces or empty name — leave as literal
                result.push_str("{{");
                result.push_str(&name);
                if closed {
                    // empty name case: {{}}
                    result.push_str("}}");
                }
            }
        } else {
            result.push(c);
        }
    }
    (result, unresolved)
}

/// Collect enabled variables from an environment into a lookup map.
pub fn resolve_variables(env: Option<&Environment>) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    if let Some(env) = env {
        for var in &env.values {
            if var.enabled {
                vars.insert(var.key.clone(), var.value.clone());
            }
        }
    }
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Serialization tests ---

    #[test]
    fn test_environment_serialize_postman_compatible() {
        let env = Environment {
            name: "dev".to_string(),
            values: vec![
                EnvironmentVariable::new("base_url", "http://localhost:3000"),
                EnvironmentVariable {
                    key: "disabled_var".to_string(),
                    value: "unused".to_string(),
                    enabled: false,
                    var_type: "secret".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["name"], "dev");
        assert_eq!(parsed["values"][0]["key"], "base_url");
        assert_eq!(parsed["values"][0]["value"], "http://localhost:3000");
        assert_eq!(parsed["values"][0]["enabled"], true);
        assert_eq!(parsed["values"][0]["type"], "default");
        assert_eq!(parsed["values"][1]["enabled"], false);
        assert_eq!(parsed["values"][1]["type"], "secret");
    }

    #[test]
    fn test_environment_deserialize_with_defaults() {
        let json = r#"{"name":"test","values":[{"key":"k","value":"v"}]}"#;
        let env: Environment = serde_json::from_str(json).unwrap();
        assert_eq!(env.values[0].enabled, true);
        assert_eq!(env.values[0].var_type, "default");
    }

    #[test]
    fn test_environment_deserialize_empty_values() {
        let json = r#"{"name":"empty"}"#;
        let env: Environment = serde_json::from_str(json).unwrap();
        assert!(env.values.is_empty());
    }

    // --- Substitution tests ---

    #[test]
    fn test_substitute_basic() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "localhost:3000".to_string());
        let (result, unresolved) = substitute("{{host}}/api", &vars);
        assert_eq!(result, "localhost:3000/api");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_substitute_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("scheme".to_string(), "https".to_string());
        vars.insert("host".to_string(), "example.com".to_string());
        vars.insert("port".to_string(), "8080".to_string());
        let (result, unresolved) = substitute("{{scheme}}://{{host}}:{{port}}", &vars);
        assert_eq!(result, "https://example.com:8080");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_substitute_unresolved() {
        let vars = HashMap::new();
        let (result, unresolved) = substitute("{{missing}}", &vars);
        assert_eq!(result, "{{missing}}");
        assert_eq!(unresolved, vec!["missing"]);
    }

    #[test]
    fn test_substitute_empty_template() {
        let vars = HashMap::new();
        let (result, unresolved) = substitute("", &vars);
        assert_eq!(result, "");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_substitute_no_variables_in_template() {
        let mut vars = HashMap::new();
        vars.insert("unused".to_string(), "val".to_string());
        let (result, unresolved) = substitute("https://example.com", &vars);
        assert_eq!(result, "https://example.com");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_substitute_adjacent_variables() {
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "hello".to_string());
        vars.insert("b".to_string(), "world".to_string());
        let (result, unresolved) = substitute("{{a}}{{b}}", &vars);
        assert_eq!(result, "helloworld");
        assert!(unresolved.is_empty());
    }

    #[test]
    fn test_substitute_unclosed_braces() {
        let vars = HashMap::new();
        let (result, _) = substitute("{{name", &vars);
        assert_eq!(result, "{{name");
    }

    #[test]
    fn test_substitute_empty_name() {
        let vars = HashMap::new();
        let (result, _) = substitute("{{}}", &vars);
        assert_eq!(result, "{{}}");
    }

    #[test]
    fn test_resolve_variables_enabled_only() {
        let env = Environment {
            name: "test".to_string(),
            values: vec![
                EnvironmentVariable::new("enabled_var", "yes"),
                EnvironmentVariable {
                    key: "disabled_var".to_string(),
                    value: "no".to_string(),
                    enabled: false,
                    var_type: "default".to_string(),
                },
            ],
        };
        let vars = resolve_variables(Some(&env));
        assert_eq!(vars.get("enabled_var"), Some(&"yes".to_string()));
        assert_eq!(vars.get("disabled_var"), None);
    }

    #[test]
    fn test_resolve_variables_none() {
        let vars = resolve_variables(None);
        assert!(vars.is_empty());
    }

    #[test]
    fn test_safe_env_name() {
        assert!(is_safe_env_name("dev"));
        assert!(is_safe_env_name("my-env"));
        assert!(is_safe_env_name("env_123"));
        assert!(!is_safe_env_name(""));
        assert!(!is_safe_env_name("bad name"));
        assert!(!is_safe_env_name("bad/name"));
        assert!(!is_safe_env_name("bad.name"));
    }
}
