use crate::config::{Rgb, PW};
use crate::term::{bgc, display_width, fgc, strip_ansi, RST};

#[derive(Clone, Debug)]
pub struct Segment {
    pub text: String, // may contain ANSI escapes (notably RST)
    pub bg: Rgb,
    pub priority: u8, // lower = more important
}

/// Render consecutive segments with powerline separators, re-injecting each segment's
/// bg after internal RSTs (so text that uses RST still ends up drawn on the segment bg).
fn render(segments: &[Segment]) -> String {
    let mut line = String::with_capacity(512);
    for (i, seg) in segments.iter().enumerate() {
        let mut bg_str = String::with_capacity(20);
        bgc(&mut bg_str, seg.bg);

        // Re-inject the bg after each inner RST so colored sub-spans return to segment bg.
        let fixed = seg.text.replace(RST, &format!("{}{}", RST, bg_str));

        line.push_str(&bg_str);
        line.push_str(&fixed);

        if let Some(next) = segments.get(i + 1) {
            // Transition: RST, then fg=current_bg + bg=next_bg + triangle.
            line.push_str(RST);
            fgc(&mut line, seg.bg);
            bgc(&mut line, next.bg);
            line.push(PW);
        } else {
            // Last segment: RST, then fg=current_bg + triangle + final RST.
            line.push_str(RST);
            fgc(&mut line, seg.bg);
            line.push(PW);
            line.push_str(RST);
        }
    }
    line
}

/// Build a powerline line, dropping the lowest-priority segments (largest `priority` number)
/// until the stripped width fits in `cols`. `suffix` is appended after all segments and
/// counts toward the width budget; if it still doesn't fit with only 1 segment left,
/// returns what we have.
pub fn build_powerline(segments: Vec<Segment>, suffix: &str, cols: usize) -> String {
    let mut candidates = segments;
    loop {
        let line = render(&candidates);
        let full = if suffix.is_empty() {
            line
        } else {
            format!("{}{}", line, suffix)
        };

        let visible = display_width(&strip_ansi(&full));
        if visible <= cols || candidates.len() <= 1 {
            return full;
        }

        // Remove the segment with the largest priority (least important).
        // Tie-break: keep the earliest index. enumerate().rev() + max_by_key's
        // "returns last on ties" picks the element first in the original order.
        let worst_idx = candidates
            .iter()
            .enumerate()
            .rev()
            .max_by_key(|(_, s)| s.priority)
            .map(|(i, _)| i)
            .unwrap_or(0);
        candidates.remove(worst_idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(text: &str, bg: Rgb, priority: u8) -> Segment {
        Segment { text: text.to_string(), bg, priority }
    }

    #[test]
    fn render_single_segment_ends_with_triangle() {
        let out = render(&[seg(" A ", (10, 10, 10), 1)]);
        assert!(out.contains('\u{E0B0}'));
    }

    #[test]
    fn truncation_removes_highest_priority_first() {
        let segments = vec![
            seg(" MODEL ", (10, 10, 10), 1),
            seg(" CTX ", (20, 20, 20), 2),
            seg(" VERSION ", (30, 30, 30), 7),
        ];
        // cols narrow enough to force truncation.
        let out = build_powerline(segments, "", 15);
        assert!(!out.contains("VERSION"), "VERSION (p7) should drop first");
        assert!(out.contains("MODEL"), "MODEL (p1) must survive");
    }

    #[test]
    fn keeps_all_segments_when_cols_large() {
        let segments = vec![
            seg(" A ", (10, 10, 10), 1),
            seg(" B ", (20, 20, 20), 2),
        ];
        let out = build_powerline(segments, "", 200);
        assert!(out.contains("A"));
        assert!(out.contains("B"));
    }

    #[test]
    fn last_segment_always_kept() {
        let segments = vec![seg(" X ", (10, 10, 10), 7)];
        let out = build_powerline(segments, "", 2);
        assert!(out.contains("X"));
    }

    #[test]
    fn truncation_tie_break_keeps_earliest() {
        // Two segments with the same (largest) priority. JS drops the earliest-index first;
        // we must match. Here A, B, C, D all have the same priority 7 except B which is p1.
        // With narrow cols, we should drop A (first in insertion order among the max-priority
        // segments) before C or D.
        let segments = vec![
            seg(" A ", (10, 10, 10), 7),
            seg(" B ", (20, 20, 20), 1),
            seg(" C ", (30, 30, 30), 7),
            seg(" D ", (40, 40, 40), 7),
        ];
        // Render full width to get the baseline, then shrink until exactly one p7 is gone.
        // With cols chosen so that one segment must drop, A must be the one removed.
        // 4 segments of 3 visible chars + 1 triangle each = 16 chars total; cols=14 forces
        // exactly one drop (3 segments = 12 chars fits, 4 segments = 16 doesn't).
        let out = build_powerline(segments, "", 14);
        assert!(!out.contains(" A "), "A (earliest p7) must be dropped first");
        assert!(out.contains(" B "), "B (p1) must survive");
    }
}
