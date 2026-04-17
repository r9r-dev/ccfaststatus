//! Golden snapshot tests: compare ccfaststatus output against captured JS reference output.
//!
//! Time-sensitive substrings (HH:MM, duration, time_left, session count) are masked
//! before comparison. To regenerate expected files, set UPDATE_GOLDEN=1 (captures
//! the Rust output; the JS reference must still be used as ground truth).

use regex::Regex;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Mask time-dependent substrings so two captures taken at different wall-clock times
/// compare equal. Order matters: compound patterns (e.g., "3h22m") before simpler ones ("22m").
fn normalize(s: &str) -> String {
    // HH:MM — the heart segment clock.
    let re_hhmm = Regex::new(r"\b\d{2}:\d{2}\b").unwrap();
    // Duration / time_left compound formats.
    let re_dhm = Regex::new(r"\b\d+d\d+h\b").unwrap();
    let re_hm = Regex::new(r"\b\d+h\d{1,2}m\b").unwrap();
    let re_ms = Regex::new(r"\b\d+m\d{2}s\b").unwrap();
    // Simple formats. No lookaround — the `regex` crate does not support it.
    // Safe because ANSI color escapes are byte-identical between expected and actual,
    // so any spurious match (e.g., the `m` in `255m`) normalises identically on both sides.
    let re_m_only = Regex::new(r"\b\d+m\b").unwrap();
    let re_s_only = Regex::new(r"\b\d+s\b").unwrap();
    let re_h_only = Regex::new(r"\b\d+h\b").unwrap();
    let re_d_only = Regex::new(r"\b\d+d\b").unwrap();
    // Sessions count ⧉N
    let re_sessions = Regex::new(r"⧉\d+").unwrap();
    // Git counts: added (), modified (), deleted (), ahead ()
    // The working tree state (untracked/modified/deleted/ahead) changes between runs.
    let re_git_added = Regex::new(r"\u{f0752}\d+").unwrap();
    let re_git_modified = Regex::new(r"\u{f0224}\d+").unwrap();
    let re_git_deleted = Regex::new(r"\u{f0754}\d+").unwrap();
    let re_git_ahead = Regex::new(r"\u{f005d}\d+").unwrap();

    let s = re_hhmm.replace_all(s, "XX:XX").into_owned();
    let s = re_dhm.replace_all(&s, "DdHh").into_owned();
    let s = re_hm.replace_all(&s, "HhMm").into_owned();
    let s = re_ms.replace_all(&s, "MmSs").into_owned();
    let s = re_m_only.replace_all(&s, "Mm").into_owned();
    let s = re_h_only.replace_all(&s, "Hh").into_owned();
    let s = re_s_only.replace_all(&s, "Ss").into_owned();
    let s = re_d_only.replace_all(&s, "Dd").into_owned();
    let s = re_sessions.replace_all(&s, "⧉N").into_owned();
    let s = re_git_added.replace_all(&s, "\u{f0752}N").into_owned();
    let s = re_git_modified.replace_all(&s, "\u{f0224}N").into_owned();
    let s = re_git_deleted.replace_all(&s, "\u{f0754}N").into_owned();
    let s = re_git_ahead.replace_all(&s, "\u{f005d}N").into_owned();
    s
}

fn run_bin(fixture_json: &str, cols: usize) -> String {
    let bin = env!("CARGO_BIN_EXE_ccfaststatus");
    let mut child = Command::new(bin)
        .env("COLUMNS", cols.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ccfaststatus");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(fixture_json.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string()
}

fn check(name: &str, cols: usize) {
    let json = std::fs::read_to_string(fixture_path(&format!("{}.json", name))).unwrap();
    let expected_path = fixture_path(&format!("{}.expected", name));

    let actual = run_bin(&json, cols);

    if std::env::var("UPDATE_GOLDEN").is_ok() {
        let mut to_write = actual.clone();
        to_write.push('\n');
        std::fs::write(&expected_path, &to_write).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&expected_path).unwrap();
    let n_expected = normalize(expected.trim_end_matches('\n'));
    let n_actual = normalize(&actual);

    assert_eq!(
        n_actual, n_expected,
        "snapshot mismatch for {}\n\nACTUAL (normalized):\n{}\n\nEXPECTED (normalized):\n{}\n\nACTUAL (raw):\n{}",
        name, n_actual, n_expected, actual
    );
}

#[test]
fn minimal() {
    check("minimal", 200);
}

#[test]
fn with_git() {
    // Clear cache so a fresh git walk runs for deterministic repo state.
    let _ = std::fs::remove_file("/tmp/.claude-statusline-git-cache.bin");
    check("with_git", 200);
}

#[test]
fn rate_limits() {
    check("rate_limits", 200);
}

#[test]
fn narrow_80cols() {
    let _ = std::fs::remove_file("/tmp/.claude-statusline-git-cache.bin");
    check("narrow_80cols", 80);
}
