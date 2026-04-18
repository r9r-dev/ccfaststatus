mod config;
mod format;
mod git;
mod input;
mod install;
mod segments;
mod sessions;
mod settings;
mod skins;
mod term;
mod theme;
mod tui;

use std::fmt::Write as _;
use std::io::{IsTerminal, Read};
use std::path::Path;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Local, Timelike};

use config::{
    BAR_WIDTH, ICN_ADDED, ICN_AHEAD, ICN_CALENDAR, ICN_COST, ICN_CTX, ICN_DELETED, ICN_FOLDER,
    ICN_GIT, ICN_HEART, ICN_MODEL, ICN_MODIFIED, ICN_SESSIONS, ICN_TIMER, ICN_WORKTREE,
    LIMIT_SHOW_THRESHOLD, P_COST, P_CTX, P_FOLDER, P_GIT, P_LIMIT_5H, P_LIMIT_7D, P_MODEL, P_TIME,
};
use format::{context_bar, fmt_duration, fmt_time, fmt_tokens};
use input::ClaudeInput;
use segments::{SegmentKind, SegmentRich};
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
    render_with(data, cols, settings::Settings::load())
}

pub(crate) fn render_with(data: ClaudeInput, cols: usize, settings: settings::Settings) -> String {
    let flags = &settings.segments;
    let palette: &theme::Theme = theme::resolve_theme(&settings.theme);
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
    let mut segments: Vec<SegmentRich> = Vec::with_capacity(8);
    let folder_for_kind = folder.clone();
    let is_worktree = data.workspace.git_worktree;
    let git_info_for_kind = git_info.clone();
    let model_for_kind = model.clone();
    let ctx_label_for_kind = ctx_label.clone();
    let time_left_for_kind = time_left.clone();
    let weekly_left_for_kind = weekly_left.clone();

    // 4.0 Time segment (heart + HH:MM + sessions + duration) — priority 5.
    if flags.time {
        let now = Local::now();
        let dur = fmt_duration(session_ms);
        let mut time_text = String::with_capacity(64);
        time_text.push(' ');
        fgc(&mut time_text, palette.tx_white);
        time_text.push_str(BOLD);
        write!(time_text, "{} {:02}:{:02}", ICN_HEART, now.hour(), now.minute()).unwrap();
        time_text.push_str(RST);
        if sessions_count > 1 {
            time_text.push(' ');
            fgc(&mut time_text, palette.tx_gray);
            write!(time_text, "{}{}", ICN_SESSIONS, sessions_count).unwrap();
            time_text.push_str(RST);
        }
        if !dur.is_empty() {
            time_text.push(' ');
            fgc(&mut time_text, palette.tx_gray);
            time_text.push_str(&dur);
            time_text.push_str(RST);
        }
        time_text.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Time {
                hour: now.hour() as u8,
                minute: now.minute() as u8,
                sessions: sessions_count,
                duration_ms: session_ms,
            },
            text: time_text,
            bg: palette.bg_time,
            fg: palette.tx_white,
            icon: ICN_HEART,
            priority: P_TIME,
        });
    }

    // 4.1 Model segment — priority 1 (always visible).
    if flags.model {
        let mut model_text = String::with_capacity(32);
        model_text.push(' ');
        fgc(&mut model_text, palette.tx_white);
        model_text.push_str(BOLD);
        write!(model_text, "{} {}", ICN_MODEL, model).unwrap();
        model_text.push_str(RST);
        model_text.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Model(model_for_kind.clone()),
            text: model_text,
            bg: palette.bg_model,
            fg: palette.tx_white,
            icon: ICN_MODEL,
            priority: P_MODEL,
        });
    }

    // 4.2 Folder + worktree indicator — priority 4.
    if flags.folder {
        let mut folder_text = String::with_capacity(64);
        folder_text.push(' ');
        fgc(&mut folder_text, palette.tx_white);
        write!(folder_text, "{} {}", ICN_FOLDER, folder).unwrap();
        folder_text.push_str(RST);
        if data.workspace.git_worktree {
            folder_text.push(' ');
            fgc(&mut folder_text, palette.tx_white);
            folder_text.push_str(ICN_WORKTREE);
            folder_text.push_str(RST);
        }
        folder_text.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Folder {
                name: folder_for_kind.clone(),
                is_worktree,
            },
            text: folder_text,
            bg: palette.bg_folder,
            fg: palette.tx_white,
            icon: ICN_FOLDER,
            priority: P_FOLDER,
        });
    }

    // 4.3 Git segment — priority 3.
    if flags.git && git_info.is_repo {
        let mut gs = String::with_capacity(64);
        if git_info.ahead > 0 {
            gs.push(' ');
            fgc(&mut gs, palette.tx_dark);
            gs.push_str(BOLD);
            write!(gs, "{}{}", ICN_AHEAD, git_info.ahead).unwrap();
            gs.push_str(RST);
        }
        if git_info.added > 0 {
            gs.push(' ');
            fgc(&mut gs, palette.tx_dark);
            write!(gs, "{}{}", ICN_ADDED, git_info.added).unwrap();
            gs.push_str(RST);
        }
        if git_info.deleted > 0 {
            gs.push(' ');
            fgc(&mut gs, palette.tx_dark);
            write!(gs, "{}{}", ICN_DELETED, git_info.deleted).unwrap();
            gs.push_str(RST);
        }
        if git_info.modified > 0 {
            gs.push(' ');
            fgc(&mut gs, palette.tx_dark);
            write!(gs, "{}{}", ICN_MODIFIED, git_info.modified).unwrap();
            gs.push_str(RST);
        }
        if gs.is_empty() {
            gs.push(' ');
            fgc(&mut gs, palette.tx_dark);
            gs.push('✓');
            gs.push_str(RST);
        }
        let mut git_text = String::with_capacity(96);
        git_text.push(' ');
        fgc(&mut git_text, palette.tx_dark);
        write!(git_text, "{} {}", ICN_GIT, git_info.branch).unwrap();
        git_text.push_str(RST);
        git_text.push_str(&gs);
        git_text.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Git(git_info_for_kind.clone()),
            text: git_text,
            bg: palette.bg_git,
            fg: palette.tx_dark,
            icon: ICN_GIT,
            priority: P_GIT,
        });
    }

    // 4.4 Context segment — priority 2 (critical).
    if flags.context {
        let bar = context_bar(ctx_pct as f64, BAR_WIDTH, palette.ctx_empty);
        let mut ctx_text = String::with_capacity(128);
        ctx_text.push(' ');
        fgc(&mut ctx_text, palette.tx_dark);
        ctx_text.push_str(ICN_CTX);
        ctx_text.push_str(RST);
        ctx_text.push(' ');
        ctx_text.push_str(&bar);
        ctx_text.push(' ');
        fgc(&mut ctx_text, palette.tx_dark);
        ctx_text.push_str(BOLD);
        write!(ctx_text, "{}%", ctx_pct).unwrap();
        ctx_text.push_str(RST);
        ctx_text.push(' ');
        fgc(&mut ctx_text, palette.tx_dark);
        if used_tokens > 0 {
            write!(ctx_text, "{}/{}", fmt_tokens(used_tokens), ctx_label).unwrap();
        } else {
            ctx_text.push_str(&ctx_label);
        }
        ctx_text.push_str(RST);
        ctx_text.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Context {
                pct: ctx_pct as f64,
                used_tokens: used_tokens as i64,
                size_label: ctx_label_for_kind.clone(),
            },
            text: ctx_text,
            bg: palette.bg_ctx,
            fg: palette.tx_dark,
            icon: ICN_CTX,
            priority: P_CTX,
        });
    }

    // 4.5 Rate limits (two segments) OR cost (one segment) — mutually exclusive.
    let show_limits = flags.limits && data.rate_limits.is_some();
    let show_cost = flags.cost && data.rate_limits.is_none() && cost_usd > 0.0;
    if show_limits {
        if let Some(pct) = block_pct {
            if pct >= LIMIT_SHOW_THRESHOLD {
                let mut t = String::with_capacity(48);
                t.push(' ');
                fgc(&mut t, palette.tx_white);
                write!(t, "{} 5h ", ICN_TIMER).unwrap();
                t.push_str(BOLD);
                write!(t, "{}%", pct).unwrap();
                t.push_str(RST);
                if !time_left.is_empty() {
                    t.push(' ');
                    fgc(&mut t, palette.tx_white);
                    t.push_str(&time_left);
                    t.push_str(RST);
                }
                t.push(' ');
                segments.push(SegmentRich {
                    kind: SegmentKind::Limit5h {
                        pct,
                        time_left: time_left_for_kind.clone(),
                    },
                    text: t,
                    bg: palette.bg_limit_5h,
                    fg: palette.tx_white,
                    icon: ICN_TIMER,
                    priority: P_LIMIT_5H,
                });
            }
        }
        if let Some(pct) = weekly_pct {
            if pct >= LIMIT_SHOW_THRESHOLD {
                let mut t = String::with_capacity(48);
                t.push(' ');
                fgc(&mut t, palette.tx_white);
                write!(t, "{} 7d ", ICN_CALENDAR).unwrap();
                t.push_str(BOLD);
                write!(t, "{}%", pct).unwrap();
                t.push_str(RST);
                if !weekly_left.is_empty() {
                    t.push(' ');
                    fgc(&mut t, palette.tx_white);
                    t.push_str(&weekly_left);
                    t.push_str(RST);
                }
                t.push(' ');
                segments.push(SegmentRich {
                    kind: SegmentKind::Limit7d {
                        pct,
                        time_left: weekly_left_for_kind.clone(),
                    },
                    text: t,
                    bg: palette.bg_limit_7d,
                    fg: palette.tx_white,
                    icon: ICN_CALENDAR,
                    priority: P_LIMIT_7D,
                });
            }
        }
    }
    if show_cost {
        let mut t = String::with_capacity(32);
        t.push(' ');
        fgc(&mut t, palette.tx_white);
        write!(t, "{} ${:.2}", ICN_COST, cost_usd).unwrap();
        t.push_str(RST);
        t.push(' ');
        segments.push(SegmentRich {
            kind: SegmentKind::Cost(cost_usd),
            text: t,
            bg: palette.bg_limit_7d,
            fg: palette.tx_white,
            icon: ICN_COST,
            priority: P_COST,
        });
    }

    // 5. Version suffix (fallback rebuild if too wide).
    let version_suffix = if flags.version && !data.version.is_empty() {
        let mut s = String::with_capacity(16);
        s.push(' ');
        fgc(&mut s, palette.tx_gray);
        s.push_str(DIM);
        write!(s, "v{}", data.version).unwrap();
        s.push_str(RST);
        s
    } else {
        String::new()
    };

    let skin = skins::resolve_skin(&settings.skin);
    let mut output = skin.render(&vec![segments.clone()], palette, cols, &version_suffix);
    if !version_suffix.is_empty() && display_width(&strip_ansi(&output)) > cols {
        output = skin.render(&vec![segments], palette, cols, "");
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

    #[test]
    fn render_without_git_segment() {
        let json = include_str!("../tests/fixtures/with_git.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.segments.git = false;
        let output = render_with(data, 200, s);
        assert!(!output.contains('\u{F062C}'), "git icon absent");
        assert!(output.contains('\u{F06A9}'), "model icon present");
    }

    #[test]
    fn render_with_dracula_uses_dracula_colors() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.theme = "dracula".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains("48;2;189;147;249"), "dracula bg_model present");
    }

    #[test]
    fn render_with_unknown_theme_falls_back() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.theme = "xyzzy".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains("48;2;154;52;142"), "fallback m365princess bg_model present");
    }

    #[test]
    fn render_with_minimal_skin_uses_middot() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "minimal".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains(" · "), "minimal skin uses · separator");
        assert!(!output.contains('\u{e0b0}'), "no powerline triangle");
    }

    #[test]
    fn render_with_pipe_skin_uses_pipe() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "pipe".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains(" | "), "pipe skin uses | separator");
    }

    #[test]
    fn render_with_rounded_skin_uses_caps() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "rounded".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains('\u{e0b6}'), "left cap present");
        assert!(output.contains('\u{e0b4}'), "right cap present");
    }

    #[test]
    fn render_with_rainbow_skin_has_prefix() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "rainbow".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains('\u{25a0}'), "rainbow prefix square present");
    }

    #[test]
    fn render_unknown_skin_falls_back_to_powerline() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "xyzzy".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains('\u{e0b0}'), "powerline triangle (fallback)");
    }

    #[test]
    fn matrix_dracula_minimal() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.theme = "dracula".to_string();
        s.skin = "minimal".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains(" · "));
        assert!(output.contains("38;2;189;147;249") || output.contains("48;2;189;147;249"),
            "dracula purple should appear as fg or bg");
    }

    #[test]
    fn bullet_skin_shows_gauge_dot_for_context() {
        let json = r#"{
            "context_window": {
                "used_percentage": 42.0,
                "context_window_size": 200000,
                "current_usage": {
                    "input_tokens": 50000,
                    "cache_creation_input_tokens": 0,
                    "cache_read_input_tokens": 0
                }
            }
        }"#;
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.skin = "bullet".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains('\u{25cf}'), "bullet dot for context");
        assert!(output.contains("42%"));
    }

    #[test]
    fn matrix_nord_rounded() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let data: ClaudeInput = serde_json::from_str(json).unwrap();
        let mut s = settings::Settings::default();
        s.theme = "nord".to_string();
        s.skin = "rounded".to_string();
        let output = render_with(data, 200, s);
        assert!(output.contains('\u{e0b6}'));
        assert!(output.contains("48;2;94;129;172"), "nord bg_model present");
    }
}
