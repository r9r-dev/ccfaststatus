# ccfaststatus

Statusline Claude Code réécrite en Rust natif. Drop-in compatible avec la config
`statusLine.command` de `~/.claude/settings.json`, byte-for-byte identique à la
référence Node.js (`~/.claude/statusline.mjs`).

## Motivation

- Binaire unique, aucune dépendance runtime (libgit2 vendored, énumération processus par FFI native).
- Startup optimisé pour un refresh rapide (cible long terme : 60 Hz / 16.6 ms).
- Toute la logique en Rust : zéro `subprocess` (plus d'appel à `git`, `ps`, `stty`).

## Perf observée

| Chemin | Temps mesuré (macOS arm64) |
|--------|----------------------------|
| Cache git chaud (médiane pure binaire) | ~9.75 ms |
| Cache git froid (premier run, disk load) | ~18 ms |
| Cache git froid (runs cold suivants) | ~11 ms |
| Taille binaire | 974 KB |

Mesures réalisées via `zsh/datetime` (`EPOCHREALTIME`) sur Apple Silicon. Le chiffre pure binaire est mesuré en fournissant le payload sur stdin directement, sans overhead de shell pipe (`cat file | binary` ajoute ~2.5 ms supplémentaires).

## Build

```sh
cargo build --release
```

Le premier build compile `libgit2` statiquement (~60-120 s). Les builds suivants
sont quasi-instantanés (LTO `fat` + `codegen-units = 1`).

## Installation

```sh
# Lier le binaire dans un dossier du PATH.
ln -sf "$PWD/target/release/ccfaststatus" ~/.local/bin/ccfaststatus
```

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

## Tests

```sh
cargo test
```

42 tests au total :
- 34 tests unitaires (formatters, ANSI, segments, etc.)
- 8 tests golden qui comparent byte-à-byte contre le script JS de référence, avec masquage des parties dépendant du temps (HH:MM, durées, time_left, compteurs git)

Fixtures golden (`tests/fixtures/*.json` + `.expected`) :
`minimal`, `with_git`, `rate_limits`, `narrow_80cols`, `cost_only`,
`no_workspace`, `worktree`, `narrow_version_drop`.

## Architecture

```
src/
  main.rs       -- orchestration (stdin, threads, segments, println)
  input.rs      -- structs serde pour le payload Claude Code
  config.rs     -- palette M365Princess, icônes Nerd Font, priorités
  term.rs       -- ANSI helpers (fgc/bgc, strip_ansi, get_cols)
  format.rs     -- fmt_time, fmt_duration, fmt_tokens, mini_bar, context_bar
  segments.rs   -- Segment + build_powerline avec troncature par priorité
  sessions.rs   -- comptage processus claude via FFI native
                   (macOS : libc::proc_listpids + sysctl KERN_PROCARGS2
                    Linux : readdir /proc/*/comm)
  git.rs        -- git2 + cache binaire bincode (TTL 5 s)
tests/
  golden.rs     -- snapshot tests vs JS reference
  fixtures/     -- JSON payloads + .expected captures
```

## Fidélité

La parité byte-for-byte avec `~/.claude/statusline.mjs` est vérifiée par `cargo
test --test golden` sur huit fixtures : `minimal`, `with_git`, `rate_limits`,
`narrow_80cols`, `cost_only`, `no_workspace`, `worktree`, `narrow_version_drop`.
