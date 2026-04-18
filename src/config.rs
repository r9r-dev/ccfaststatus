#![allow(dead_code)]

pub const BAR_WIDTH: usize = 5;
pub const GIT_CACHE_TTL_MS: u64 = 5_000;
pub const LIMIT_SHOW_THRESHOLD: i64 = 0;
pub const GIT_CACHE_FILE: &str = "/tmp/.claude-statusline-git-cache.bin";

// Powerline separators
pub const PW: char = '\u{E0B0}';
pub const PW_THIN: char = '\u{E0B1}';

// RGB type (kept for Segment.bg, used across config/theme/skins)
pub type Rgb = (u8, u8, u8);

// Note: couleurs déplacées dans src/theme.rs (Theme::M365PRINCESS).

// Icons (Nerd Font nf-md-*)
pub const ICN_HEART:     &str = "♥";
pub const ICN_MODEL:     &str = "\u{F06A9}"; // nf-md-robot
pub const ICN_FOLDER:    &str = "\u{F024B}"; // nf-md-folder
pub const ICN_GIT:       &str = "\u{F062C}"; // nf-md-source_branch
pub const ICN_CTX:       &str = "\u{F035B}"; // nf-md-memory
pub const ICN_AHEAD:     &str = "\u{F005D}"; // nf-md-arrow_up_bold
pub const ICN_ADDED:     &str = "\u{F0752}"; // nf-md-file_plus
pub const ICN_DELETED:   &str = "\u{F0754}"; // nf-md-file_minus
pub const ICN_MODIFIED:  &str = "\u{F0224}"; // nf-md-file_document_edit
pub const ICN_TIMER:     &str = "\u{F051B}"; // nf-md-timer_sand
pub const ICN_CALENDAR:  &str = "\u{F00F0}"; // nf-md-calendar_clock
pub const ICN_COST:      &str = "\u{F01C1}"; // nf-md-currency_usd
pub const ICN_WORKTREE:  &str = "\u{F0C7E}"; // nf-md-source_branch_plus
pub const ICN_SESSIONS:  &str = "⧉";         // U+29C9

// Priorités (lower = plus important, jamais retirées en premier)
pub const P_MODEL: u8 = 1;
pub const P_CTX: u8 = 2;
pub const P_GIT: u8 = 3;
pub const P_FOLDER: u8 = 4;
pub const P_TIME: u8 = 5;
pub const P_LIMIT_5H: u8 = 6;
pub const P_COST: u8 = 6;
pub const P_LIMIT_7D: u8 = 7;
pub const P_VERSION: u8 = 7;
