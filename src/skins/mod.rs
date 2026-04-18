#![allow(dead_code)]

use crate::segments::{SegmentRich, SegmentRows};
use crate::theme::Theme;

pub trait Skin: Sync {
    fn name(&self) -> &'static str;
    fn render(&self, rows: &SegmentRows, theme: &Theme, cols: usize, suffix: &str) -> String;
}

/// Priority-based truncation shared by all skins: drops the highest-priority-number
/// (least important) segments until total width fits in `cols - suffix_width`.
/// Preserves original insertion order. Tie-break: earliest-index drops first.
pub fn fit_segments(segments: &[SegmentRich], cols: usize, suffix_width: usize) -> Vec<SegmentRich> {
    let mut kept: Vec<SegmentRich> = segments.to_vec();

    loop {
        let total_width = estimate_width(&kept) + suffix_width;
        if total_width <= cols || kept.len() <= 1 {
            return kept;
        }
        // Pick the earliest index among those with maximum priority number.
        let victim = kept
            .iter()
            .enumerate()
            .rev()
            .max_by_key(|(_, s)| s.priority)
            .map(|(i, _)| i);
        match victim {
            Some(i) => {
                kept.remove(i);
            }
            None => return kept,
        }
    }
}

fn estimate_width(kept: &[SegmentRich]) -> usize {
    kept.iter()
        .map(|s| crate::term::display_width(&crate::term::strip_ansi(&s.text)) + 2)
        .sum()
}

pub fn resolve_skin(name: &str) -> &'static dyn Skin {
    match name {
        "powerline" => &POWERLINE_STUB,
        _ => &POWERLINE_STUB,
    }
}

struct PowerlineStub;
impl Skin for PowerlineStub {
    fn name(&self) -> &'static str {
        "powerline"
    }
    fn render(&self, _rows: &SegmentRows, _theme: &Theme, _cols: usize, _suffix: &str) -> String {
        String::new()
    }
}
static POWERLINE_STUB: PowerlineStub = PowerlineStub;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segments::SegmentKind;

    pub(crate) fn seg(priority: u8, text: &str) -> SegmentRich {
        SegmentRich {
            kind: SegmentKind::Model(text.to_string()),
            text: text.to_string(),
            bg: (0, 0, 0),
            fg: (255, 255, 255),
            icon: "",
            priority,
        }
    }

    #[test]
    fn fit_keeps_all_when_fits() {
        let segs = vec![seg(1, "aa"), seg(2, "bb"), seg(3, "cc")];
        let got = fit_segments(&segs, 100, 0);
        assert_eq!(got.len(), 3);
    }

    #[test]
    fn fit_drops_highest_priority_first_when_overflow() {
        let segs = vec![seg(1, "aa"), seg(5, "bb"), seg(3, "cc")];
        let got = fit_segments(&segs, 8, 0);
        assert!(got.iter().all(|s| s.priority != 5), "priority 5 should be dropped first");
    }

    #[test]
    fn resolve_unknown_skin_falls_back() {
        let s = resolve_skin("xyzzy");
        assert_eq!(s.name(), "powerline");
    }
}
