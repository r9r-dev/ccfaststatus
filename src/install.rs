use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run() {
    println!("install stub");
}

pub fn settings_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(Path::new(&home).join(".claude").join("settings.json"))
}

pub fn read_settings(path: &Path) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(s) => {
            let parsed: Value = serde_json::from_str(&s)
                .map_err(|e| format!("{} is not valid JSON: {}", path.display(), e))?;
            if !parsed.is_object() {
                return Err(format!("{} root must be a JSON object", path.display()));
            }
            Ok(parsed)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("failed to read {}: {}", path.display(), e)),
    }
}

pub fn is_already_configured(settings: &Value) -> bool {
    settings
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        == Some("ccfaststatus")
}

pub fn update_settings(mut settings: Value, interval: u32) -> Value {
    let obj = settings
        .as_object_mut()
        .expect("settings must be a JSON object");
    obj.insert(
        "statusLine".to_string(),
        json!({
            "type": "command",
            "command": "ccfaststatus",
            "refreshInterval": interval
        }),
    );
    settings
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adds_statusline_when_missing() {
        let input = json!({"otherKey": "preserved"});
        let out = update_settings(input, 2);
        assert_eq!(out["otherKey"], "preserved");
        assert_eq!(out["statusLine"]["type"], "command");
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 2);
    }

    #[test]
    fn overwrites_foreign_statusline() {
        let input = json!({
            "statusLine": {"type": "command", "command": "/old/path", "refreshInterval": 5}
        });
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
    }

    #[test]
    fn updates_only_interval_when_already_ccfaststatus() {
        let input = json!({
            "statusLine": {"type": "command", "command": "ccfaststatus", "refreshInterval": 3},
            "theme": "dark"
        });
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
        assert_eq!(out["theme"], "dark");
    }

    #[test]
    fn handles_empty_object() {
        let input = json!({});
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
    }

    #[test]
    fn read_missing_returns_empty_object() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-missing.json");
        let _ = std::fs::remove_file(&tmp);
        let v = read_settings(&tmp).expect("should not fail on missing");
        assert!(v.is_object());
        assert_eq!(v.as_object().unwrap().len(), 0);
    }

    #[test]
    fn read_valid_json_parses() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-valid.json");
        std::fs::write(&tmp, r#"{"theme":"dark"}"#).unwrap();
        let v = read_settings(&tmp).unwrap();
        assert_eq!(v["theme"], "dark");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn read_invalid_json_errors() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-invalid.json");
        std::fs::write(&tmp, "{not json}").unwrap();
        let r = read_settings(&tmp);
        assert!(r.is_err());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn read_non_object_root_errors() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-array.json");
        std::fs::write(&tmp, "[]").unwrap();
        let r = read_settings(&tmp);
        assert!(r.is_err(), "array root should be rejected");
        let _ = std::fs::remove_file(&tmp);
    }
}
