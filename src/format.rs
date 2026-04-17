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
        return format!("{:.1}M", n as f64 / 1e6);
    }
    if n >= 1_000 {
        return format!("{}k", (n as f64 / 1000.0).round() as u64);
    }
    n.to_string()
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
    }
}
