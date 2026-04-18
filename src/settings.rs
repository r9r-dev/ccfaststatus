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
}
