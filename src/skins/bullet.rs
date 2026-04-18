use super::{fit_segments, Skin};
use crate::segments::{SegmentKind, SegmentRich, SegmentRows};
use crate::term::{display_width, fgc, strip_ansi, RST};
use crate::theme::{Rgb, Theme};

pub struct Bullet;

fn pct_color(pct: f64) -> Rgb {
    let p = pct.clamp(0.0, 100.0);
    if p < 50.0 {
        let t = p / 50.0;
        let r = (255.0 * t) as u8;
        let g = 200;
        let b = 100_u8.saturating_sub((100.0 * t) as u8);
        (r, g, b)
    } else {
        let t = (p - 50.0) / 50.0;
        let r = 255;
        let g = (200.0 * (1.0 - t) + 80.0 * t) as u8;
        let b = 0;
        (r, g, b)
    }
}

fn render_segment(seg: &SegmentRich, out: &mut String) {
    match &seg.kind {
        SegmentKind::Context { pct, .. } => {
            fgc(out, pct_color(*pct));
            out.push('\u{25cf}');
            out.push_str(RST);
            out.push_str(&format!(" {:.0}%", pct));
        }
        SegmentKind::Limit5h { pct, .. } | SegmentKind::Limit7d { pct, .. } => {
            fgc(out, pct_color(*pct as f64));
            out.push('\u{25cf}');
            out.push_str(RST);
            out.push_str(&format!(" {}%", pct));
        }
        SegmentKind::Cost(usd) => {
            let pct = (usd * 100.0).min(100.0);
            fgc(out, pct_color(pct));
            out.push('\u{25cf}');
            out.push_str(RST);
            out.push_str(&format!(" ${:.2}", usd));
        }
        _ => {
            fgc(out, seg.bg);
            out.push_str(strip_ansi(&seg.text).trim());
            out.push_str(RST);
        }
    }
}

impl Skin for Bullet {
    fn name(&self) -> &'static str {
        "bullet"
    }

    fn render(&self, rows: &SegmentRows, _theme: &Theme, cols: usize, suffix: &str) -> String {
        let segments = rows.first().cloned().unwrap_or_default();
        let kept = fit_segments(&segments, cols, display_width(&strip_ansi(suffix)));
        let mut out = String::with_capacity(256);
        for (i, seg) in kept.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            render_segment(seg, &mut out);
        }
        out.push_str(suffix);
        out
    }
}

pub static BULLET: Bullet = Bullet;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::SegmentRich;

    #[test]
    fn bullet_renders_context_with_colored_dot() {
        let s = SegmentRich {
            kind: SegmentKind::Context { pct: 42.0, used_tokens: 100_000, size_label: "200k".to_string() },
            text: "dummy".to_string(),
            bg: (0, 0, 0),
            fg: (0, 0, 0),
            icon: "",
            priority: 2,
        };
        let rows = vec![vec![s]];
        let out = BULLET.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains('\u{25cf}'), "bullet dot present");
        assert!(out.contains("42%"));
    }

    #[test]
    fn bullet_non_gauge_falls_back_to_minimal_style() {
        let s = SegmentRich {
            kind: SegmentKind::Model("Opus".to_string()),
            text: " Opus ".to_string(),
            bg: (154, 52, 142),
            fg: (255, 255, 255),
            icon: "",
            priority: 1,
        };
        let rows = vec![vec![s]];
        let out = BULLET.render(&rows, &crate::theme::M365PRINCESS, 200, "");
        assert!(out.contains("Opus"));
        assert!(!out.contains('\u{25cf}'), "no bullet for non-gauge");
    }

    #[test]
    fn pct_color_green_at_zero() {
        let (r, g, b) = pct_color(0.0);
        assert_eq!(r, 0);
        assert_eq!(g, 200);
        assert_eq!(b, 100);
    }

    #[test]
    fn pct_color_red_at_hundred() {
        let (r, g, b) = pct_color(100.0);
        assert_eq!(r, 255);
        assert_eq!(g, 80);
        assert_eq!(b, 0);
    }
}
