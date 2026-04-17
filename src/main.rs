mod config;
mod format;
mod git;
mod input;
mod install;
mod segments;
mod sessions;
mod term;

use std::fmt::Write as _;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, Timelike};

use config::{
    BAR_WIDTH, BG_CTX, BG_FOLDER, BG_GIT, BG_LIMIT_5H, BG_LIMIT_7D, BG_MODEL, BG_TIME, CTX_EMPTY,
    ICN_ADDED, ICN_AHEAD, ICN_CALENDAR, ICN_COST, ICN_CTX, ICN_DELETED, ICN_FOLDER, ICN_GIT,
    ICN_HEART, ICN_MODEL, ICN_MODIFIED, ICN_SESSIONS, ICN_TIMER, ICN_WORKTREE,
    LIMIT_SHOW_THRESHOLD, P_COST, P_CTX, P_FOLDER, P_GIT, P_LIMIT_5H, P_LIMIT_7D, P_MODEL, P_TIME,
    TX_DARK, TX_GRAY, TX_WHITE,
};
use format::{context_bar, fmt_duration, fmt_time, fmt_tokens};
use input::ClaudeInput;
use segments::{build_powerline, Segment};
use term::{display_width, fgc, get_cols, strip_ansi, BOLD, DIM, RST};

fn main() {
    if std::io::stdin().is_terminal() {
        install::run();
        return;
    }

    // 1. Read stdin (all of it).
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        eprintln!("ccfaststatus: failed to read stdin");
        return;
    }
    let data: ClaudeInput = match serde_json::from_str(&buf) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("ccfaststatus: invalid stdin json: {}", e);
            return;
        }
    };

    let cols = get_cols();
    println!("{}", render(data, cols));
}

pub(crate) fn render(data: ClaudeInput, cols: usize) -> String {
    // 2. Resolve cwd, then start parallel collectors.
    let cwd = data
        .workspace
        .current_dir
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| data.cwd.clone());

    let cwd_for_git = cwd.clone();
    let git_handle = thread::spawn(move || git::info(&cwd_for_git));
    let sessions_handle = thread::spawn(sessions::count);

    let git_info = git_handle.join().unwrap_or_default();
    let sessions_count = sessions_handle.join().unwrap_or(0);

    // 3. Derived scalars.
    let model = shorten_model(&data.model.display_name);
    let folder = Path::new(&cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&cwd)
        .to_string();

    let ctx_pct = data
        .context_window
        .as_ref()
        .map(|c| c.used_percentage.floor() as i64)
        .unwrap_or(0);
    let ctx_size = data
        .context_window
        .as_ref()
        .map(|c| c.context_window_size)
        .unwrap_or(200_000);
    let ctx_label = if ctx_size >= 1_000_000 {
        format!("{}M", (ctx_size as f64 / 1e6).round() as u64)
    } else {
        format!("{}k", (ctx_size as f64 / 1000.0).round() as u64)
    };

    let used_tokens = data
        .context_window
        .as_ref()
        .and_then(|c| c.current_usage.as_ref())
        .map(|u| u.input_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens)
        .unwrap_or(0);

    let cost_usd = data.cost.as_ref().map(|c| c.total_cost_usd).unwrap_or(0.0);
    let session_ms = data.cost.as_ref().map(|c| c.total_duration_ms).unwrap_or(0);

    let (block_pct, time_left, weekly_pct, weekly_left) = rate_limit_data(&data);

    // 4. Build segments in JS insertion order.
    let mut segments: Vec<Segment> = Vec::with_capacity(8);

    // 4.0 Time segment (heart + HH:MM + sessions + duration) — priority 5.
    let now = Local::now();
    let dur = fmt_duration(session_ms);
    let mut time_text = String::with_capacity(64);
    time_text.push(' ');
    fgc(&mut time_text, TX_WHITE);
    time_text.push_str(BOLD);
    write!(time_text, "{} {:02}:{:02}", ICN_HEART, now.hour(), now.minute()).unwrap();
    time_text.push_str(RST);
    if sessions_count > 1 {
        time_text.push(' ');
        fgc(&mut time_text, TX_GRAY);
        write!(time_text, "{}{}", ICN_SESSIONS, sessions_count).unwrap();
        time_text.push_str(RST);
    }
    if !dur.is_empty() {
        time_text.push(' ');
        fgc(&mut time_text, TX_GRAY);
        time_text.push_str(&dur);
        time_text.push_str(RST);
    }
    time_text.push(' ');
    segments.push(Segment { text: time_text, bg: BG_TIME, priority: P_TIME });

    // 4.1 Model segment — priority 1 (always visible).
    let mut model_text = String::with_capacity(32);
    model_text.push(' ');
    fgc(&mut model_text, TX_WHITE);
    model_text.push_str(BOLD);
    write!(model_text, "{} {}", ICN_MODEL, model).unwrap();
    model_text.push_str(RST);
    model_text.push(' ');
    segments.push(Segment { text: model_text, bg: BG_MODEL, priority: P_MODEL });

    // 4.2 Folder + worktree indicator — priority 4.
    let mut folder_text = String::with_capacity(64);
    folder_text.push(' ');
    fgc(&mut folder_text, TX_WHITE);
    write!(folder_text, "{} {}", ICN_FOLDER, folder).unwrap();
    folder_text.push_str(RST);
    if data.workspace.git_worktree {
        folder_text.push(' ');
        fgc(&mut folder_text, TX_WHITE);
        folder_text.push_str(ICN_WORKTREE);
        folder_text.push_str(RST);
    }
    folder_text.push(' ');
    segments.push(Segment { text: folder_text, bg: BG_FOLDER, priority: P_FOLDER });

    // 4.3 Git segment — priority 3.
    if git_info.is_repo {
        let mut gs = String::with_capacity(64);
        if git_info.ahead > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            gs.push_str(BOLD);
            write!(gs, "{}{}", ICN_AHEAD, git_info.ahead).unwrap();
            gs.push_str(RST);
        }
        if git_info.added > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_ADDED, git_info.added).unwrap();
            gs.push_str(RST);
        }
        if git_info.deleted > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_DELETED, git_info.deleted).unwrap();
            gs.push_str(RST);
        }
        if git_info.modified > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_MODIFIED, git_info.modified).unwrap();
            gs.push_str(RST);
        }
        if gs.is_empty() {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            gs.push('✓');
            gs.push_str(RST);
        }
        let mut git_text = String::with_capacity(96);
        git_text.push(' ');
        fgc(&mut git_text, TX_DARK);
        write!(git_text, "{} {}", ICN_GIT, git_info.branch).unwrap();
        git_text.push_str(RST);
        git_text.push_str(&gs);
        git_text.push(' ');
        segments.push(Segment { text: git_text, bg: BG_GIT, priority: P_GIT });
    }

    // 4.4 Context segment — priority 2 (critical).
    let bar = context_bar(ctx_pct as f64, BAR_WIDTH, CTX_EMPTY);
    let mut ctx_text = String::with_capacity(128);
    ctx_text.push(' ');
    fgc(&mut ctx_text, TX_DARK);
    ctx_text.push_str(ICN_CTX);
    ctx_text.push_str(RST);
    ctx_text.push(' ');
    ctx_text.push_str(&bar);
    ctx_text.push(' ');
    fgc(&mut ctx_text, TX_DARK);
    ctx_text.push_str(BOLD);
    write!(ctx_text, "{}%", ctx_pct).unwrap();
    ctx_text.push_str(RST);
    ctx_text.push(' ');
    fgc(&mut ctx_text, TX_DARK);
    if used_tokens > 0 {
        write!(ctx_text, "{}/{}", fmt_tokens(used_tokens), ctx_label).unwrap();
    } else {
        ctx_text.push_str(&ctx_label);
    }
    ctx_text.push_str(RST);
    ctx_text.push(' ');
    segments.push(Segment { text: ctx_text, bg: BG_CTX, priority: P_CTX });

    // 4.5 Rate limits (two segments) OR cost (one segment) — mutually exclusive.
    if data.rate_limits.is_some() {
        if let Some(pct) = block_pct {
            if pct >= LIMIT_SHOW_THRESHOLD {
                let mut t = String::with_capacity(48);
                t.push(' ');
                fgc(&mut t, TX_WHITE);
                write!(t, "{} 5h ", ICN_TIMER).unwrap();
                t.push_str(BOLD);
                write!(t, "{}%", pct).unwrap();
                t.push_str(RST);
                if !time_left.is_empty() {
                    t.push(' ');
                    fgc(&mut t, TX_WHITE);
                    t.push_str(&time_left);
                    t.push_str(RST);
                }
                t.push(' ');
                segments.push(Segment { text: t, bg: BG_LIMIT_5H, priority: P_LIMIT_5H });
            }
        }
        if let Some(pct) = weekly_pct {
            if pct >= LIMIT_SHOW_THRESHOLD {
                let mut t = String::with_capacity(48);
                t.push(' ');
                fgc(&mut t, TX_WHITE);
                write!(t, "{} 7d ", ICN_CALENDAR).unwrap();
                t.push_str(BOLD);
                write!(t, "{}%", pct).unwrap();
                t.push_str(RST);
                if !weekly_left.is_empty() {
                    t.push(' ');
                    fgc(&mut t, TX_WHITE);
                    t.push_str(&weekly_left);
                    t.push_str(RST);
                }
                t.push(' ');
                segments.push(Segment { text: t, bg: BG_LIMIT_7D, priority: P_LIMIT_7D });
            }
        }
    } else if cost_usd > 0.0 {
        let mut t = String::with_capacity(32);
        t.push(' ');
        fgc(&mut t, TX_WHITE);
        write!(t, "{} ${:.2}", ICN_COST, cost_usd).unwrap();
        t.push_str(RST);
        t.push(' ');
        segments.push(Segment { text: t, bg: BG_LIMIT_7D, priority: P_COST });
    }

    // 5. Version suffix (fallback rebuild if too wide).
    let version_suffix = if !data.version.is_empty() {
        let mut s = String::with_capacity(16);
        s.push(' ');
        fgc(&mut s, TX_GRAY);
        s.push_str(DIM);
        write!(s, "v{}", data.version).unwrap();
        s.push_str(RST);
        s
    } else {
        String::new()
    };

    let mut output = build_powerline(segments.clone(), &version_suffix, cols);
    if !version_suffix.is_empty() && display_width(&strip_ansi(&output)) > cols {
        output = build_powerline(segments, "", cols);
    }

    output
}

/// Shorten the model display name: "Opus 4.7 (1M context)" → "Opus".
/// Strips "(...)" then a trailing " N" or " N.N" version suffix.
fn shorten_model(raw: &str) -> String {
    let stripped_parens = match raw.find('(') {
        Some(pos) => raw[..pos].trim_end().to_string(),
        None => raw.to_string(),
    };
    trim_trailing_version(&stripped_parens)
}

/// Remove a trailing " <digits>" or " <digits>.<digits>" suffix, matching the
/// JS regex `\s+\d+(\.\d+)?$`. Requires at least one whitespace before digits
/// and requires digits at the very end.
fn trim_trailing_version(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut i = chars.len();

    // 1. Consume trailing digits. If none, the regex can't match.
    let digits_end = i;
    while i > 0 && chars[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == digits_end {
        return s.to_string();
    }

    // 2. Optional ".<digits>" prefix to the tail digits.
    if i > 0 && chars[i - 1] == '.' {
        let dot_pos = i - 1;
        let mut j = dot_pos;
        while j > 0 && chars[j - 1].is_ascii_digit() {
            j -= 1;
        }
        if j < dot_pos {
            i = j;
        }
        // else: dot without preceding digits — leave i unchanged (dot stays part of kept text).
    }

    // 3. Require at least one whitespace separator.
    let mut ws_end = i;
    while ws_end > 0 && chars[ws_end - 1].is_whitespace() {
        ws_end -= 1;
    }
    if ws_end == i {
        return s.to_string();
    }

    chars[..ws_end].iter().collect()
}

/// Compute rate-limit percentages and time-left strings.
/// Returns `(five_hour_pct, five_hour_time_left, seven_day_pct, seven_day_time_left)`.
fn rate_limit_data(d: &ClaudeInput) -> (Option<i64>, String, Option<i64>, String) {
    let now_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let rl = match d.rate_limits.as_ref() {
        Some(r) => r,
        None => return (None, String::new(), None, String::new()),
    };

    let mut block_pct = None;
    let mut time_left = String::new();
    if let Some(fh) = rl.five_hour.as_ref() {
        block_pct = Some(fh.used_percentage.round().clamp(0.0, 100.0) as i64);
        if let Some(resets) = fh.resets_at {
            time_left = fmt_time((resets - now_s) * 1000);
        }
    }

    let mut weekly_pct = None;
    let mut weekly_left = String::new();
    if let Some(sd) = rl.seven_day.as_ref() {
        weekly_pct = Some(sd.used_percentage.round().clamp(0.0, 100.0) as i64);
        if let Some(resets) = sd.resets_at {
            weekly_left = fmt_time((resets - now_s) * 1000);
        }
    }

    (block_pct, time_left, weekly_pct, weekly_left)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorten_model_full() {
        assert_eq!(shorten_model("Opus 4.7 (1M context)"), "Opus");
    }

    #[test]
    fn shorten_model_minor_version() {
        assert_eq!(shorten_model("Sonnet 4.6"), "Sonnet");
    }

    #[test]
    fn shorten_model_no_version() {
        assert_eq!(shorten_model("Haiku"), "Haiku");
    }

    #[test]
    fn shorten_model_integer_version() {
        assert_eq!(shorten_model("Opus 4"), "Opus");
    }

    #[test]
    fn shorten_model_parens_no_version() {
        assert_eq!(shorten_model("Claude (beta)"), "Claude");
    }

    #[test]
    fn trim_keeps_string_with_no_trailing_digits() {
        assert_eq!(trim_trailing_version("Claude 4."), "Claude 4.");
    }
}
