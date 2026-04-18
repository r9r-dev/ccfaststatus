use super::{fit_segments, Skin};
use crate::segments::SegmentRows;
use crate::term::{display_width, fgc, strip_ansi, RST};
use crate::theme::Theme;

pub struct Minimal;

impl Skin for Minimal {
    fn name(&self) -> &'static str {
        "minimal"
    }

    fn render(&self, rows: &SegmentRows, _theme: &Theme, cols: usize, suffix: &str) -> String {
        let segments = rows.first().cloned().unwrap_or_default();
        let kept = fit_segments(&segments, cols, display_width(&strip_ansi(suffix)));

        let mut out = String::with_capacity(256);
        for (i, seg) in kept.iter().enumerate() {
            if i > 0 {
                out.push_str(" · ");
            }
            fgc(&mut out, seg.bg);
            // Use text but stripped of ANSI (fg becomes seg.bg uniformly).
            out.push_str(strip_ansi(&seg.text).trim());
            out.push_str(RST);
        }
        out.push_str(suffix);
        out
    }
}

pub static MINIMAL: Minimal = Minimal;

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
    fn minimal_uses_middot_separator_no_bg() {
        let rows = vec![vec![seg(1, " foo "), seg(2, " bar ")]];
        let out = MINIMAL.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains(" · "));
        assert!(!out.contains('\u{e0b0}'), "no powerline separator");
    }
}
