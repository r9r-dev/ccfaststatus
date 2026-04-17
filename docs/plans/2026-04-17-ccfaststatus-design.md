# ccfaststatus — Design

## Objectif

Reproduire à l'identique la statusline Claude Code actuelle (`~/.claude/statusline.mjs`, Node.js), mais en Rust natif, sans subprocess, avec un startup aussi rapide que possible (cible long terme : 16.6 ms pour un refresh 60 Hz).

## Contrat I/O

- Lit sur **stdin** un payload JSON émis par Claude Code (modèle, version, workspace, git_worktree, context_window, cost, rate_limits).
- Écrit sur **stdout** une ligne ANSI unique (powerline, Nerd Font).
- Zéro output sur stderr, zéro panic remontée (dégradation silencieuse sur erreur).

## Architecture

Binaire unique `ccfaststatus`. Pas de subprocess (`git`, `ps`, `stty` remplacés par des crates natives).

```
src/
  main.rs      -- entry: lit stdin, orchestre la collecte, imprime
  input.rs     -- structs serde pour le payload Claude Code
  config.rs    -- constantes: palette couleurs, icônes, BAR_WIDTH, TTL, chemin cache
  term.rs      -- ANSI (fgc/bgc/RST/BOLD/DIM), strip_ansi, get_cols
  format.rs    -- fmt_time, fmt_duration, fmt_tokens, mini_bar, context_bar
  git.rs       -- GitInfo, cache binaire, appels git2
  sessions.rs  -- comptage processus `claude` via sysinfo
  segments.rs  -- Segment, build_powerline, render, troncature par priorité
tests/
  golden.rs    -- fixtures JSON → sortie ANSI attendue (snapshot byte-à-byte)
  segments.rs  -- tests de troncature
```

**Flow** `main` :

1. Lire stdin jusqu'à EOF, parser en `ClaudeInput` (serde).
2. Lancer en parallèle (threads) : `git::info(cwd)`, `sessions::count()`, `term::cols()`.
3. Joindre les résultats.
4. Construire `Vec<Segment>` avec priorités identiques au JS :
   - p1 = modèle (toujours visible)
   - p2 = contexte
   - p3 = git
   - p4 = folder
   - p5 = heure/sessions/durée
   - p6 = limite 5h *ou* coût
   - p7 = limite 7d *et* suffix version
5. `build_powerline(segments, suffix, cols)` tronque par priorité décroissante tant que `strip_ansi(line).len() > cols`.
6. `println!` sur stdout.

## Dépendances

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
bincode = "1"
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

Justifications :

- `git2` en vendored + `default-features = false` → libgit2 statique, pas de SSH/HTTPS inutile, binaire portable ~2–3 Mo.
- `sysinfo` sans features disque/réseau, uniquement `system`.
- `chrono` sans `serde`, `wasm`, locales (on n'affiche que HH:MM).
- `bincode` pour le cache binaire (parse ~10× plus rapide que serde_json).
- **Pas de `anyhow`** : `Result<_, Box<dyn Error>>` en surface, `unwrap_or_default` ailleurs. Le JS fait `catch {}` partout — on dégrade pareil.

## Budgets de performance

Mesurés au cas par cas, one-shot, cache git chaud :

| Étape | Budget |
|-------|--------|
| Spawn + runtime init (macOS) | ~5 ms |
| Lecture stdin + parse JSON | ~2 ms |
| Collecte parallèle (git cache hit + sessions + cols) | ~1 ms |
| Render + write stdout | < 1 ms |
| **Total cache chaud** | **~8 ms** |
| Git cache miss (libgit2 status repo moyen) | +10–15 ms |

La cible 16.6 ms (60 Hz) est atteignable cache chaud. Cache froid on dépasse — acceptable car le cache a un TTL de 5 s, donc 1 refresh sur ~300 à 60 Hz paie le coût.

**Note long terme** : passer en dessous de 16.6 ms garanti nécessitera un **mode daemon** (socket Unix, état en mémoire) ; hors scope v1.

Optimisations appliquées :

1. **Collecte parallèle** via `std::thread::spawn` pour git / sessions / cols.
2. **Cache binaire** (`bincode`) au lieu de JSON. Fichier renommé `/tmp/.claude-statusline-git-cache.bin` pour éviter toute confusion avec l'ancien format JSON.
3. **Pré-allocation** : `String::with_capacity(512)` pour la ligne finale.
4. **Pas de `format!`** dans les hot paths → `write!` direct dans un `String` buffer.
5. **sysinfo minimal** : `refresh_processes(ProcessesToUpdate::All, false)`, pas de CPU/mémoire.

## Fidélité vs script JS

**Identique** :

- Palette M365Princess (BG_TIME, BG_MODEL, BG_FOLDER, BG_GIT, BG_CTX, BG_LIMIT_5H, BG_LIMIT_7D).
- Icônes Nerd Font (tous les codepoints repris tels quels).
- Séparateurs powerline (`\uE0B0`, `\uE0B1`).
- Priorités de troncature.
- Formats `fmt_time`, `fmt_duration`, `fmt_tokens`.
- Barre contexte braille (8 niveaux verticaux).
- Condition d'affichage des segments rate limits / coût / version.
- Cache git TTL 5 s, path `/tmp/.claude-statusline-git-cache.*`.

**Écarts assumés** :

1. Comptage des sessions actives via `sysinfo` (syscalls natifs) au lieu de `ps -Ao comm`. Résultat identique tant que l'exécutable Claude Code s'appelle `claude`.
2. Terminal width via `terminal_size::terminal_size()` au lieu de `stty -f /dev/tty size`. Fallback identique sur `$COLUMNS` puis `120`.
3. Format du cache binaire (bincode) au lieu de JSON. Comportement applicatif inchangé.

## Gestion des erreurs

- Stratégie "best effort" : toute erreur de collecte (git absent, sysinfo échoue, cache corrompu) → donnée omise, autres segments rendus normalement.
- Jamais de panic remontée à l'utilisateur. `main` retourne `Result` mais les helpers retournent des `Option`.
- Cache corrompu = ignoré + réécrit au prochain cache miss.

## Tests

Trois niveaux, tous dans `cargo test` :

1. **Unitaires** (dans les modules `format.rs`, `term.rs`) — `fmt_time`, `fmt_duration`, `fmt_tokens`, `mini_bar`, `context_bar`, `strip_ansi`. Cas limites : 0, overflow, négatifs. Cible : ~20 tests.

2. **Segments** (`tests/segments.rs`) — vecteurs synthétiques, vérifie que `build_powerline` retire les segments dans l'ordre p7 → p6 → ... → p1 quand `cols` diminue.

3. **Golden** (`tests/golden.rs`) — fixtures `tests/fixtures/*.json` extraites depuis des runs réels du script JS, chaque fixture a un `.expected` contenant la sortie ANSI byte-à-byte. Régénération par `UPDATE_GOLDEN=1 cargo test`. Fixtures couvrant au minimum :
   - `minimal.json` — pas de git, pas de rate_limits
   - `with_git.json` — git repo avec ahead/modified
   - `rate_limits.json` — 5h + 7d affichés
   - `narrow_80cols.json` — troncature forcée
   - `worktree.json` — indicateur worktree

**Hors tests** : intégration git2 et sysinfo (I/O externe, vérif manuelle).

## Extensions futures (hors scope v1)

- **Thèmes** : `config.rs` est déjà isolé → charger une palette depuis `~/.config/ccfaststatus/theme.toml`.
- **Activation/désactivation de segments** : `segments` est déjà un `Vec` → filtrer selon config.
- **Cache configurable** : TTL et path déplaçables en config.
- **Mode daemon** : socket Unix + état persistant pour descendre sous 5 ms par refresh.
