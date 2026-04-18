#![allow(dead_code)]

use std::path::PathBuf;

pub fn config_path() -> Result<PathBuf, String> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("ccfaststatus").join("config.toml"));
        }
    }
    let home = std::env::var("HOME")
        .map_err(|_| "variable d'environnement HOME non définie".to_string())?;
    Ok(PathBuf::from(home).join(".config").join("ccfaststatus").join("config.toml"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentFlags {
    pub time: bool,
    pub model: bool,
    pub folder: bool,
    pub git: bool,
    pub context: bool,
    pub cost: bool,
    pub limits: bool,
    pub version: bool,
}

impl Default for SegmentFlags {
    fn default() -> Self {
        Self {
            time: true,
            model: true,
            folder: true,
            git: true,
            context: true,
            cost: true,
            limits: true,
            version: true,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub segments: SegmentFlags,
}

impl Settings {
    pub fn parse_toml(s: &str) -> Result<Self, String> {
        let value: toml::Value = s
            .parse()
            .map_err(|e: toml::de::Error| format!("TOML invalide : {}", e))?;

        let mut settings = Settings::default();

        if let Some(segs) = value.get("segments").and_then(|v| v.as_table()) {
            let get = |k: &str, d: bool| segs.get(k).and_then(|v| v.as_bool()).unwrap_or(d);
            settings.segments = SegmentFlags {
                time:    get("time",    true),
                model:   get("model",   true),
                folder:  get("folder",  true),
                git:     get("git",     true),
                context: get("context", true),
                cost:    get("cost",    true),
                limits:  get("limits",  true),
                version: get("version", true),
            };
        }

        let f = &settings.segments;
        if !f.time && !f.model && !f.folder && !f.git
            && !f.context && !f.cost && !f.limits && !f.version
        {
            settings.segments.model = true;
        }

        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_all_segments_enabled() {
        let s = Settings::default();
        assert!(s.segments.time);
        assert!(s.segments.model);
        assert!(s.segments.folder);
        assert!(s.segments.git);
        assert!(s.segments.context);
        assert!(s.segments.cost);
        assert!(s.segments.limits);
        assert!(s.segments.version);
    }

    #[test]
    fn config_path_uses_xdg_config_home_when_set() {
        let saved = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", "/custom/xdg");
        let p = config_path().unwrap();
        assert_eq!(p, std::path::PathBuf::from("/custom/xdg/ccfaststatus/config.toml"));
        match saved {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn config_path_falls_back_to_home_dot_config() {
        let saved_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let saved_home = std::env::var("HOME").ok();
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::set_var("HOME", "/tmp/fake-home");
        let p = config_path().unwrap();
        assert_eq!(p, std::path::PathBuf::from("/tmp/fake-home/.config/ccfaststatus/config.toml"));
        if let Some(v) = saved_xdg {
            std::env::set_var("XDG_CONFIG_HOME", v);
        }
        match saved_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn parse_empty_toml_returns_defaults() {
        let s = Settings::parse_toml("").unwrap();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn parse_full_config() {
        let t = r#"
[segments]
time    = false
model   = true
folder  = false
git     = true
context = true
cost    = false
limits  = true
version = false
"#;
        let s = Settings::parse_toml(t).unwrap();
        assert!(!s.segments.time);
        assert!(s.segments.model);
        assert!(!s.segments.folder);
        assert!(s.segments.git);
        assert!(!s.segments.cost);
        assert!(!s.segments.version);
    }

    #[test]
    fn parse_partial_config_keeps_defaults_for_missing_keys() {
        let t = r#"
[segments]
git = false
"#;
        let s = Settings::parse_toml(t).unwrap();
        assert!(!s.segments.git, "git explicitly disabled");
        assert!(s.segments.time, "time keeps default true");
        assert!(s.segments.model, "model keeps default true");
    }

    #[test]
    fn parse_unknown_keys_are_ignored() {
        let t = r#"
[segments]
git = false
future_segment = true

[future_section]
key = "value"
"#;
        let s = Settings::parse_toml(t).unwrap();
        assert!(!s.segments.git);
    }

    #[test]
    fn parse_malformed_toml_returns_err() {
        let t = "[[[ not valid";
        assert!(Settings::parse_toml(t).is_err());
    }

    #[test]
    fn all_flags_false_forces_model_true() {
        let t = r#"
[segments]
time = false
model = false
folder = false
git = false
context = false
cost = false
limits = false
version = false
"#;
        let s = Settings::parse_toml(t).unwrap();
        assert!(s.segments.model, "model forcé à true si tout est désactivé");
    }
}
