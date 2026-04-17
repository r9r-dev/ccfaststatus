use crate::config::Rgb;
use std::fmt::Write;

pub const RST: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

/// Appends an ANSI 24-bit foreground color escape into `out`.
pub fn fgc(out: &mut String, (r, g, b): Rgb) {
    write!(out, "\x1b[38;2;{};{};{}m", r, g, b).unwrap();
}

/// Appends an ANSI 24-bit background color escape into `out`.
pub fn bgc(out: &mut String, (r, g, b): Rgb) {
    write!(out, "\x1b[48;2;{};{};{}m", r, g, b).unwrap();
}

/// Convenience: returns the foreground color escape as an owned `String`.
#[allow(dead_code)]
pub fn fgc_s(rgb: Rgb) -> String {
    let mut s = String::with_capacity(20);
    fgc(&mut s, rgb);
    s
}

/// Convenience: returns the background color escape as an owned `String`.
#[allow(dead_code)]
pub fn bgc_s(rgb: Rgb) -> String {
    let mut s = String::with_capacity(20);
    bgc(&mut s, rgb);
    s
}

/// Remove all ANSI CSI sequences (`ESC [ ... m`) from `s`, preserving all other characters.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // skip 'm'
            }
        } else {
            let ch_len = utf8_char_width(bytes[i]);
            // Safety net: if ch_len would read past end, clamp to 1.
            let end = (i + ch_len).min(bytes.len());
            out.push_str(&s[i..end]);
            i = end;
        }
    }
    out
}

fn utf8_char_width(b: u8) -> usize {
    if b < 0x80 { 1 }
    else if b < 0xC0 { 1 } // invalid leading byte — treat as 1 to make progress
    else if b < 0xE0 { 2 }
    else if b < 0xF0 { 3 }
    else { 4 }
}

/// Detect terminal column count. Falls back to `COLUMNS` env var, then `120`.
pub fn get_cols() -> usize {
    if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
        return w as usize;
    }
    if let Ok(v) = std::env::var("COLUMNS") {
        if let Ok(n) = v.parse::<usize>() {
            return n;
        }
    }
    120
}

/// Display width (Unicode scalar count) of an already-ANSI-stripped string.
/// Note: this does NOT handle wide CJK / variation selectors. Sufficient for our powerline content.
pub fn display_width(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_basic() {
        assert_eq!(strip_ansi("\x1b[31mhello\x1b[0m"), "hello");
    }

    #[test]
    fn strip_ansi_truecolor() {
        assert_eq!(strip_ansi("\x1b[38;2;255;0;0mX\x1b[0m"), "X");
    }

    #[test]
    fn strip_ansi_no_escape() {
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn strip_ansi_preserves_utf8() {
        // Heart (U+2665) is 3 bytes in UTF-8. Must not be split.
        assert_eq!(strip_ansi("\x1b[0m♥test"), "♥test");
    }

    #[test]
    fn display_width_counts_chars() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("♥"), 1);
    }
}
