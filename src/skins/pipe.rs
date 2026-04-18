use super::{fit_segments, Skin};
use crate::segments::SegmentRows;
use crate::term::{display_width, fgc, strip_ansi, RST};
use crate::theme::Theme;

pub struct Pipe;

impl Skin for Pipe {
    fn name(&self) -> &'static str {
        "pipe"
    }

    fn render(&self, rows: &SegmentRows, _theme: &Theme, cols: usize, suffix: &str) -> String {
        let segments = rows.first().cloned().unwrap_or_default();
        let kept = fit_segments(&segments, cols, display_width(&strip_ansi(suffix)));

        let mut out = String::with_capacity(256);
        for (i, seg) in kept.iter().enumerate() {
            if i > 0 {
                out.push_str(" | ");
            }
            fgc(&mut out, seg.bg);
            out.push_str(strip_ansi(&seg.text).trim());
            out.push_str(RST);
        }
        out.push_str(suffix);
        out
    }
}

pub static PIPE: Pipe = Pipe;

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
    fn pipe_uses_pipe_separator() {
        let rows = vec![vec![seg(1, " foo "), seg(2, " bar ")]];
        let out = PIPE.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains(" | "));
    }
}
