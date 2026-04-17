# ccfaststatus

The Fastest Status Line for Claude Code. Binaire Rust natif, drop-in compatible
avec la config `statusLine.command` de `~/.claude/settings.json`.

## Motivation

Permettre un refresh à la seconde — l'intervalle le plus court autorisé par
Claude Code — avec toute la logique en Rust : zéro `subprocess` (pas d'appel à
`git`, `ps`, `stty`), libgit2 vendored, énumération processus par FFI native.

## Perf observée

| Chemin | Temps mesuré (macOS arm64) |
|--------|----------------------------|
| Cache git chaud (médiane pure binaire) | ~9.75 ms |
| Cache git froid (premier run, disk load) | ~18 ms |
| Cache git froid (runs cold suivants) | ~11 ms |
| Taille binaire | 974 KB |

Mesures réalisées via `zsh/datetime` (`EPOCHREALTIME`) sur Apple Silicon. Le chiffre pure binaire est mesuré en fournissant le payload sur stdin directement, sans overhead de shell pipe (`cat file | binary` ajoute ~2.5 ms supplémentaires).

## Installation

### Homebrew (macOS arm64)

```sh
brew install r9r-dev/tap/ccfaststatus
```

### Depuis les sources

```sh
cargo build --release
ln -sf "$PWD/target/release/ccfaststatus" ~/.local/bin/ccfaststatus
```

Le premier build compile `libgit2` statiquement (~60-120 s). Les builds suivants
sont quasi-instantanés (LTO `fat` + `codegen-units = 1`).

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
- 8 tests golden qui comparent la sortie ANSI à des fixtures figées, avec masquage des parties dépendant du temps (HH:MM, durées, time_left, compteurs git)

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
  golden.rs     -- snapshot tests contre fixtures figées
  fixtures/     -- JSON payloads + .expected captures
```
