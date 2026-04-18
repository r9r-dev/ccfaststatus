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

pub const CATPPUCCIN: Theme = Theme {
    name: "catppuccin",
    bg_time:      (30, 30, 46),
    bg_model:     (203, 166, 247),
    bg_folder:    (245, 194, 231),
    bg_git:       (250, 179, 135),
    bg_ctx:       (137, 180, 250),
    bg_limit_5h:  (137, 180, 250),
    bg_limit_7d:  (245, 194, 231),
    tx_white:     (205, 214, 244),
    tx_dark:      (30, 30, 46),
    tx_gray:      (127, 132, 156),
    ctx_empty:    (69, 71, 90),
};

pub const TOKYO_NIGHT: Theme = Theme {
    name: "tokyo-night",
    bg_time:      (26, 27, 38),
    bg_model:     (122, 162, 247),
    bg_folder:    (187, 154, 247),
    bg_git:       (224, 175, 104),
    bg_ctx:       (125, 207, 255),
    bg_limit_5h:  (125, 207, 255),
    bg_limit_7d:  (247, 118, 142),
    tx_white:     (192, 202, 245),
    tx_dark:      (26, 27, 38),
    tx_gray:      (86, 95, 137),
    ctx_empty:    (41, 46, 66),
};

pub const GRUVBOX: Theme = Theme {
    name: "gruvbox",
    bg_time:      (40, 40, 40),
    bg_model:     (211, 134, 155),
    bg_folder:    (214, 93, 14),
    bg_git:       (250, 189, 47),
    bg_ctx:       (131, 165, 152),
    bg_limit_5h:  (131, 165, 152),
    bg_limit_7d:  (204, 36, 29),
    tx_white:     (235, 219, 178),
    tx_dark:      (40, 40, 40),
    tx_gray:      (168, 153, 132),
    ctx_empty:    (80, 73, 69),
};

pub const NORD: Theme = Theme {
    name: "nord",
    bg_time:      (46, 52, 64),
    bg_model:     (94, 129, 172),
    bg_folder:    (129, 161, 193),
    bg_git:       (136, 192, 208),
    bg_ctx:       (143, 188, 187),
    bg_limit_5h:  (143, 188, 187),
    bg_limit_7d:  (191, 97, 106),
    tx_white:     (236, 239, 244),
    tx_dark:      (46, 52, 64),
    tx_gray:      (76, 86, 106),
    ctx_empty:    (59, 66, 82),
};

pub const DRACULA: Theme = Theme {
    name: "dracula",
    bg_time:      (40, 42, 54),
    bg_model:     (189, 147, 249),
    bg_folder:    (255, 121, 198),
    bg_git:       (241, 250, 140),
    bg_ctx:       (139, 233, 253),
    bg_limit_5h:  (139, 233, 253),
    bg_limit_7d:  (255, 85, 85),
    tx_white:     (248, 248, 242),
    tx_dark:      (40, 42, 54),
    tx_gray:      (98, 114, 164),
    ctx_empty:    (68, 71, 90),
};

pub const ALL_THEMES: &[&Theme] = &[
    &M365PRINCESS,
    &CATPPUCCIN,
    &TOKYO_NIGHT,
    &GRUVBOX,
    &NORD,
    &DRACULA,
];

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

    #[test]
    fn all_six_themes_resolvable() {
        for name in ["m365princess", "catppuccin", "tokyo-night", "gruvbox", "nord", "dracula"] {
            assert_eq!(resolve_theme(name).name, name, "theme '{}' not resolvable", name);
        }
    }

    #[test]
    fn all_theme_names_unique() {
        let mut names: Vec<&str> = ALL_THEMES.iter().map(|t| t.name).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate theme name");
    }
}
