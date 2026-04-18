use super::{minimal, Skin};
use crate::segments::SegmentRows;
use crate::term::{display_width, fgc, RST};
use crate::theme::Theme;

const RAINBOW_COLORS: [(u8, u8, u8); 6] = [
    (255, 80, 80),
    (255, 170, 60),
    (255, 230, 80),
    (80, 220, 100),
    (80, 180, 250),
    (200, 120, 255),
];

pub struct Rainbow;

impl Skin for Rainbow {
    fn name(&self) -> &'static str {
        "rainbow"
    }

    fn render(&self, rows: &SegmentRows, theme: &Theme, cols: usize, suffix: &str) -> String {
        let mut prefix = String::new();
        for c in &RAINBOW_COLORS {
            fgc(&mut prefix, *c);
            prefix.push('\u{25a0}');
            prefix.push_str(RST);
        }
        prefix.push(' ');
        let prefix_width = display_width(&crate::term::strip_ansi(&prefix));
        let body = minimal::MINIMAL.render(rows, theme, cols.saturating_sub(prefix_width), suffix);
        format!("{}{}", prefix, body)
    }
}

pub static RAINBOW: Rainbow = Rainbow;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::{SegmentKind, SegmentRich};

    fn seg(priority: u8, text: &str) -> SegmentRich {
        SegmentRich {
            kind: SegmentKind::Model(text.to_string()),
            text: text.to_string(),
            bg: (100, 50, 150),
            fg: (255, 255, 255),
            icon: "",
            priority,
        }
    }

    #[test]
    fn rainbow_starts_with_colored_prefix() {
        let rows = vec![vec![seg(1, "foo")]];
        let out = RAINBOW.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains('\u{25a0}'), "prefix square present");
        let count = out.matches("\x1b[38;2;").count();
        assert!(count >= 6, "at least 6 fg color codes for rainbow (got {})", count);
    }
}
