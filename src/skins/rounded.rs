use super::{fit_segments, Skin};
use crate::segments::SegmentRows;
use crate::term::{bgc, display_width, fgc, strip_ansi, RST};
use crate::theme::Theme;

pub struct Rounded;

impl Skin for Rounded {
    fn name(&self) -> &'static str {
        "rounded"
    }

    fn render(&self, rows: &SegmentRows, _theme: &Theme, cols: usize, suffix: &str) -> String {
        let segments = rows.first().cloned().unwrap_or_default();
        let kept = fit_segments(&segments, cols, display_width(&strip_ansi(suffix)));

        let mut out = String::with_capacity(512);
        for (i, seg) in kept.iter().enumerate() {
            if i == 0 {
                fgc(&mut out, seg.bg);
                out.push('\u{e0b6}');
                out.push_str(RST);
            } else if let Some(prev) = kept.get(i - 1) {
                fgc(&mut out, prev.bg);
                bgc(&mut out, seg.bg);
                out.push('\u{e0b0}');
                out.push_str(RST);
            }
            bgc(&mut out, seg.bg);
            out.push_str(&seg.text);
            out.push_str(RST);
        }
        if let Some(last) = kept.last() {
            fgc(&mut out, last.bg);
            out.push('\u{e0b4}');
            out.push_str(RST);
        }
        out.push_str(suffix);
        out
    }
}

pub static ROUNDED: Rounded = Rounded;

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
    fn rounded_uses_rounded_caps() {
        let rows = vec![vec![seg(1, " foo ")]];
        let out = ROUNDED.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains('\u{e0b6}'), "left rounded cap");
        assert!(out.contains('\u{e0b4}'), "right rounded cap");
    }
}
