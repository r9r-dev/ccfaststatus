use super::Skin;
use crate::segments::{build_powerline, Segment, SegmentRows};
use crate::theme::Theme;

pub struct Powerline;

impl Skin for Powerline {
    fn name(&self) -> &'static str {
        "powerline"
    }

    fn render(&self, rows: &SegmentRows, _theme: &Theme, cols: usize, suffix: &str) -> String {
        // v0.5 : 1 row. Multi-row prévu v0.6+.
        let rich = rows.first().cloned().unwrap_or_default();
        let legacy: Vec<Segment> = rich
            .into_iter()
            .map(|r| Segment {
                text: r.text,
                bg: r.bg,
                priority: r.priority,
            })
            .collect();
        build_powerline(legacy, suffix, cols)
    }
}

pub static POWERLINE: Powerline = Powerline;

#[cfg(test)]
mod tests {
    use super::super::Skin;
    use super::*;

    #[test]
    fn powerline_renders_segment_with_triangle() {
        use crate::segments::{SegmentKind, SegmentRich};
        let seg = SegmentRich {
            kind: SegmentKind::Model("X".to_string()),
            text: " X ".to_string(),
            bg: (10, 10, 10),
            fg: (255, 255, 255),
            icon: "",
            priority: 1,
        };
        let rows = vec![vec![seg]];
        let out = POWERLINE.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains('\u{e0b0}'), "powerline triangle present");
        assert!(out.contains("X"));
    }
}
