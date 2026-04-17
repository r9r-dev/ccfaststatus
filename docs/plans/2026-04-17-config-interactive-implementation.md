# Configuration interactive — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ajouter à ccfaststatus un mode install interactif déclenché en TTY (quand l'utilisateur lance `ccfaststatus` dans un terminal) qui configure `~/.claude/settings.json` en deux prompts.

**Architecture:** Entrée unique via `main()`. Un check `stdin().is_terminal()` tout en début de main bascule entre mode statusline normal (pipe Claude Code) et mode install (TTY). Nouveau module `src/install.rs` gère la localisation, read/parse/mutate/write du JSON, les prompts stdin, la vérification round-trip et l'aperçu visuel.

**Tech Stack:** Rust (stable), `serde_json` (déjà présent), `std::io::IsTerminal` (stdlib), `std::env` (HOME), pas de nouvelle dépendance.

**Référence design :** `docs/plans/2026-04-17-config-interactive-design.md`

---

## Task 1 — Skeleton du module `install`

**Files:**
- Create: `src/install.rs`
- Modify: `src/main.rs:1-7` (ajouter `mod install;`)
- Modify: `src/main.rs:29-35` (check TTY en tête de `main()`)

**Step 1: Créer le stub**

```rust
// src/install.rs
pub fn run() {
    println!("install stub");
}
```

**Step 2: Câbler dans main**

Modifier `src/main.rs` :

```rust
mod config;
mod format;
mod git;
mod input;
mod install;      // NEW
mod segments;
mod sessions;
mod term;

use std::io::{IsTerminal, Read};   // ajoute IsTerminal
```

Puis tout au début de `main()` :

```rust
fn main() {
    if std::io::stdin().is_terminal() {
        install::run();
        return;
    }
    // ... reste inchangé
```

**Step 3: Build + smoke test**

```bash
cargo build --release
./target/release/ccfaststatus
```

Expected: `install stub`

```bash
echo '{}' | ./target/release/ccfaststatus
```

Expected: statusline normale (pas `install stub`)

**Step 4: Commit**

```bash
git add src/main.rs src/install.rs
git commit -m "feat(install): skeleton + TTY branch"
```

---

## Task 2 — Fonction pure `update_settings` (TDD)

**Files:**
- Modify: `src/install.rs`

**Step 1: Écrire les tests**

Ajouter en bas de `src/install.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn adds_statusline_when_missing() {
        let input = json!({"otherKey": "preserved"});
        let out = update_settings(input, 2);
        assert_eq!(out["otherKey"], "preserved");
        assert_eq!(out["statusLine"]["type"], "command");
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 2);
    }

    #[test]
    fn overwrites_foreign_statusline() {
        let input = json!({
            "statusLine": {"type": "command", "command": "/old/path", "refreshInterval": 5}
        });
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
    }

    #[test]
    fn updates_only_interval_when_already_ccfaststatus() {
        let input = json!({
            "statusLine": {"type": "command", "command": "ccfaststatus", "refreshInterval": 3},
            "theme": "dark"
        });
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
        assert_eq!(out["theme"], "dark");
    }

    #[test]
    fn handles_empty_object() {
        let input = json!({});
        let out = update_settings(input, 1);
        assert_eq!(out["statusLine"]["command"], "ccfaststatus");
        assert_eq!(out["statusLine"]["refreshInterval"], 1);
    }
}
```

**Step 2: Lancer, vérifier l'échec**

```bash
cargo test --lib install::tests 2>&1 | tail -15
```

Expected: FAIL avec `cannot find function 'update_settings'`

**Step 3: Implémenter**

```rust
use serde_json::{json, Value};

pub fn is_already_configured(settings: &Value) -> bool {
    settings
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        == Some("ccfaststatus")
}

pub fn update_settings(mut settings: Value, interval: u32) -> Value {
    let obj = settings.as_object_mut().expect("settings must be a JSON object");
    obj.insert(
        "statusLine".to_string(),
        json!({
            "type": "command",
            "command": "ccfaststatus",
            "refreshInterval": interval
        }),
    );
    settings
}
```

**Step 4: Vérifier les tests passent**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: 4 passed

**Step 5: Commit**

```bash
git add src/install.rs
git commit -m "feat(install): pure update_settings + is_already_configured"
```

---

## Task 3 — `settings_path()` + `read_settings()`

**Files:**
- Modify: `src/install.rs`

**Step 1: Tests**

Ajouter au module tests :

```rust
#[test]
fn read_missing_returns_empty_object() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-missing.json");
    let _ = std::fs::remove_file(&tmp);
    let v = read_settings(&tmp).expect("should not fail on missing");
    assert!(v.is_object());
    assert_eq!(v.as_object().unwrap().len(), 0);
}

#[test]
fn read_valid_json_parses() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-valid.json");
    std::fs::write(&tmp, r#"{"theme":"dark"}"#).unwrap();
    let v = read_settings(&tmp).unwrap();
    assert_eq!(v["theme"], "dark");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn read_invalid_json_errors() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-invalid.json");
    std::fs::write(&tmp, "{not json}").unwrap();
    let r = read_settings(&tmp);
    assert!(r.is_err());
    let _ = std::fs::remove_file(&tmp);
}
```

**Step 2: Vérifier l'échec**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: FAIL, `read_settings` inconnu.

**Step 3: Implémenter**

```rust
use std::path::{Path, PathBuf};
use std::fs;

pub fn settings_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(Path::new(&home).join(".claude").join("settings.json"))
}

pub fn read_settings(path: &Path) -> Result<Value, String> {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s)
            .map_err(|e| format!("{} is not valid JSON: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(format!("failed to read {}: {}", path.display(), e)),
    }
}
```

**Step 4: Tests passent**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: 7 passed

**Step 5: Commit**

```bash
git add src/install.rs
git commit -m "feat(install): settings_path + read_settings"
```

---

## Task 4 — `write_settings()`

**Files:**
- Modify: `src/install.rs`

**Step 1: Tests**

```rust
#[test]
fn write_roundtrip() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-write.json");
    let v = json!({"statusLine": {"command": "ccfaststatus"}, "theme": "dark"});
    write_settings(&tmp, &v).unwrap();
    let re = read_settings(&tmp).unwrap();
    assert_eq!(re, v);
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn write_creates_parent_dir() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-dir/settings.json");
    let _ = std::fs::remove_dir_all(tmp.parent().unwrap());
    write_settings(&tmp, &json!({})).unwrap();
    assert!(tmp.exists());
    let _ = std::fs::remove_dir_all(tmp.parent().unwrap());
}

#[test]
fn write_is_pretty_2space() {
    let tmp = std::env::temp_dir().join("ccfaststatus-test-pretty.json");
    write_settings(&tmp, &json!({"a": {"b": 1}})).unwrap();
    let s = std::fs::read_to_string(&tmp).unwrap();
    assert!(s.contains("  \"b\""));   // 2 espaces d'indent
    let _ = std::fs::remove_file(&tmp);
}
```

**Step 2: Fail**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: FAIL.

**Step 3: Implémenter**

```rust
pub fn write_settings(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {}", parent.display(), e))?;
    }
    let pretty = serde_json::to_string_pretty(value)
        .map_err(|e| format!("serialize error: {}", e))?;
    fs::write(path, pretty + "\n")
        .map_err(|e| format!("failed to write {}: {}", path.display(), e))
}
```

**Step 4: Tests passent**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: 10 passed

**Step 5: Commit**

```bash
git add src/install.rs
git commit -m "feat(install): write_settings with parent dir + pretty 2-space"
```

---

## Task 5 — Parsing intervalle + prompt

**Files:**
- Modify: `src/install.rs`

**Step 1: Tests**

```rust
#[test]
fn parse_interval_uses_default_on_empty() {
    assert_eq!(parse_interval("", 1).unwrap(), 1);
    assert_eq!(parse_interval("\n", 3).unwrap(), 3);
    assert_eq!(parse_interval("   ", 5).unwrap(), 5);
}

#[test]
fn parse_interval_accepts_positive_int() {
    assert_eq!(parse_interval("2", 1).unwrap(), 2);
    assert_eq!(parse_interval("  42  \n", 1).unwrap(), 42);
}

#[test]
fn parse_interval_rejects_zero_and_negative() {
    assert!(parse_interval("0", 1).is_err());
    assert!(parse_interval("-3", 1).is_err());
}

#[test]
fn parse_interval_rejects_non_numeric() {
    assert!(parse_interval("abc", 1).is_err());
    assert!(parse_interval("1.5", 1).is_err());
}
```

**Step 2: Fail**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

**Step 3: Implémenter**

```rust
pub fn parse_interval(raw: &str, default: u32) -> Result<u32, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Ok(default);
    }
    t.parse::<u32>()
        .map_err(|_| format!("not a positive integer: {}", raw.trim()))
        .and_then(|n| if n == 0 { Err("must be > 0".to_string()) } else { Ok(n) })
}

fn prompt_interval(default: u32) -> u32 {
    use std::io::{stdin, stdout, BufRead, Write};
    let stdin = stdin();
    let mut out = stdout();
    loop {
        print!("Intervalle de rafraîchissement en secondes [{}]: ", default);
        let _ = out.flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            return default;
        }
        match parse_interval(&line, default) {
            Ok(n) => return n,
            Err(msg) => eprintln!("  ✗ {}", msg),
        }
    }
}
```

**Step 4: Tests passent**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: 14 passed

**Step 5: Commit**

```bash
git add src/install.rs
git commit -m "feat(install): parse_interval + prompt_interval with retry"
```

---

## Task 6 — Refactor `main.rs` : extraire `render()` pour le preview

**Files:**
- Modify: `src/main.rs`

**Step 1: Extraire une fonction `render`**

Actuellement tout le code entre la lecture stdin et le `println!` final est inline dans `main()`. Extraire en une fonction pure :

```rust
fn render(data: ClaudeInput, cols: usize) -> String {
    // tout ce qui est entre la ligne 44 (let cwd = ...) et la ligne 285 (println!)
    // except lire stdin et le println lui-même
    // ...
    // au lieu de `println!("{}", output);`
    // retourner `output`
}
```

Et `main()` devient :

```rust
fn main() {
    if std::io::stdin().is_terminal() {
        install::run();
        return;
    }

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
```

**Step 2: Vérifier que tout build et tests passent**

```bash
cargo build --release 2>&1 | tail -5
cargo test 2>&1 | tail -10
```

Expected: build OK + tous les tests existants (y compris goldens si repo clean) passent.

**Step 3: Smoke test**

```bash
echo '{}' | ./target/release/ccfaststatus | head -c 60
```

Expected: sortie ANSI non vide (identique à avant la refacto).

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "refactor: extract render() from main for reuse"
```

---

## Task 7 — `preview()` dans install

**Files:**
- Modify: `src/install.rs`
- Modify: `src/main.rs` (exposer `render` et `ClaudeInput` au module install)

**Step 1: Rendre `render` accessible**

Dans `src/main.rs`, retirer le `fn render` privé et le remplacer par `pub(crate) fn render`. S'assurer que `ClaudeInput` est bien `pub` dans `src/input.rs` (devrait déjà l'être).

**Step 2: Implémenter preview**

```rust
// src/install.rs
use crate::input::ClaudeInput;
use crate::term::get_cols;

pub fn preview() -> String {
    let data: ClaudeInput = serde_json::from_str("{}").expect("{} is valid");
    crate::render(data, get_cols())
}
```

**Step 3: Test unitaire**

```rust
#[test]
fn preview_produces_non_empty_output() {
    let p = preview();
    assert!(!p.is_empty());
    assert!(p.contains("\x1b["));  // ANSI escape présent
}
```

**Step 4: Vérifier**

```bash
cargo test --lib install::tests 2>&1 | tail -5
```

Expected: 15 passed

**Step 5: Commit**

```bash
git add src/install.rs src/main.rs
git commit -m "feat(install): preview() reusing render"
```

---

## Task 8 — Orchestrateur `run()`

**Files:**
- Modify: `src/install.rs`

**Step 1: Implémentation**

Remplacer le stub `pub fn run()` :

```rust
use crate::term::{BOLD, RST};

pub fn run() {
    if let Err(e) = run_inner() {
        eprintln!("ccfaststatus: {}", e);
        std::process::exit(1);
    }
}

fn run_inner() -> Result<(), String> {
    let path = settings_path()?;
    let current = read_settings(&path)?;
    let already = is_already_configured(&current);

    if already {
        println!("{}ccfaststatus est déjà configurée.{}", BOLD, RST);
    } else {
        println!("{}Status Line non configurée. Installation…{}", BOLD, RST);
    }

    let default_interval = current
        .get("statusLine")
        .and_then(|s| s.get("refreshInterval"))
        .and_then(|n| n.as_u64())
        .map(|n| n as u32)
        .unwrap_or(1);

    let interval = prompt_interval(default_interval);
    let updated = update_settings(current, interval);
    write_settings(&path, &updated)?;

    // Round-trip verification
    let verify = read_settings(&path)?;
    if !is_already_configured(&verify) {
        return Err("round-trip verification failed".to_string());
    }

    if already {
        println!("Mis à jour.");
    } else {
        println!("Status Line installée. Redémarre Claude Code.");
    }

    println!();
    println!("Aperçu :");
    println!("{}", preview());

    Ok(())
}
```

**Step 2: Smoke test manuel**

```bash
cargo build --release
# Sauvegarde avant test
cp ~/.claude/settings.json /tmp/settings-backup.json
./target/release/ccfaststatus
# Répondre "Enter" pour accepter le défaut
# Vérifier
grep -A3 statusLine ~/.claude/settings.json
# Restaurer
cp /tmp/settings-backup.json ~/.claude/settings.json
```

Expected: la section `statusLine` de `settings.json` pointe sur `ccfaststatus`, intervalle 1 (ou la valeur saisie).

**Step 3: Commit**

```bash
git add src/install.rs
git commit -m "feat(install): run() orchestrator with round-trip verify + preview"
```

---

## Task 9 — `caveats` dans le Formula Homebrew

**Files:**
- Modify: `r9r-dev/homebrew-tap:Formula/ccfaststatus.rb` (repo séparé)

**Step 1: Cloner et éditer**

```bash
cd /tmp && rm -rf homebrew-tap
git clone https://github.com/r9r-dev/homebrew-tap.git
cd homebrew-tap
```

Dans `Formula/ccfaststatus.rb`, ajouter juste avant la ligne `test do` :

```ruby
def caveats
  <<~EOS
    Run the following to configure your Claude Code status line:
      ccfaststatus
  EOS
end
```

**Step 2: Commit et push**

```bash
git add Formula/ccfaststatus.rb
git commit -m "ccfaststatus: add caveats for interactive config"
git push origin main
```

**Step 3: Vérification locale**

```bash
brew update
brew info ccfaststatus | grep -A2 caveats
```

Expected: le texte des caveats s'affiche.

---

## Task 10 — README

**Files:**
- Modify: `README.md`

**Step 1: Section « Configuration »**

Remplacer le bloc qui dit :

```md
Puis dans `~/.claude/settings.json` :

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccfaststatus",
    "refreshInterval": 3
  }
}
```

Par :

```md
## Configuration

Après installation, lance `ccfaststatus` depuis un terminal pour configurer
interactivement `~/.claude/settings.json` :

```sh
ccfaststatus
```

Deux prompts :
1. Si la Status Line n'est pas encore configurée, elle sera installée.
2. L'intervalle de rafraîchissement (défaut : 1 seconde, le minimum autorisé
   par Claude Code).

Redémarre Claude Code après configuration.
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README section config interactive"
```

---

## Task 11 — Bump version + release

**Utiliser le skill `/publish`** (déjà défini dans `.claude/skills/publish/SKILL.md`) :

- Bump `Cargo.toml` → `version = "0.2.0"` (feature majeure, pas patch)
- Tag `v0.2.0`, push
- Attendre le workflow Release
- Récupérer le SHA256
- Mettre à jour `Formula/ccfaststatus.rb` dans le tap (version + url + sha256)
- `brew upgrade ccfaststatus`
- Relancer `ccfaststatus` depuis un terminal pour valider le flow de bout en bout

---

## Critères de validation finale

- [ ] `cargo test` : tous les tests passent (y compris les nouveaux `install::tests`)
- [ ] `echo '{}' | ccfaststatus` : sortie ANSI statusline inchangée vs v0.1.0
- [ ] `ccfaststatus` (TTY) : deux prompts, update correct du settings.json, preview visible
- [ ] `brew info ccfaststatus` : caveats affichés
- [ ] Hot path benchmark : médiane ≤ 10 ms (vs ~9.75 ms actuel — on s'autorise +0.25 ms max)
- [ ] Preview final montre bien les segments (pas de ligne vide)
