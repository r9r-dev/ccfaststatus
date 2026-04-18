# Roadmap ccfaststatus — Config, Thèmes/Skins, Métriques

**Date** : 2026-04-18
**Statut** : design validé
**Auteur** : Romain Lamour

---

## Contexte

L'utilisateur souhaite ajouter trois capacités à ccfaststatus :
1. Une configuration utilisateur pour choisir les segments visibles
2. De nouvelles métriques
3. Un système de thèmes

Aujourd'hui la statusline est monolithique : segments hardcodés dans `main.rs::render`, palette de couleurs figée dans `config.rs`, aucune config utilisateur propre (seul `install.rs` manipule le `settings.json` de Claude Code).

## Principe directeur : rétro-compatibilité stricte

Un utilisateur qui fait `brew upgrade ccfaststatus` sans config file doit obtenir **exactement le même rendu qu'avant**. Les nouvelles fonctionnalités sont toutes opt-in via `~/.config/ccfaststatus/config.toml`. Aucune des 3 releases ne change le comportement par défaut d'un poste existant.

## Séquence des releases

| Release | Feature | Risque | Justification de l'ordre |
|---|---|---|---|
| v0.3.0 | Config utilisateur + TUI de setup | moyen | Socle : les deux autres features s'y greffent |
| v0.4.0 | Thèmes (couleurs) + Skins (forme) | moyen | Cosmétique, réutilise le parseur TOML, introduit `trait Skin` et `SegmentKind` qui préparent la v0.5 |
| v0.5.0 | 5 nouvelles métriques | élevé | Hot path, FFI macOS, s'appuie sur config (opt-in) et sur `SegmentKind` (bullet skin) |

---

## Release 1 — v0.3.0 : Config utilisateur

### Emplacement du fichier
- `$XDG_CONFIG_HOME/ccfaststatus/config.toml` avec fallback `~/.config/ccfaststatus/config.toml`
- Résolution manuelle via `std::env::var("XDG_CONFIG_HOME")` + `$HOME` (cohérent avec `install.rs::settings_path`, zéro nouvelle dépendance)

### Format TOML
```toml
[segments]
time    = true
model   = true
folder  = true
git     = true
context = true
cost    = true
limits  = true
version = true
```

Règles :
- Fichier absent → défauts = tous `true` (comportement actuel préservé)
- Clé absente → défaut individuel
- Clé inconnue → ignorée silencieusement (forward-compat)
- TOML malformé → warning `stderr`, défauts

### Garde-fou
Si **tous** les segments sont désactivés : fallback forcé `model = true` à la lecture. Empêche un output vide qui ressemble à un bug.

### Nouveau module `src/settings.rs`
Distinct de `install.rs` (qui manipule le `settings.json` de Claude Code).
```rust
pub struct Settings {
    pub segments: SegmentFlags,
    // v0.4: theme, skin
    // v0.5: metrics flags
}

impl Settings {
    pub fn load() -> Self;        // jamais de panic sur le hot path
    pub fn write(&self) -> Result<(), io::Error>;
}
```

### Intégration hot path
- `main.rs::render` ouvre `let settings = Settings::load();` en tête
- Chaque `segments.push(...)` wrappé par `if settings.segments.X`
- Coût TOML parsing cold-start : ~50-200 µs. Budget total < 10 ms → OK

### TUI de configuration (ratatui)
Dépendances ajoutées :
- `ratatui` (~500 KB compilé)
- `crossterm` (~150 KB, backend terminal)

Nouveau module `src/tui/` avec :
- Panneau gauche : catégories (`Segments`, `Thème` v0.4, `Skin` v0.4, `Métriques` v0.5)
- Panneau central : options de la catégorie sélectionnée
- Panneau droit : preview live de la statusline avec la config en cours

Navigation :
- `Tab` / `Shift+Tab` : changer de panneau
- `↑↓` : naviguer dans les options
- `Space` : toggle un flag
- `Enter` : sélectionner
- `s` : sauvegarder et quitter
- `q` / `Esc` : quitter sans sauvegarder

**Design data-driven** : la liste des segments exposés dans le panneau "Segments" est une source de vérité unique (ex. `const ALL_SEGMENTS: &[SegmentDef]`), pour que l'ajout d'une métrique en v0.5 ne nécessite aucune modification du code TUI.

### Flow `install.rs` réorganisé
1. Install Claude Code `settings.json` (actuel, inchangé)
2. Propose `Configurer ccfaststatus ? [o/N]`
3. Si oui : lance la TUI `tui::run()`

### Tests
- `Settings::load()` : fichier absent / partiel / malformé / tous `false`
- Round-trip read/write TOML
- Golden tests existants : inchangés
- Nouveau golden : config avec `git = false` → segment git absent

### Non-goals v0.3.0
- Pas de reload à chaud (statusline ré-exécuté à chaque tick)
- Pas d'overrides par projet (`.ccfaststatus.toml` local)
- Pas de CLI `ccfaststatus config set ...` — tout via TUI ou édition manuelle du TOML

---

## Release 2 — v0.4.0 : Thèmes + Skins

### Deux axes orthogonaux
- **Thème** = palette de couleurs (bg_time, bg_model, tx_white, etc.)
- **Skin** = forme visuelle (séparateurs, backgrounds, padding, stratégie de rendu)

Combinables librement : `tokyo-night` + `bullet` est valide.

### TOML étendu
```toml
[theme]
name = "m365princess"   # défaut (palette actuelle)

[skin]
name = "powerline"      # défaut (rendu actuel)
```

Clé absente ou inconnue → fallback défaut + warning stderr.

### Thèmes livrés (6)

| Nom | Vibe |
|---|---|
| `m365princess` | palette actuelle, pastel plum/blush/salmon (défaut) |
| `catppuccin` | mocha, pastel chaud |
| `tokyo-night` | sombre bleuté |
| `gruvbox` | retro warm |
| `nord` | cool blues, minimaliste |
| `dracula` | violet/rose saturé |

Architecture :
- `src/theme.rs` avec `struct Theme { bg_time, bg_model, ..., ctx_empty }`
- Chaque thème = `const` compilé dans le binaire, zéro I/O runtime
- `fn resolve_theme(name: &str) -> &'static Theme` avec fallback
- `config.rs` conserve : icônes, priorités, `BAR_WIDTH`, `GIT_CACHE_TTL_MS`, powerline chars
- `config.rs` perd : `BG_*`, `TX_*`, `CTX_EMPTY` (migrent dans `theme.rs`)

### Skins livrés (6)

| Skin | Paradigme |
|---|---|
| `powerline` | triangles `\ue0b0`, bg pleins (actuel, défaut) |
| `minimal` | séparateur ` · `, fg coloré, pas de bg |
| `rounded` | extrémités `\ue0b6`/`\ue0b4`, bg pleins |
| `pipe` | séparateur `|`, fg coloré, pas de bg |
| `rainbow` | préfixe arc-en-ciel en tête + style minimal |
| `bullet` | ronds `●` colorés pour métriques à jauge (ctx%, cost, limits), fg uniquement |

### Architecture `trait Skin` — catalyseur d'extensibilité
Le skin `bullet` a besoin de **réinterpréter** certains segments (transformer `ctx 42%` en un rond de couleur). Cela impose que `Skin` soit une stratégie de rendu, pas un struct de paramètres, et que `Segment` porte sa sémantique.

```rust
trait Skin {
    fn render(&self, rows: &SegmentRows, theme: &Theme, cols: usize, suffix: &str) -> String;
}

type SegmentRows = Vec<Vec<Segment>>;   // v0.4: toujours 1 row. v0.6+: multi-lignes possible.

struct Segment {
    text: String,                  // fallback pré-formaté (utilisé par powerline/minimal/rounded/pipe)
    kind: SegmentKind,             // sémantique pour skins expressifs (bullet)
    bg: Rgb,
    fg: Option<Rgb>,               // explicite pour autoriser override
    icon: Option<&'static str>,    // explicite pour autoriser override d'icône
    priority: u8,
}

enum SegmentKind {
    Time { hour: u8, minute: u8, sessions: usize, duration_ms: u64 },
    Model(String),
    Folder { name: String, is_worktree: bool },
    Git(GitInfo),
    Context { pct: f64, used_tokens: i64, size_label: String },
    Cost(f64),
    Limit5h { pct: i64, time_left: String },
    Limit7d { pct: i64, time_left: String },
    Version(String),
}
```

**Cette abstraction débloque les évolutions futures** (mentionné explicitement par l'utilisateur) :
- Multi-lignes : `SegmentRows` prévu dès v0.4, juste utilisé avec 1 row
- Icônes custom par thème : `Segment.icon` laisse le thème overrider
- Couleurs custom par segment : `Segment.fg` laisse le thème overrider

### Troncature par priorité
Le mécanisme actuel (`build_powerline` qui drop-par-priorité) est extrait en `fn fit_segments(segments, cols, suffix_width) -> Vec<Segment>`, partagé par tous les skins.

### Install TUI v0.4
Deux nouveaux panneaux dans la TUI : `Thème` et `Skin`, avec preview live de la combinaison.

### Tests
- `resolve_theme(name)` et `resolve_skin(name)` avec fallback sur inconnu
- `SegmentKind` correctement assigné par `main.rs::render`
- Matrice réduite de golden tests : 2 thèmes × 3 skins = 6 nouvelles fixtures (pas les 36 combinaisons)
- Les tests golden existants passent sans modif (défaut = `m365princess` + `powerline`)

### Non-goals v0.4.0
- Pas de thème/skin custom dans le TOML
- Pas de skin par segment
- Pas d'animation rainbow (statusline stateless, re-rendu à chaque tick)

### Risque identifié
Le skin `bullet` est le plus complexe car il réinterprète les métriques à jauge. À livrer **en dernier** dans la release, isolable derrière un feature flag Rust si nécessaire.

---

## Release 3 — v0.5.0 : 5 nouvelles métriques

### Principe : toutes opt-in, défaut `false`

```toml
[segments]
# segments existants inchangés...
burn_rate      = false
last_commit    = false
msg_count      = false
claude_proc    = false
battery        = false
```

**Rétro-compatibilité stricte** : un `brew upgrade` vers v0.5 sans édition de config produit le même output que v0.4 ou v0.2.

### 1. Burn rate (tokens/min)
- Source : `data.context_window.current_usage.{input + cache_creation + cache_read}`
- Calcul v0.5 : approximation pure `used_tokens / (total_duration_ms / 60_000)`
  - Pas de cache fichier → simple, sans état
  - Stratégie (a) cache fichier `/tmp/.ccfaststatus-session-<id>.bin` réservée pour v0.6 si précision insuffisante
- Segment : icône `⚡` + `1.2k/min`
- Priorité : 6

### 2. Durée depuis dernier commit
- Source : handle git2 déjà ouvert → `head.peel_to_commit()?.time().seconds()`
- Calcul : `now - commit_time`, formaté via `fmt_duration`
- **Intégré dans le segment git** (pas un nouveau segment) : ` main  2h  +3 ~1`
- Économise de la largeur de terminal ; si `git = false`, cette info disparaît aussi
- Coût : ~1 µs

### 3. Nombre de messages
- Source : vérifier d'abord si `ClaudeInput` porte l'info
- Si absent : parser le transcript JSONL pointé par `data.transcript_path` avec compte de lignes (`BufReader::lines().count()`)
- Risque : transcripts volumineux. Mitigation : FFI `fstat` + scan rapide, ou estimation par taille fichier. Benchmark requis
- Segment : icône `💬` + `42`
- Priorité : 7

### 4. CPU/RAM du process claude
- Sessions.rs liste déjà les PIDs via `proc_listpids` : réutiliser
- Pour chaque PID : `sysctl({ CTL_KERN, KERN_PROC, KERN_PROC_PID, pid })` → `rusage` + `kp_proc.p_pctcpu`
- Agrégation : somme `%CPU` et somme `RSS` (ru_maxrss)
- Segment : icône `🖥` + `45% 1.2G`
- Coût : ~N × 10 µs (N = nombre de sessions, typiquement 1-3)
- Priorité : 7

### 5. Batterie
- Source : IOKit FFI `IOPSCopyPowerSourcesInfo` + `IOPSGetProvidingPowerSourceType`
- Infos : `Current Capacity`, `Max Capacity`, `Is Charging`, `Time to Empty`
- Segment : icône dynamique (plein / demi / quart + `⚡` si charge) + `78%`
- Auto-hidden si le device n'a pas de batterie (desktop)
- Coût : ~50 µs
- Priorité : 5

### Architecture
Nouveau module `src/metrics/` :
- `burn_rate.rs`
- `last_commit.rs`
- `msg_count.rs`
- `claude_proc.rs`
- `battery.rs`

Chaque module expose une fonction pure `collect() -> Option<T>`, invoquée seulement si le flag est `true`.

Parallélisation : les métriques I/O (`msg_count`, `claude_proc`, `battery`) lancées dans leur propre thread au même endroit que `git::info` et `sessions::count`.

### TUI v0.5
Grâce au design data-driven de la TUI v0.3, l'ajout des 5 flags dans le panneau "Segments" est automatique : on étend `ALL_SEGMENTS`, la TUI s'adapte.

### Tests
- Unitaires pour chaque `collect()` avec mocks/fixtures
- Golden tests par métrique activée
- Perf : benchmark total avec tout activé < 10 ms sur macOS arm64

### Non-goals v0.5.0
- Pas d'historique burn rate (graphique ASCII)
- Pas de tracking CPU/RAM agrégé session
- Pas de métriques système (CPU machine, RAM libre) — l'user a précisé "process claude uniquement"
- Pas de Linux — cohérent avec la cible Homebrew arm64 macOS

---

## Dépendances ajoutées (cumulées)

| Crate | Release | Poids | Raison |
|---|---|---|---|
| `toml` | v0.3 | ~80 KB | Parsing config |
| `ratatui` | v0.3 | ~500 KB | TUI panneaux |
| `crossterm` | v0.3 | ~150 KB | Backend terminal pour ratatui |

Pas de nouvelle dépendance en v0.4 (thèmes en const) ni en v0.5 (FFI directes via `libc` déjà présent).

## Budget perf

Cible actuelle : **< 10 ms sur macOS arm64** sur le hot path pipe-stdin.

Coûts ajoutés estimés :
- v0.3 : TOML parsing 50-200 µs (négligeable)
- v0.4 : résolution thème/skin + indirection rendu (< 100 µs)
- v0.5 avec tout activé : ~500 µs supplémentaires (msg_count le pire cas)

Budget respecté. Un benchmark `hyperfine` sera exécuté avant chaque release pour valider.

## Ordre d'implémentation dans chaque release

Les 3 releases suivent la même logique interne :
1. Refacto/fondation (sans changement fonctionnel observable)
2. Nouvelle capacité derrière un flag désactivé
3. Activation progressive + TUI + docs + tests
4. Release via skill `/publish`
