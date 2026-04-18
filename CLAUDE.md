# ccfaststatus — Notes pour Claude

## Pitch

« The Fastest Status Line for Claude Code », binaire Rust natif. Distribué via
Homebrew (`brew install r9r-dev/tap/ccfaststatus`). macOS arm64 uniquement.

## Architecture rapide

```
src/
  main.rs       -- entry point : TTY check → install::run() OU pipeline normal
                   + fn render(data, cols) -> String (pub(crate), réutilisée par install::preview)
  install.rs    -- mode config interactif : settings_path, read/write_settings,
                   update_settings (pure, testée), parse_interval + prompt_interval,
                   preview(), run() orchestrator avec round-trip verify
  config.rs     -- palette M365Princess, icônes Nerd Font, priorités segments
  input.rs      -- ClaudeInput (serde) — a Default pour parser {} en input minimal
  term.rs       -- ANSI helpers (pub const BOLD, RST, fgc/bgc, strip_ansi, get_cols)
  format.rs     -- fmt_time, fmt_duration, fmt_tokens, mini_bar, context_bar
  segments.rs   -- Segment + build_powerline (troncature par priorité)
  sessions.rs   -- comptage processus "claude" via FFI native (libc proc_listpids + sysctl)
  git.rs        -- git2 + cache binaire bincode (TTL 5 s)
tests/
  golden.rs     -- snapshot tests ANSI contre fixtures figées
  fixtures/     -- .json (payload) + .expected (sortie ANSI capturée)
```

## Workflow de release

Un skill `/publish` existe dans `.claude/skills/publish/SKILL.md`. Il décrit les 6
étapes (bump Cargo.toml, tag, push, attente CI, récup SHA256, MAJ Formula dans
`r9r-dev/homebrew-tap`). L'utiliser pour toute release.

## Conventions

- **Langue :** français pour UX, commits, commentaires. Les messages d'erreur
  user-facing dans `install.rs` sont en français ; les autres (golden tests,
  stderr tech) peuvent rester en anglais.
- **Branche :** commits directs sur `main`, pas de PR interne.
- **Co-author :** chaque commit Claude inclut
  `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`.
- **Perf :** cible hot path < 10 ms sur macOS arm64. Tout ajout sur le chemin
  pipe-stdin doit être benchmarké. Le check `is_terminal()` initial coûte ~0.5 µs.
- **Zéro subprocess** sur le hot path : pas d'appel à `git`, `ps`, `stty`. Tout
  en FFI native ou via crate (git2 vendored).

## Tests

Tests locaux : `cargo test --bin ccfaststatus` (50 tests dont 16 pour install).

Les 8 golden tests (`cargo test --test golden`) peuvent échouer localement
quand le repo ccfaststatus a des commits non-pushés ou des changements
uncommitted, car la fixture `with_git` pointe le repo lui-même. C'est normal
— ils passent sur un repo clean.

## Points d'attention

- `update_settings` panique si la racine de `settings.json` n'est pas un
  objet. `read_settings` valide en amont et rejette (test dédié).
- `is_already_configured` compare `command == "ccfaststatus"` (string exact).
  Si le chemin absolu `/Users/xxx/.local/bin/ccfaststatus` est présent,
  c'est considéré comme non-installé et réinstallé (cas du passage
  pre-brew → post-brew).
- Round-trip verify utilise `verify != updated` (égalité stricte Value),
  pas juste `is_already_configured`, pour attraper toute corruption silencieuse.
- `render()` est `pub(crate)` pour que `install::preview()` puisse l'appeler.
  Si un jour la crate devient `lib`, il faudra repasser en `pub`.

## Homebrew tap

Repo séparé : `r9r-dev/homebrew-tap`. Formula dans `Formula/ccfaststatus.rb`.
Le bloc `caveats` affiche le message d'installation interactive à la fin du
`brew install`. Pour bump le Formula, utiliser le skill `/publish`.
