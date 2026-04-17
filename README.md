# ccfaststatus

Statusline Claude Code réécrite en Rust natif. Drop-in compatible avec la config
`statusLine.command` de `~/.claude/settings.json`, byte-for-byte identique à la
référence Node.js (`~/.claude/statusline.mjs`).

## Motivation

- Binaire unique, aucune dépendance runtime (libgit2 vendored, sysinfo natif).
- Startup optimisé pour un refresh rapide (cible long terme : 60 Hz / 16.6 ms).
- Toute la logique en Rust : zéro `subprocess` (plus d'appel à `git`, `ps`, `stty`).

## Perf observée

| Chemin | Temps mesuré (macOS arm64) |
|--------|----------------------------|
| Cache git chaud (médiane sur 20) | ~19.4 ms |
| Cache git froid (médiane sur 5) | ~18.5 ms |
| Taille binaire | 991 KB |

Mesures réalisées via `zsh/datetime` (`EPOCHREALTIME`) sur Apple Silicon. Ce chiffre inclut le `cat file | binary` (overhead shell ~2.5 ms), donc le coût réel du binaire est autour de 15-17 ms.

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

38 tests au total :
- 34 tests unitaires (formatters, ANSI, segments, etc.)
- 4 tests golden qui comparent byte-à-byte contre le script JS de référence, avec masquage des parties dépendant du temps (HH:MM, durées, time_left, compteurs git)

## Architecture

```
src/
  main.rs       -- orchestration (stdin, threads, segments, println)
  input.rs      -- structs serde pour le payload Claude Code
  config.rs     -- palette M365Princess, icônes Nerd Font, priorités
  term.rs       -- ANSI helpers (fgc/bgc, strip_ansi, get_cols)
  format.rs     -- fmt_time, fmt_duration, fmt_tokens, mini_bar, context_bar
  segments.rs   -- Segment + build_powerline avec troncature par priorité
  sessions.rs   -- comptage processus claude via sysinfo
  git.rs        -- git2 + cache binaire bincode (TTL 5 s)
tests/
  golden.rs     -- snapshot tests vs JS reference
  fixtures/     -- JSON payloads + .expected captures
```

## Fidélité

La parité byte-for-byte avec `~/.claude/statusline.mjs` est vérifiée par `cargo
test --test golden` sur quatre fixtures : minimal, avec git, avec rate_limits,
et 80 colonnes (troncature forcée).
