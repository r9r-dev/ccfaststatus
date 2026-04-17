use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ClaudeInput {
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub context_window: Option<ContextWindow>,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub rate_limits: Option<RateLimits>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Workspace {
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub git_worktree: bool,
}

#[derive(Debug, Deserialize)]
pub struct ContextWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub context_window_size: u64,
    #[serde(default)]
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub total_duration_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct RateLimits {
    #[serde(default)]
    pub five_hour: Option<RateLimit>,
    #[serde(default)]
    pub seven_day: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimit {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub resets_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_payload() {
        let json = r#"{
            "model": {"display_name": "Opus 4.7 (1M context)"},
            "version": "2.0.1",
            "workspace": {"current_dir": "/tmp", "git_worktree": true},
            "context_window": {"used_percentage": 42.3, "context_window_size": 1000000},
            "cost": {"total_cost_usd": 1.23, "total_duration_ms": 600000},
            "rate_limits": {"five_hour": {"used_percentage": 42, "resets_at": 1745000000}}
        }"#;
        let parsed: ClaudeInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model.display_name, "Opus 4.7 (1M context)");
        assert_eq!(parsed.version, "2.0.1");
        assert!(parsed.workspace.git_worktree);
        assert_eq!(parsed.context_window.unwrap().context_window_size, 1000000);
    }

    #[test]
    fn parses_empty_object() {
        let parsed: ClaudeInput = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.version, "");
        assert!(parsed.context_window.is_none());
    }
}
