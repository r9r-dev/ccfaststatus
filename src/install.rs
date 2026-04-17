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

pub fn write_settings(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let pretty = serde_json::to_string_pretty(value)
        .map_err(|e| format!("serialize error: {}", e))?;
    fs::write(path, pretty + "\n")
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))
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

pub fn parse_interval(raw: &str, default: u32) -> Result<u32, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(default);
    }
    t.parse::<u32>()
        .map_err(|_| format!("not a positive integer: {}", raw.trim()))
        .and_then(|n| if n == 0 { Err("must be > 0".to_string()) } else { Ok(n) })
}

fn prompt_interval(default: u32) -> u32 {
    use std::io::{stdin, stdout, BufRead, Write};
    let stdin = stdin();
    let mut out = stdout();
    loop {
        print!("Intervalle de rafraîchissement en secondes [{}]: ", default);
        let _ = out.flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return default;
        }
        match parse_interval(&line, default) {
            Ok(n) => return n,
            Err(msg) => eprintln!("  ✗ {}", msg),
        }
    }
}

pub fn preview() -> String {
    use crate::input::ClaudeInput;
    use crate::term::get_cols;
    let data: ClaudeInput = serde_json::from_str("{}").expect("{} is valid");
    crate::render(data, get_cols())
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

    #[test]
    fn write_roundtrip() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-write.json");
        let v = json!({"statusLine": {"command": "ccfaststatus"}, "theme": "dark"});
        write_settings(&tmp, &v).unwrap();
        let re = read_settings(&tmp).unwrap();
        assert_eq!(re, v);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn write_creates_parent_dir() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-dir/settings.json");
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap());
        write_settings(&tmp, &json!({})).unwrap();
        assert!(tmp.exists());
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap());
    }

    #[test]
    fn write_is_pretty_2space() {
        let tmp = std::env::temp_dir().join("ccfaststatus-test-pretty.json");
        write_settings(&tmp, &json!({"a": {"b": 1}})).unwrap();
        let s = std::fs::read_to_string(&tmp).unwrap();
        assert!(s.contains("  \"b\""));   // 2 espaces d'indent
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn parse_interval_uses_default_on_empty() {
        assert_eq!(parse_interval("", 1).unwrap(), 1);
        assert_eq!(parse_interval("\n", 3).unwrap(), 3);
        assert_eq!(parse_interval("   ", 5).unwrap(), 5);
    }

    #[test]
    fn parse_interval_accepts_positive_int() {
        assert_eq!(parse_interval("2", 1).unwrap(), 2);
        assert_eq!(parse_interval("  42  \n", 1).unwrap(), 42);
    }

    #[test]
    fn parse_interval_rejects_zero_and_negative() {
        assert!(parse_interval("0", 1).is_err());
        assert!(parse_interval("-3", 1).is_err());
    }

    #[test]
    fn parse_interval_rejects_non_numeric() {
        assert!(parse_interval("abc", 1).is_err());
        assert!(parse_interval("1.5", 1).is_err());
    }

    #[test]
    fn preview_produces_non_empty_output() {
        let p = preview();
        assert!(!p.is_empty());
        assert!(p.contains("\x1b["));  // ANSI escape présent
    }
}
