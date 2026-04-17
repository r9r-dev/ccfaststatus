use crate::config::{Rgb, BAR_WIDTH};
use crate::term::{fgc, RST};

/// Format a duration in milliseconds as `"Xh"`, `"XhYm"`, or `"XdYh"`.
/// Returns `""` if `ms <= 0`.
/// Reference JS: fmtTime (statusline.mjs:61)
pub fn fmt_time(ms: i64) -> String {
    if ms <= 0 {
        return String::new();
    }
    let total_h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    if total_h >= 24 {
        return format!("{}d{}h", total_h / 24, total_h % 24);
    }
    if total_h > 0 {
        format!("{}h{}m", total_h, m)
    } else {
        format!("{}m", m)
    }
}

/// Format a session duration as `"XhYYm"`, `"XmYYs"`, or `"Xs"`.
/// Returns `""` if `ms == 0` (empty session).
/// Reference JS: fmtDuration (statusline.mjs:69)
pub fn fmt_duration(ms: u64) -> String {
    if ms == 0 {
        return String::new();
    }
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    if h > 0 {
        format!("{}h{:02}m", h, m)
    } else if m > 0 {
        format!("{}m{:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// Format a token count as `"1.2M"`, `"42k"`, or `"999"`.
/// Zero returns `"0"` (not empty).
/// Reference JS: fmtTokens (statusline.mjs:79)
pub fn fmt_tokens(n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    if n >= 1_000_000 {
        let rounded = (n as f64 / 1e5).round() / 10.0;
        return format!("{:.1}M", rounded);
    }
    if n >= 1_000 {
        return format!("{}k", (n as f64 / 1000.0).round() as u64);
    }
    n.to_string()
}

/// Simple box-drawing bar ("━━━━━" filled, "─────" empty).
/// Reference JS: miniBar (statusline.mjs:87)
pub fn mini_bar(pct: f64, empty_rgb: Rgb) -> String {
    let filled = ((pct / 100.0) * BAR_WIDTH as f64).round() as usize;
    let filled = filled.min(BAR_WIDTH);
    let mut out = String::with_capacity(32);
    fgc(&mut out, (255, 255, 255));
    for _ in 0..filled {
        out.push('━');
    }
    fgc(&mut out, empty_rgb);
    for _ in 0..(BAR_WIDTH - filled) {
        out.push('─');
    }
    out.push_str(RST);
    out
}

/// Vertical braille bar: 8 fill levels per cell, fixed character width.
/// Reference JS: contextBar (statusline.mjs:93)
pub fn context_bar(pct: f64, width: usize, empty_rgb: Rgb) -> String {
    // Fill pattern order (bottom → top, left → right alternating per the JS source).
    const DOTS: [u16; 8] = [0x40, 0x80, 0x04, 0x20, 0x02, 0x10, 0x01, 0x08];
    let steps = DOTS.len();
    let total = ((pct / 100.0) * (width * steps) as f64).round() as isize;

    let mut out = String::with_capacity(width * 8);
    for i in 0..width {
        let filled = (total - (i * steps) as isize).clamp(0, steps as isize) as usize;
        let mut bits: u16 = 0;
        for s in 0..filled {
            bits |= DOTS[s];
        }
        let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
        if filled == 0 {
            fgc(&mut out, empty_rgb);
        } else {
            fgc(&mut out, (255, 255, 255));
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_time_zero_or_negative() {
        assert_eq!(fmt_time(0), "");
        assert_eq!(fmt_time(-1), "");
    }

    #[test]
    fn fmt_time_minutes_only() {
        assert_eq!(fmt_time(15 * 60_000), "15m");
    }

    #[test]
    fn fmt_time_hours_and_minutes() {
        assert_eq!(fmt_time(3 * 3_600_000 + 22 * 60_000), "3h22m");
    }

    #[test]
    fn fmt_time_days_and_hours() {
        assert_eq!(fmt_time(2 * 24 * 3_600_000 + 5 * 3_600_000), "2d5h");
    }

    #[test]
    fn fmt_duration_seconds() {
        assert_eq!(fmt_duration(42_000), "42s");
    }

    #[test]
    fn fmt_duration_minutes() {
        assert_eq!(fmt_duration(3 * 60_000 + 7_000), "3m07s");
    }

    #[test]
    fn fmt_duration_hours() {
        assert_eq!(fmt_duration(2 * 3_600_000 + 5 * 60_000), "2h05m");
    }

    #[test]
    fn fmt_duration_zero() {
        assert_eq!(fmt_duration(0), "");
    }

    #[test]
    fn fmt_tokens_small() {
        assert_eq!(fmt_tokens(0), "0");
        assert_eq!(fmt_tokens(42), "42");
        assert_eq!(fmt_tokens(999), "999");
    }

    #[test]
    fn fmt_tokens_thousands() {
        assert_eq!(fmt_tokens(1500), "2k");
        assert_eq!(fmt_tokens(42_000), "42k");
    }

    #[test]
    fn fmt_tokens_millions() {
        assert_eq!(fmt_tokens(1_200_000), "1.2M");
        assert_eq!(fmt_tokens(3_456_789), "3.5M");
        // Regression: ensure half-away-from-zero tie-break matches JS toFixed(1).
        assert_eq!(fmt_tokens(1_250_000), "1.3M", "1.25 must round up like JS toFixed");
        assert_eq!(fmt_tokens(2_250_000), "2.3M", "2.25 must round up like JS toFixed");
        assert_eq!(fmt_tokens(1_350_000), "1.4M");
        assert_eq!(fmt_tokens(3_500_000), "3.5M");
    }
}

#[cfg(test)]
mod tests_bars {
    use super::*;

    #[test]
    fn mini_bar_zero_percent() {
        let s = mini_bar(0.0, (70, 110, 140));
        // No filled ━, exactly BAR_WIDTH empty ─
        assert!(!s.contains('━'));
        assert_eq!(s.matches('─').count(), BAR_WIDTH);
    }

    #[test]
    fn mini_bar_full() {
        let s = mini_bar(100.0, (70, 110, 140));
        assert_eq!(s.matches('━').count(), BAR_WIDTH);
        assert!(!s.contains('─'));
    }

    #[test]
    fn mini_bar_half() {
        let s = mini_bar(50.0, (70, 110, 140));
        // 0.5 * 5 = 2.5 → round half-away-from-zero → 3 filled, 2 empty.
        assert_eq!(s.matches('━').count(), 3);
        assert_eq!(s.matches('─').count(), 2);
    }

    #[test]
    fn context_bar_zero_pct() {
        let s = context_bar(0.0, 5, (70, 110, 140));
        // All cells are braille blank U+2800.
        assert_eq!(s.matches('\u{2800}').count(), 5);
    }

    #[test]
    fn context_bar_full() {
        let s = context_bar(100.0, 5, (70, 110, 140));
        // All cells are braille full U+28FF.
        assert_eq!(s.matches('\u{28FF}').count(), 5);
    }
}
