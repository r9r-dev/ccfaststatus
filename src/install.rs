use serde_json::{json, Value};

pub fn run() {
    println!("install stub");
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
}
