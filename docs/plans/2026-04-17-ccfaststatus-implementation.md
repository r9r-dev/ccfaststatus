# ccfaststatus Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Porter la statusline Claude Code (`~/.claude/statusline.mjs`) vers un binaire Rust natif, à l'identique visuellement, sans subprocess.

**Architecture:** Binaire CLI `ccfaststatus` lisant un payload JSON sur stdin et imprimant une ligne powerline ANSI sur stdout. Collecte parallèle via `std::thread` (git via `git2`, sessions via `sysinfo`, cols via `terminal_size`). Cache git binaire (`bincode`) dans `/tmp/.claude-statusline-git-cache.bin` avec TTL 5 s.

**Tech Stack:** Rust stable, `serde_json`, `git2` (vendored libgit2), `sysinfo`, `terminal_size`, `chrono`, `bincode`.

**Référence design :** `docs/plans/2026-04-17-ccfaststatus-design.md`

**Source à répliquer :** `/Users/rlamour/.claude/statusline.mjs` (326 lignes, lue intégralement avant d'écrire ce plan)

---

## Ordre des tâches

1. Scaffold Cargo + dépendances
2. Structs d'input JSON (serde)
3. Module config (couleurs, icônes, constantes)
4. Module term (ANSI helpers, strip_ansi, cols)
5. Module format (fmt_time, fmt_duration, fmt_tokens)
6. Barres (mini_bar, context_bar)
7. Module segments (builder + troncature)
8. Module sessions (sysinfo)
9. Module git (git2 + cache bincode)
10. Intégration `main.rs`
11. Fixtures golden + test snapshot
12. Build release + validation perf

---

## Task 1 : Scaffold Cargo

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

**Step 1 : Initialiser le projet Cargo**

Run : `cd /Users/rlamour/Developer/code/perso/ccfaststatus && cargo init --bin --name ccfaststatus`
Expected : `Creating binary (application) package`

**Step 2 : Remplacer le `Cargo.toml` par la config finale**

Contenu de `Cargo.toml` :

```toml
[package]
name = "ccfaststatus"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1.3"
git2 = { version = "0.19", default-features = false, features = ["vendored-libgit2"] }
sysinfo = { version = "0.32", default-features = false, features = ["system"] }
terminal_size = "0.4"
chrono = { version = "0.4", default-features = false, features = ["clock"] }

[profile.release]
lto = "fat"
codegen-units = 1
strip = true
opt-level = 3
panic = "abort"
```

**Step 3 : `.gitignore`**

Contenu :

```
/target
Cargo.lock.bak
```

(Note : on commit `Cargo.lock` car c'est un binaire.)

**Step 4 : Vérifier que le build passe**

Run : `cargo build`
Expected : `Compiling ccfaststatus ...` puis `Finished dev`. Le premier build compile libgit2 (peut prendre 1–2 min).

**Step 5 : Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore
git commit -m "feat: scaffold projet Rust avec dépendances natives"
```

---

## Task 2 : Structs d'input JSON

**Files:**
- Create: `src/input.rs`
- Modify: `src/main.rs` (ajouter `mod input;`)
- Test: dans `src/input.rs` (tests inline)

Le payload Claude Code contient (champs qu'on utilise, tous optionnels côté robustesse) :

```json
{
  "model": { "display_name": "Opus 4.7 (1M context)" },
  "version": "2.0.1",
  "cwd": "/path",
  "workspace": { "current_dir": "/path", "git_worktree": true },
  "context_window": {
    "used_percentage": 42.3,
    "context_window_size": 1000000,
    "current_usage": {
      "input_tokens": 123,
      "cache_creation_input_tokens": 456,
      "cache_read_input_tokens": 789
    }
  },
  "cost": { "total_cost_usd": 1.23, "total_duration_ms": 600000 },
  "rate_limits": {
    "five_hour": { "used_percentage": 42, "resets_at": 1745000000 },
    "seven_day": { "used_percentage": 10, "resets_at": 1745400000 }
  }
}
```

**Step 1 : Écrire le test d'abord**

Ajouter à `src/input.rs` :

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ClaudeInput {
    #[serde(default)]
    pub model: Model,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub workspace: Workspace,
    #[serde(default)]
    pub context_window: Option<ContextWindow>,
    #[serde(default)]
    pub cost: Option<Cost>,
    #[serde(default)]
    pub rate_limits: Option<RateLimits>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Model {
    #[serde(default)]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Workspace {
    #[serde(default)]
    pub current_dir: Option<String>,
    #[serde(default)]
    pub git_worktree: bool,
}

#[derive(Debug, Deserialize)]
pub struct ContextWindow {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub context_window_size: u64,
    #[serde(default)]
    pub current_usage: Option<CurrentUsage>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CurrentUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub total_cost_usd: f64,
    #[serde(default)]
    pub total_duration_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct RateLimits {
    #[serde(default)]
    pub five_hour: Option<RateLimit>,
    #[serde(default)]
    pub seven_day: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimit {
    #[serde(default)]
    pub used_percentage: f64,
    #[serde(default)]
    pub resets_at: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_payload() {
        let json = r#"{
            "model": {"display_name": "Opus 4.7 (1M context)"},
            "version": "2.0.1",
            "workspace": {"current_dir": "/tmp", "git_worktree": true},
            "context_window": {"used_percentage": 42.3, "context_window_size": 1000000},
            "cost": {"total_cost_usd": 1.23, "total_duration_ms": 600000},
            "rate_limits": {"five_hour": {"used_percentage": 42, "resets_at": 1745000000}}
        }"#;
        let parsed: ClaudeInput = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.model.display_name, "Opus 4.7 (1M context)");
        assert_eq!(parsed.version, "2.0.1");
        assert!(parsed.workspace.git_worktree);
        assert_eq!(parsed.context_window.unwrap().context_window_size, 1000000);
    }

    #[test]
    fn parses_empty_object() {
        let parsed: ClaudeInput = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed.version, "");
        assert!(parsed.context_window.is_none());
    }
}
```

**Step 2 : Ajouter `mod input;` à `src/main.rs`**

Contenu `src/main.rs` :

```rust
mod input;

fn main() {
    println!("TODO");
}
```

**Step 3 : Lancer les tests**

Run : `cargo test --lib input`
Expected : `2 passed`

**Step 4 : Commit**

```bash
git add src/input.rs src/main.rs
git commit -m "feat(input): structs serde pour le payload Claude Code"
```

---

## Task 3 : Module config

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (ajouter `mod config;`)

Constantes extraites fidèlement de `statusline.mjs:3-58`.

**Step 1 : Écrire `src/config.rs`**

```rust
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
pub const ICN_TIMER:     &str = "\u{F13CB}"; // nf-md-timer_sand (codepoint à vérifier)
pub const ICN_CALENDAR:  &str = "\u{F0ED0}"; // nf-md-calendar_clock (codepoint à vérifier)
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
```

**IMPORTANT** : ICN_TIMER (`󰔛`) et ICN_CALENDAR (`󰃰`) dans le JS sont des caractères littéraux Unicode dans le fichier source. Extraire les vrais codepoints avec :

```bash
python3 -c "print(hex(ord('\U000F0000')))"  # adapte selon le char copié
```

ou en Node : `'󰔛'.codePointAt(0).toString(16)`. Remplacer les TODO ci-dessus par les vrais codepoints avant commit.

**Step 2 : Ajouter `mod config;` à `src/main.rs`**

**Step 3 : Vérifier le build**

Run : `cargo build`
Expected : Finished dev

**Step 4 : Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat(config): palette, icônes et constantes de layout"
```

---

## Task 4 : Module term (ANSI + cols)

**Files:**
- Create: `src/term.rs`
- Modify: `src/main.rs` (ajouter `mod term;`)

**Step 1 : Écrire `src/term.rs` avec tests**

```rust
use crate::config::Rgb;
use std::fmt::Write;

pub const RST: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";

pub fn fgc(out: &mut String, (r, g, b): Rgb) {
    write!(out, "\x1b[38;2;{};{};{}m", r, g, b).unwrap();
}

pub fn bgc(out: &mut String, (r, g, b): Rgb) {
    write!(out, "\x1b[48;2;{};{};{}m", r, g, b).unwrap();
}

pub fn fgc_s(rgb: Rgb) -> String {
    let mut s = String::with_capacity(20);
    fgc(&mut s, rgb);
    s
}

pub fn bgc_s(rgb: Rgb) -> String {
    let mut s = String::with_capacity(20);
    bgc(&mut s, rgb);
    s
}

/// Retire toutes les séquences ANSI CSI (`\x1b[...m`).
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
            // push one full UTF-8 scalar
            let ch_len = utf8_char_width(bytes[i]);
            out.push_str(&s[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

fn utf8_char_width(b: u8) -> usize {
    if b < 0x80 { 1 }
    else if b < 0xC0 { 1 } // invalid, skip as 1
    else if b < 0xE0 { 2 }
    else if b < 0xF0 { 3 }
    else { 4 }
}

/// Nombre de colonnes du terminal. Fallback `COLUMNS` env puis 120.
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

/// Largeur d'affichage approximative (grapheme count) pour une chaîne déjà stripped.
/// On compte les scalars Unicode ; suffisant pour les Nerd Font et caractères powerline.
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
        assert_eq!(strip_ansi("\x1b[0m♥test"), "♥test");
    }

    #[test]
    fn display_width_counts_chars() {
        assert_eq!(display_width("hello"), 5);
        assert_eq!(display_width("♥"), 1);
    }
}
```

**Step 2 : Ajouter `mod term;` à `src/main.rs`**

**Step 3 : Lancer les tests**

Run : `cargo test --lib term`
Expected : `5 passed`

**Step 4 : Commit**

```bash
git add src/term.rs src/main.rs
git commit -m "feat(term): helpers ANSI, strip_ansi et détection cols"
```

---

## Task 5 : Module format (time/duration/tokens)

**Files:**
- Create: `src/format.rs`
- Modify: `src/main.rs`

Répliquer `fmtTime`, `fmtDuration`, `fmtTokens` (statusline.mjs:61-84).

**Step 1 : Écrire `src/format.rs`**

```rust
/// Formate une durée en ms en "Xh", "XhYm", "XdYh". Retourne "" si ≤ 0.
/// Référence JS: fmtTime (statusline.mjs:61)
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

/// Formate une durée de session en "XhYYm", "XmYYs", "Xs". Retourne "" si 0.
/// Référence JS: fmtDuration (statusline.mjs:69)
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

/// Formate un nombre de tokens: "1.2M", "42k", "999".
/// Référence JS: fmtTokens (statusline.mjs:79)
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
```

**Step 2 : Lancer les tests**

Run : `cargo test --lib format`
Expected : `11 passed`

**Step 3 : Commit**

```bash
git add src/format.rs src/main.rs
git commit -m "feat(format): fmt_time/duration/tokens avec tests unitaires"
```

---

## Task 6 : Barres (mini + context)

**Files:**
- Modify: `src/format.rs` (ajouter en fin de fichier)

Répliquer `miniBar` (statusline.mjs:87) et `contextBar` (statusline.mjs:93).

**Step 1 : Étendre `src/format.rs`**

Ajouter en haut :

```rust
use crate::config::{Rgb, BAR_WIDTH};
use crate::term::{fgc, RST};
```

Puis :

```rust
/// Barre simple box-drawing "━━━━━━" / "──────".
/// Référence JS: miniBar (statusline.mjs:87)
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

/// Barre braille verticale 8 niveaux, largeur fixe en caractères.
/// Référence JS: contextBar (statusline.mjs:93)
pub fn context_bar(pct: f64, width: usize, empty_rgb: Rgb) -> String {
    // Ordre de remplissage vertical (du bas vers le haut alternativement).
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
mod tests_bars {
    use super::*;

    #[test]
    fn mini_bar_zero_percent() {
        let s = mini_bar(0.0, (70, 110, 140));
        // Toute la barre doit être des '─' (empty) sans '━'.
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
        // Round(2.5) = 3 en Rust; avec BAR_WIDTH=5 → 3 pleins, 2 vides.
        assert_eq!(s.matches('━').count(), 3);
        assert_eq!(s.matches('─').count(), 2);
    }

    #[test]
    fn context_bar_zero_pct() {
        let s = context_bar(0.0, 5, (70, 110, 140));
        // Tous braille blank (U+2800)
        assert_eq!(s.matches('\u{2800}').count(), 5);
    }

    #[test]
    fn context_bar_full() {
        let s = context_bar(100.0, 5, (70, 110, 140));
        // Tous braille full (U+28FF)
        assert_eq!(s.matches('\u{28FF}').count(), 5);
    }
}
```

**ATTENTION** : l'arrondi round() en Rust (banker's rounding IEEE-754) diffère de `Math.round()` en JS (round half away from zero). Vérifier avec une fixture et un payload à 50 % si la sortie diverge ; si oui, remplacer `.round()` par `(x + 0.5).floor()` pour matcher le JS.

**Step 2 : Lancer les tests**

Run : `cargo test --lib format`
Expected : `16 passed` (11 + 5)

**Step 3 : Commit**

```bash
git add src/format.rs
git commit -m "feat(format): mini_bar et context_bar (braille 8 niveaux)"
```

---

## Task 7 : Module segments

**Files:**
- Create: `src/segments.rs`
- Modify: `src/main.rs`

Répliquer `buildPowerline` + `renderSegments` (statusline.mjs:114-147).

**Step 1 : Écrire `src/segments.rs`**

```rust
use crate::config::{Rgb, PW};
use crate::term::{bgc, display_width, fgc, strip_ansi, RST};
use std::fmt::Write;

pub struct Segment {
    pub text: String,  // peut contenir des séquences ANSI (notamment RST)
    pub bg: Rgb,
    pub priority: u8,  // lower = plus important
}

/// Rend une suite de segments avec séparateurs powerline.
/// Référence JS: renderSegments (statusline.mjs:132)
fn render(segments: &[Segment]) -> String {
    let mut line = String::with_capacity(512);
    for (i, seg) in segments.iter().enumerate() {
        let mut bg_str = String::with_capacity(20);
        bgc(&mut bg_str, seg.bg);

        // Réinjecter le bg après chaque RST à l'intérieur du texte
        let fixed = seg.text.replace(RST, &format!("{}{}", RST, bg_str));

        line.push_str(&bg_str);
        line.push_str(&fixed);

        if let Some(next) = segments.get(i + 1) {
            // Transition: RST → fg(current_bg) + bg(next_bg) + triangle
            line.push_str(RST);
            fgc(&mut line, seg.bg);
            bgc(&mut line, next.bg);
            line.push(PW);
        } else {
            // Dernière: RST → fg(current_bg) + triangle + RST
            line.push_str(RST);
            fgc(&mut line, seg.bg);
            line.push(PW);
            line.push_str(RST);
        }
    }
    line
}

/// Construit la ligne powerline en retirant les segments les moins prioritaires
/// (plus grand `priority`) tant qu'elle dépasse `cols` colonnes.
/// Référence JS: buildPowerline (statusline.mjs:114)
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

        // Retirer le segment avec la plus grande priority (le moins important)
        let worst_idx = candidates
            .iter()
            .enumerate()
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
        // cols très étroit force la troncature
        let out = build_powerline(segments, "", 15);
        assert!(!out.contains("VERSION"), "VERSION (p7) devrait sortir en premier");
        assert!(out.contains("MODEL"), "MODEL (p1) doit toujours être là");
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
}
```

**Step 2 : Ajouter `mod segments;` à `src/main.rs`**

**Step 3 : Lancer les tests**

Run : `cargo test --lib segments`
Expected : `4 passed`

**Step 4 : Commit**

```bash
git add src/segments.rs src/main.rs
git commit -m "feat(segments): builder powerline avec troncature par priorité"
```

---

## Task 8 : Module sessions (sysinfo)

**Files:**
- Create: `src/sessions.rs`
- Modify: `src/main.rs`

Répliquer `getActiveSessions` (statusline.mjs:163).

**Step 1 : Écrire `src/sessions.rs`**

```rust
use sysinfo::{ProcessesToUpdate, System};

/// Compte les processus dont le nom d'exécutable est exactement "claude".
/// Équivalent natif de `ps -Ao comm | grep ^claude$`.
/// Référence JS: getActiveSessions (statusline.mjs:163)
pub fn count() -> usize {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes()
        .values()
        .filter(|p| p.name().to_str() == Some("claude"))
        .count()
}
```

**Step 2 : Ajouter `mod sessions;` à `src/main.rs`**

**Step 3 : Validation manuelle**

Dans un terminal avec Claude Code ouvert :

Run : `cargo run --release 2>/dev/null <<< '{}'`

Pas de crash attendu. Valider que le module compile via :

Run : `cargo build`
Expected : Finished dev

**Step 4 : Commit**

```bash
git add src/sessions.rs src/main.rs
git commit -m "feat(sessions): comptage des processus claude via sysinfo"
```

---

## Task 9 : Module git (git2 + cache bincode)

**Files:**
- Create: `src/git.rs`
- Modify: `src/main.rs`

Répliquer `getGitInfo` + cache (statusline.mjs:150-206).

**Step 1 : Écrire `src/git.rs`**

```rust
use crate::config::{GIT_CACHE_FILE, GIT_CACHE_TTL_MS};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub cwd: String,
    pub is_repo: bool,
    pub branch: String,
    pub ahead: u32,
    pub added: u32,
    pub deleted: u32,
    pub modified: u32,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    ts_ms: u64,
    data: GitInfo,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn read_cache(path: &str, cwd: &str, ttl_ms: u64) -> Option<GitInfo> {
    let bytes = fs::read(path).ok()?;
    let entry: CacheEntry = bincode::deserialize(&bytes).ok()?;
    if now_ms().saturating_sub(entry.ts_ms) >= ttl_ms {
        return None;
    }
    if entry.data.cwd != cwd {
        return None;
    }
    Some(entry.data)
}

fn write_cache(path: &str, data: &GitInfo) {
    let entry = CacheEntry { ts_ms: now_ms(), data: data.clone() };
    if let Ok(bytes) = bincode::serialize(&entry) {
        let _ = fs::write(path, bytes);
    }
}

fn collect_fresh(cwd: &Path) -> GitInfo {
    let mut info = GitInfo {
        cwd: cwd.to_string_lossy().into_owned(),
        ..Default::default()
    };

    let repo = match Repository::discover(cwd) {
        Ok(r) => r,
        Err(_) => return info,
    };
    info.is_repo = true;

    // Branch name
    info.branch = match repo.head() {
        Ok(h) => h.shorthand().unwrap_or("HEAD").to_string(),
        Err(_) => "HEAD".to_string(),
    };

    // Ahead count vs upstream
    if let Ok(head) = repo.head() {
        if let Some(local_oid) = head.target() {
            if let Ok(branch_name) = repo.branch_upstream_name(head.name().unwrap_or("")) {
                if let Some(upstream_ref) = branch_name.as_str() {
                    if let Ok(upstream_ref_obj) = repo.find_reference(upstream_ref) {
                        if let Some(upstream_oid) = upstream_ref_obj.target() {
                            if let Ok((ahead, _behind)) = repo.graph_ahead_behind(local_oid, upstream_oid) {
                                info.ahead = ahead as u32;
                            }
                        }
                    }
                }
            }
        }
    }

    // Status counts
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        for s in statuses.iter() {
            let flags = s.status();
            // Une entrée = un fichier. On incrémente selon la catégorie dominante,
            // cohérent avec la logique JS (xy[0] ou xy[1] matching).
            if flags.contains(Status::WT_NEW) || flags.contains(Status::INDEX_NEW) {
                info.added += 1;
            } else if flags.contains(Status::WT_DELETED) || flags.contains(Status::INDEX_DELETED) {
                info.deleted += 1;
            } else if flags.contains(Status::WT_MODIFIED)
                || flags.contains(Status::INDEX_MODIFIED)
                || flags.contains(Status::WT_RENAMED)
                || flags.contains(Status::INDEX_RENAMED)
            {
                info.modified += 1;
            }
        }
    }

    info
}

/// Retourne les infos git pour `cwd`, utilise un cache fichier si frais.
/// Référence JS: getGitInfo (statusline.mjs:173)
pub fn info(cwd: &str) -> GitInfo {
    if let Some(cached) = read_cache(GIT_CACHE_FILE, cwd, GIT_CACHE_TTL_MS) {
        return cached;
    }
    let fresh = collect_fresh(Path::new(cwd));
    write_cache(GIT_CACHE_FILE, &fresh);
    fresh
}
```

**Step 2 : Ajouter `mod git;` à `src/main.rs`**

**Step 3 : Validation manuelle**

Run : `cargo build`
Expected : Finished dev (le premier build de git2 prend du temps la première fois).

**Step 4 : Commit**

```bash
git add src/git.rs src/main.rs
git commit -m "feat(git): infos git via git2 avec cache bincode"
```

---

## Task 10 : Intégration `main.rs`

**Files:**
- Modify: `src/main.rs`

Assemble tout. Flow : read stdin → parse → collecte parallèle → build segments → render → print.

**Step 1 : Réécrire `src/main.rs`**

```rust
mod config;
mod format;
mod git;
mod input;
mod segments;
mod sessions;
mod term;

use std::fmt::Write as _;
use std::io::Read;
use std::thread;

use chrono::Local;

use config::*;
use format::*;
use input::ClaudeInput;
use segments::{build_powerline, Segment};
use term::*;

fn main() {
    // 1. Read stdin
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        return;
    }

    let data: ClaudeInput = match serde_json::from_str(&buf) {
        Ok(d) => d,
        Err(_) => return,
    };

    let cwd = data
        .workspace
        .current_dir
        .clone()
        .unwrap_or_else(|| data.cwd.clone());

    // 2. Collecte parallèle
    let cwd_git = cwd.clone();
    let git_handle = thread::spawn(move || git::info(&cwd_git));
    let sessions_handle = thread::spawn(sessions::count);
    let cols = get_cols();

    let git = git_handle.join().unwrap_or_default();
    let sessions_count = sessions_handle.join().unwrap_or(0);

    // 3. Préparations
    let model = shorten_model(&data.model.display_name);
    let folder = cwd.rsplit('/').next().unwrap_or(&cwd).to_string();
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

    // 4. Build segments
    let mut segments: Vec<Segment> = Vec::with_capacity(8);

    // Heure + sessions + durée
    let now = Local::now();
    let mut time_text = String::with_capacity(64);
    time_text.push(' ');
    fgc(&mut time_text, TX_WHITE);
    time_text.push_str(BOLD);
    write!(time_text, "{} {:02}:{:02}", ICN_HEART, now.hour12_24(), now.minute()).unwrap();
    time_text.push_str(RST);
    if sessions_count > 1 {
        time_text.push(' ');
        fgc(&mut time_text, TX_GRAY);
        write!(time_text, "{}{}", ICN_SESSIONS, sessions_count).unwrap();
        time_text.push_str(RST);
    }
    let dur = fmt_duration(session_ms);
    if !dur.is_empty() {
        time_text.push(' ');
        fgc(&mut time_text, TX_GRAY);
        time_text.push_str(&dur);
        time_text.push_str(RST);
    }
    time_text.push(' ');
    segments.push(Segment { text: time_text, bg: BG_TIME, priority: P_TIME });

    // Model
    let mut model_text = String::with_capacity(32);
    model_text.push(' ');
    fgc(&mut model_text, TX_WHITE);
    model_text.push_str(BOLD);
    write!(model_text, "{} {}", ICN_MODEL, model).unwrap();
    model_text.push_str(RST);
    model_text.push(' ');
    segments.push(Segment { text: model_text, bg: BG_MODEL, priority: P_MODEL });

    // Folder + worktree
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

    // Git
    if git.is_repo {
        let mut gs = String::with_capacity(64);
        if git.ahead > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            gs.push_str(BOLD);
            write!(gs, "{}{}", ICN_AHEAD, git.ahead).unwrap();
            gs.push_str(RST);
        }
        if git.added > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_ADDED, git.added).unwrap();
            gs.push_str(RST);
        }
        if git.deleted > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_DELETED, git.deleted).unwrap();
            gs.push_str(RST);
        }
        if git.modified > 0 {
            gs.push(' ');
            fgc(&mut gs, TX_DARK);
            write!(gs, "{}{}", ICN_MODIFIED, git.modified).unwrap();
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
        write!(git_text, "{} {}", ICN_GIT, git.branch).unwrap();
        git_text.push_str(RST);
        git_text.push_str(&gs);
        git_text.push(' ');
        segments.push(Segment { text: git_text, bg: BG_GIT, priority: P_GIT });
    }

    // Context
    let bar = context_bar(ctx_pct as f64, BAR_WIDTH, CTX_EMPTY);
    let tok_str = if used_tokens > 0 {
        let mut s = String::with_capacity(32);
        fgc(&mut s, TX_DARK);
        s.push_str(BOLD);
        write!(s, "{}%", ctx_pct).unwrap();
        s.push_str(RST);
        s.push(' ');
        fgc(&mut s, TX_DARK);
        write!(s, "{}/{}", fmt_tokens(used_tokens), ctx_label).unwrap();
        s.push_str(RST);
        s
    } else {
        let mut s = String::with_capacity(24);
        fgc(&mut s, TX_DARK);
        s.push_str(BOLD);
        write!(s, "{}%", ctx_pct).unwrap();
        s.push_str(RST);
        s.push(' ');
        fgc(&mut s, TX_DARK);
        s.push_str(&ctx_label);
        s.push_str(RST);
        s
    };

    let mut ctx_text = String::with_capacity(128);
    ctx_text.push(' ');
    fgc(&mut ctx_text, TX_DARK);
    ctx_text.push_str(ICN_CTX);
    ctx_text.push_str(RST);
    ctx_text.push(' ');
    ctx_text.push_str(&bar);
    ctx_text.push(' ');
    ctx_text.push_str(&tok_str);
    ctx_text.push(' ');
    segments.push(Segment { text: ctx_text, bg: BG_CTX, priority: P_CTX });

    // Rate limits ou cost
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

    // Version suffix
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

    // 5. Build powerline
    let mut output = build_powerline(segments.clone(), &version_suffix, cols);
    // Si le suffix version a été inclus mais dépasse encore, refaire sans suffix.
    if !version_suffix.is_empty() && display_width(&strip_ansi(&output)) > cols {
        output = build_powerline(segments, "", cols);
    }

    println!("{}", output);
}

/// "Opus 4.7 (1M context)" → "Opus"
/// Référence JS: statusline.mjs:215
fn shorten_model(raw: &str) -> String {
    // Retire "(.*)" puis le nombre de version final (4.7 ou 4)
    let re_paren = regex_lite_strip_parens(raw);
    trim_trailing_version(&re_paren)
}

fn regex_lite_strip_parens(s: &str) -> String {
    // Remplace /\s*\(.*\)/ par "". On coupe à la première '(' et strippe le reste.
    if let Some(pos) = s.find('(') {
        s[..pos].trim_end().to_string()
    } else {
        s.to_string()
    }
}

fn trim_trailing_version(s: &str) -> String {
    // Retire "\s+\d+(\.\d+)?$"
    let chars: Vec<char> = s.chars().collect();
    let mut end = chars.len();
    // mange chiffres
    while end > 0 && chars[end - 1].is_ascii_digit() {
        end -= 1;
    }
    // optionnellement un '.' suivi d'autres chiffres (déjà consommés ci-dessus si présent en suffix)
    if end > 0 && chars[end - 1] == '.' {
        end -= 1;
        while end > 0 && chars[end - 1].is_ascii_digit() {
            end -= 1;
        }
    }
    // espace(s) obligatoire(s) avant
    let mut trimmed_end = end;
    while trimmed_end > 0 && chars[trimmed_end - 1].is_whitespace() {
        trimmed_end -= 1;
    }
    if trimmed_end < end {
        chars[..trimmed_end].iter().collect()
    } else {
        s.to_string()
    }
}

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

use std::time::{SystemTime, UNIX_EPOCH};

trait TimeExt {
    fn hour12_24(&self) -> u32;
}
impl<T: chrono::Timelike> TimeExt for T {
    fn hour12_24(&self) -> u32 {
        self.hour()
    }
}
```

**NOTE IMPORTANTE** : `Segment` doit dériver `Clone` (ajouter `#[derive(Clone)]` dans `src/segments.rs`) pour permettre `build_powerline(segments.clone(), ...)`.

**Step 2 : Ajouter `#[derive(Clone)]` à `Segment`**

Modifier `src/segments.rs` :

```rust
#[derive(Clone)]
pub struct Segment {
    pub text: String,
    pub bg: Rgb,
    pub priority: u8,
}
```

**Step 3 : Build**

Run : `cargo build --release`
Expected : Finished release

**Step 4 : Test manuel**

```bash
echo '{"model":{"display_name":"Opus 4.7 (1M context)"},"version":"2.0.1","workspace":{"current_dir":"/Users/rlamour/Developer/code/perso/ccfaststatus"},"context_window":{"used_percentage":42.3,"context_window_size":1000000,"current_usage":{"input_tokens":100000}}}' | ./target/release/ccfaststatus
```

Expected : une ligne powerline colorée visible avec l'heure, "Opus", "ccfaststatus", info git, barre contexte.

**Step 5 : Commit**

```bash
git add src/main.rs src/segments.rs
git commit -m "feat(main): orchestration et assemblage des segments"
```

---

## Task 11 : Fixtures golden + test snapshot

**Files:**
- Create: `tests/fixtures/minimal.json`
- Create: `tests/fixtures/minimal.expected`
- Create: `tests/fixtures/with_git.json`
- Create: `tests/fixtures/with_git.expected`
- Create: `tests/fixtures/rate_limits.json`
- Create: `tests/fixtures/rate_limits.expected`
- Create: `tests/fixtures/narrow_80cols.json`
- Create: `tests/fixtures/narrow_80cols.expected`
- Create: `tests/golden.rs`

**Step 1 : Générer les fixtures JSON depuis le script JS**

Pour chaque cas, préparer un payload JSON dans `tests/fixtures/<name>.json` puis :

```bash
cat tests/fixtures/minimal.json | COLUMNS=200 node /Users/rlamour/.claude/statusline.mjs > tests/fixtures/minimal.expected
```

(Le script JS utilise `COLUMNS` si stty échoue ; on force `COLUMNS=200` pour désactiver la troncature sur les cas non-narrow.)

Exemple `tests/fixtures/minimal.json` :

```json
{
  "model": {"display_name": "Opus 4.7 (1M context)"},
  "version": "2.0.1",
  "workspace": {"current_dir": "/tmp"},
  "context_window": {"used_percentage": 10, "context_window_size": 200000}
}
```

Exemple `tests/fixtures/narrow_80cols.json` : idem minimal mais on génèrera le expected avec `COLUMNS=80`.

**CRITIQUE** : l'heure et la durée changent à chaque run ; il faut figer le temps côté test. Deux options :
- **(a)** Nettoyer les parties variables (heure, durée, `time_left`) des deux côtés avant diff — approche pragmatique.
- **(b)** Passer un `CCFASTSTATUS_FAKE_NOW_MS` env var côté Rust uniquement pour les tests. Hors scope v1.

**Option retenue : (a)**. Écrire une fonction `normalize(line)` dans le test qui remplace `\d{2}:\d{2}` et `\d+[hms]\d*[ms]?s?` par `XX:XX` et `DUR`, et la ligne `time_left` par `TLEFT`.

**Step 2 : Écrire `tests/golden.rs`**

```rust
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::Write;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn normalize(s: &str) -> String {
    // Masque les parties dépendantes du temps.
    let re_time = regex_find_replace(s, &[
        // HH:MM en format 00:00 → 23:59
        (r"\b\d{2}:\d{2}\b", "XX:XX"),
    ]);
    let re_dur = regex_find_replace(&re_time, &[
        (r"\d+h\d{2}m", "DURh"),
        (r"\d+m\d{2}s", "DURm"),
        (r"\b\d+s\b", "DURs"),
    ]);
    re_dur
}

// Implémentation "regex" maison (replace all literal patterns).
// Si regex-lite ou regex est nécessaire, ajouter `regex = "1"` en [dev-dependencies].
// Plus simple: utiliser `regex` crate en dev-dep.

fn run_bin(fixture_json: &str, cols: usize) -> String {
    let bin = env!("CARGO_BIN_EXE_ccfaststatus");
    let mut child = Command::new(bin)
        .env("COLUMNS", cols.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ccfaststatus");
    child.stdin.as_mut().unwrap().write_all(fixture_json.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim_end_matches('\n').to_string()
}

fn check(name: &str, cols: usize) {
    let json = std::fs::read_to_string(fixture_path(&format!("{}.json", name))).unwrap();
    let expected = std::fs::read_to_string(fixture_path(&format!("{}.expected", name))).unwrap();
    let actual = run_bin(&json, cols);
    let n_expected = normalize(expected.trim_end_matches('\n'));
    let n_actual = normalize(&actual);
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(fixture_path(&format!("{}.expected", name)), &actual).unwrap();
        return;
    }
    assert_eq!(n_actual, n_expected, "snapshot mismatch for {}", name);
}

#[test] fn minimal() { check("minimal", 200); }
#[test] fn with_git() { check("with_git", 200); }
#[test] fn rate_limits() { check("rate_limits", 200); }
#[test] fn narrow_80cols() { check("narrow_80cols", 80); }
```

**Étape 2.1** : Ajouter `regex = "1"` dans `[dev-dependencies]` de `Cargo.toml` et utiliser `regex::Regex::new(pat).unwrap().replace_all(s, repl).into_owned()` à la place de `regex_find_replace`.

**Step 3 : Génération et validation**

```bash
# Régénérer les .expected avec le binaire Rust (une fois qu'il est considéré correct)
UPDATE_GOLDEN=1 cargo test --test golden

# Comparer visuellement chaque .expected au résultat du script JS sur la même fixture
for f in tests/fixtures/*.json; do
  base="${f%.json}"
  echo "=== $(basename $base) ==="
  diff <(cat "$f" | COLUMNS=200 node /Users/rlamour/.claude/statusline.mjs) "$base.expected" || true
done
```

Tant que le diff n'est pas vide (hors parties temporelles), **revenir sur les tâches 5-10** jusqu'à match byte-à-byte.

**Step 4 : Commit**

```bash
git add tests/ Cargo.toml Cargo.lock
git commit -m "test(golden): fixtures et snapshot contre le script JS de référence"
```

---

## Task 12 : Build release + validation perf

**Files:**
- Create: `scripts/bench.sh` (optionnel)

**Step 1 : Build release final**

Run : `cargo build --release`
Expected : Finished `release`, binaire à `target/release/ccfaststatus`.

**Step 2 : Mesurer la taille du binaire**

Run : `ls -lh target/release/ccfaststatus`
Expected : ~2–3 Mo.

**Step 3 : Benchmarker le startup**

```bash
cat > /tmp/payload.json <<'EOF'
{"model":{"display_name":"Opus 4.7 (1M context)"},"version":"2.0.1","workspace":{"current_dir":"/Users/rlamour/Developer/code/perso/ccfaststatus"},"context_window":{"used_percentage":42.3,"context_window_size":1000000,"current_usage":{"input_tokens":100000}},"cost":{"total_cost_usd":0.42,"total_duration_ms":600000}}
EOF

# Préchauffer le cache git
cat /tmp/payload.json | ./target/release/ccfaststatus > /dev/null

# Mesurer 20 runs
for i in {1..20}; do
  /usr/bin/time -p sh -c "cat /tmp/payload.json | ./target/release/ccfaststatus > /dev/null" 2>&1 | grep real
done
```

Expected : `real 0.00X` avec X entre 0.005 et 0.015 sur macOS (cache chaud).

**Step 4 : Comparaison visuelle finale**

```bash
diff <(cat /tmp/payload.json | node /Users/rlamour/.claude/statusline.mjs) \
     <(cat /tmp/payload.json | ./target/release/ccfaststatus)
```

Expected : diff vide (hors parties temporelles) sur tous les cas.

**Step 5 : Documenter dans README.md**

Créer `README.md` minimal :

```markdown
# ccfaststatus

Statusline Claude Code réécrite en Rust natif. Compatible drop-in avec la config
`statusLine.command` de `~/.claude/settings.json`.

## Build

    cargo build --release

## Installation

    ln -sf $PWD/target/release/ccfaststatus ~/.local/bin/ccfaststatus

Puis dans `~/.claude/settings.json` :

    "statusLine": {
      "type": "command",
      "command": "ccfaststatus",
      "refreshInterval": 3
    }
```

**Step 6 : Commit**

```bash
git add README.md
git commit -m "docs: README avec instructions build et install"
```

---

## Récapitulatif

| # | Tâche | LOC cible | TDD |
|---|-------|-----------|-----|
| 1 | Scaffold | — | ❌ |
| 2 | Input structs | ~80 | ✅ |
| 3 | Config | ~60 | ❌ |
| 4 | Term | ~80 | ✅ |
| 5 | Format | ~70 | ✅ |
| 6 | Bars | ~60 | ✅ |
| 7 | Segments | ~90 | ✅ |
| 8 | Sessions | ~15 | ❌ |
| 9 | Git + cache | ~110 | ❌ |
| 10 | Main | ~220 | ⚠️ (golden) |
| 11 | Fixtures golden | — | ✅ |
| 12 | Release + bench | — | ❌ |

**Total estimé** : ~800 LOC + tests.

**Commits attendus** : ~12 (un par tâche).

**Validation finale** : diff byte-à-byte Rust vs JS sur 4 fixtures + bench startup < 15 ms cache chaud.
