#![allow(dead_code)]

pub const BAR_WIDTH: usize = 5;
pub const GIT_CACHE_TTL_MS: u64 = 5_000;
pub const LIMIT_SHOW_THRESHOLD: i64 = 0;
pub const GIT_CACHE_FILE: &str = "/tmp/.claude-statusline-git-cache.bin";

// Powerline separators
pub const PW: char = '\u{E0B0}';
pub const PW_THIN: char = '\u{E0B1}';

// RGB type
pub type Rgb = (u8, u8, u8);

// Segment background colors (M365Princess pastel palette)
pub const BG_TIME:     Rgb = (30,  30,  35);  // #1E1E23 near-black
pub const BG_MODEL:    Rgb = (154, 52,  142); // #9A348E plum
pub const BG_FOLDER:   Rgb = (218, 98,  125); // #DA627D blush
pub const BG_GIT:      Rgb = (252, 161, 125); // #FCA17D salmon
pub const BG_CTX:      Rgb = (134, 187, 216); // #86BBD8 sky
pub const BG_LIMIT_5H: Rgb = (91,  143, 176); // #5B8FB0 steel_blue clair
pub const BG_LIMIT_7D: Rgb = (51,  101, 138); // #33658A teal_blue

// Text colors
pub const TX_WHITE: Rgb = (255, 255, 255);
pub const TX_DARK:  Rgb = (40,  25,  55);
pub const TX_GRAY:  Rgb = (156, 163, 175);

// Empty slot colors for the context bar
pub const CTX_EMPTY: Rgb = (70, 110, 140);

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
