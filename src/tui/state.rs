use crate::settings::{SegmentFlags, Settings};
use crate::theme::ALL_THEMES;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Categories,
    Options,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Category {
    Segments,
    Theme,
}

pub const ALL_CATEGORIES: &[Category] = &[Category::Segments, Category::Theme];

pub struct SegmentDef {
    pub key: &'static str,
    pub label: &'static str,
    pub getter: fn(&SegmentFlags) -> bool,
    pub setter: fn(&mut SegmentFlags, bool),
}

pub const ALL_SEGMENTS: &[SegmentDef] = &[
    SegmentDef { key: "time",    label: "Horaire",             getter: |f| f.time,    setter: |f, v| f.time = v },
    SegmentDef { key: "model",   label: "Modèle",              getter: |f| f.model,   setter: |f, v| f.model = v },
    SegmentDef { key: "folder",  label: "Dossier",             getter: |f| f.folder,  setter: |f, v| f.folder = v },
    SegmentDef { key: "git",     label: "Git",                 getter: |f| f.git,     setter: |f, v| f.git = v },
    SegmentDef { key: "context", label: "Contexte",            getter: |f| f.context, setter: |f, v| f.context = v },
    SegmentDef { key: "cost",    label: "Coût",                getter: |f| f.cost,    setter: |f, v| f.cost = v },
    SegmentDef { key: "limits",  label: "Rate limits (5h/7d)", getter: |f| f.limits,  setter: |f, v| f.limits = v },
    SegmentDef { key: "version", label: "Version",             getter: |f| f.version, setter: |f, v| f.version = v },
];

pub struct App {
    pub settings: Settings,
    pub focus: Panel,
    pub category_idx: usize,
    pub option_idx: usize,
    pub should_quit: bool,
    pub should_save: bool,
}

impl App {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            focus: Panel::Categories,
            category_idx: 0,
            option_idx: 0,
            should_quit: false,
            should_save: false,
        }
    }

    pub fn current_category(&self) -> Category {
        ALL_CATEGORIES[self.category_idx]
    }

    pub fn toggle_current_option(&mut self) {
        match self.current_category() {
            Category::Segments => {
                let def = &ALL_SEGMENTS[self.option_idx];
                let current = (def.getter)(&self.settings.segments);
                (def.setter)(&mut self.settings.segments, !current);
            }
            Category::Theme => {
                let t = ALL_THEMES[self.option_idx];
                self.settings.theme = t.name.to_string();
            }
        }
    }

    pub fn option_count(&self) -> usize {
        match self.current_category() {
            Category::Segments => ALL_SEGMENTS.len(),
            Category::Theme => ALL_THEMES.len(),
        }
    }

    pub fn move_down(&mut self) {
        match self.focus {
            Panel::Categories => {
                if self.category_idx + 1 < ALL_CATEGORIES.len() {
                    self.category_idx += 1;
                    self.option_idx = 0;
                }
            }
            Panel::Options => {
                if self.option_idx + 1 < self.option_count() {
                    self.option_idx += 1;
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        match self.focus {
            Panel::Categories => {
                if self.category_idx > 0 {
                    self.category_idx -= 1;
                    self.option_idx = 0;
                }
            }
            Panel::Options => {
                if self.option_idx > 0 {
                    self.option_idx -= 1;
                }
            }
        }
    }

    pub fn next_panel(&mut self) {
        self.focus = match self.focus {
            Panel::Categories => Panel::Options,
            Panel::Options => Panel::Categories,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_with_defaults_and_no_quit() {
        let app = App::new(Settings::default());
        assert_eq!(app.focus, Panel::Categories);
        assert_eq!(app.category_idx, 0);
        assert_eq!(app.option_idx, 0);
        assert!(!app.should_quit);
        assert!(!app.should_save);
    }

    #[test]
    fn toggle_flips_current_segment() {
        let mut app = App::new(Settings::default());
        app.focus = Panel::Options;
        app.option_idx = 3;
        assert!(app.settings.segments.git);
        app.toggle_current_option();
        assert!(!app.settings.segments.git);
        app.toggle_current_option();
        assert!(app.settings.segments.git);
    }

    #[test]
    fn move_down_in_options_is_bounded() {
        let mut app = App::new(Settings::default());
        app.focus = Panel::Options;
        for _ in 0..50 {
            app.move_down();
        }
        assert_eq!(app.option_idx, ALL_SEGMENTS.len() - 1);
    }

    #[test]
    fn select_theme_updates_settings() {
        let mut app = App::new(Settings::default());
        app.category_idx = 1;
        app.focus = Panel::Options;
        app.option_idx = 5;
        app.toggle_current_option();
        assert_eq!(app.settings.theme, "dracula");
    }

    #[test]
    fn next_panel_toggles() {
        let mut app = App::new(Settings::default());
        assert_eq!(app.focus, Panel::Categories);
        app.next_panel();
        assert_eq!(app.focus, Panel::Options);
        app.next_panel();
        assert_eq!(app.focus, Panel::Categories);
    }
}
