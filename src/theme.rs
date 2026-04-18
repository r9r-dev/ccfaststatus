#![allow(dead_code)]

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub bg_time: Rgb,
    pub bg_model: Rgb,
    pub bg_folder: Rgb,
    pub bg_git: Rgb,
    pub bg_ctx: Rgb,
    pub bg_limit_5h: Rgb,
    pub bg_limit_7d: Rgb,
    pub tx_white: Rgb,
    pub tx_dark: Rgb,
    pub tx_gray: Rgb,
    pub ctx_empty: Rgb,
}

pub const M365PRINCESS: Theme = Theme {
    name: "m365princess",
    bg_time:      (30, 30, 35),
    bg_model:     (154, 52, 142),
    bg_folder:    (218, 98, 125),
    bg_git:       (252, 161, 125),
    bg_ctx:       (134, 187, 216),
    bg_limit_5h:  (91, 143, 176),
    bg_limit_7d:  (51, 101, 138),
    tx_white:     (255, 255, 255),
    tx_dark:      (40, 25, 55),
    tx_gray:      (156, 163, 175),
    ctx_empty:    (70, 110, 140),
};

pub const ALL_THEMES: &[&Theme] = &[&M365PRINCESS];

pub fn resolve_theme(name: &str) -> &'static Theme {
    for t in ALL_THEMES {
        if t.name == name {
            return t;
        }
    }
    &M365PRINCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_theme_returns_it() {
        let t = resolve_theme("m365princess");
        assert_eq!(t.name, "m365princess");
    }

    #[test]
    fn resolve_unknown_theme_falls_back_to_default() {
        let t = resolve_theme("nonexistent");
        assert_eq!(t.name, "m365princess");
    }

    #[test]
    fn m365princess_matches_current_palette() {
        assert_eq!(M365PRINCESS.bg_time, (30, 30, 35));
        assert_eq!(M365PRINCESS.bg_model, (154, 52, 142));
        assert_eq!(M365PRINCESS.tx_white, (255, 255, 255));
    }
}
