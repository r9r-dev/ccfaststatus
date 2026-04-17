# Configuration interactive de la statusline

**Date :** 2026-04-17
**Statut :** design validé, prêt pour implémentation

## Objectif

Permettre à un utilisateur qui vient de faire `brew install r9r-dev/tap/ccfaststatus`
de configurer sa `statusLine` dans `~/.claude/settings.json` en lançant
simplement `ccfaststatus` dans son terminal.

## Contraintes

- Aucun impact sur le chemin chaud (appel piped depuis Claude Code) : budget < 1 µs
  pour la détection de mode.
- Pas d'interactivité dans `brew install` (non-interactif par design Homebrew).
- Préserver toutes les autres clés de `settings.json`.
- Couleurs ANSI requises pour permettre un futur preview live de la statusline.

## Déclenchement

Détection par TTY au tout début de `main()` :

```rust
use std::io::IsTerminal;

if std::io::stdin().is_terminal() {
    install::run();
    return;
}
// ... flow statusline normal (lecture stdin JSON)
```

- `stdin` pipe (cas Claude Code) → mode statusline normal
- `stdin` TTY (user tape `ccfaststatus`) → mode install interactif

Coût mesuré : un seul syscall `isatty(0)`, négligeable par rapport au hot path
(9.75 ms).

Un `caveats` est ajouté au Formula Homebrew pour inciter l'utilisateur à lancer
la commande après `brew install` :

```ruby
def caveats
  <<~EOS
    Run the following to configure your status line:
      ccfaststatus
  EOS
end
```

## Flow du mode install

1. Localiser `~/.claude/settings.json` via `$HOME`
2. Lire et parser en `serde_json::Value`. Si le fichier est absent, partir
   d'un objet JSON vide.
3. Inspecter `settings["statusLine"]["command"]` :
   - **Exactement `"ccfaststatus"`** → « ccfaststatus est déjà configurée. »
   - **Autre chose (ou absent)** → « Status Line non configurée. Installation… »
     puis écrire `statusLine = { type: "command", command: "ccfaststatus", refreshInterval: <réponse> }`
4. **Toujours** demander l'intervalle de rafraîchissement (défaut 1).
5. Réécrire `settings.json` en pretty-print, 2 espaces, autres clés intactes.
6. **Vérification finale :**
   - Round-trip : re-parser le fichier écrit, confirmer `statusLine.command == "ccfaststatus"`
     et intervalle correct.
   - Preview : afficher un rendu d'exemple de la statusline (payload `{}` à
     travers le pipeline normal), façon validation visuelle.

## UX

Tous les messages en français. Couleurs ANSI sobres (bold pour labels, vert pour
succès) via `term.rs` déjà en place.

### Cas A — pas installée

```
Status Line non configurée. Installation…
Intervalle de rafraîchissement en secondes [1]: _
Status Line installée. Redémarre Claude Code.

Aperçu :
 ♥ 14:27  󰚩   󰉋 ccfaststatus  󰍛 ⠀⠀⠀⠀⠀ 0% 200k
```

### Cas B — déjà installée

```
ccfaststatus est déjà configurée.
Intervalle de rafraîchissement en secondes [1]: _
Mis à jour.

Aperçu :
 ♥ 14:27  ...
```

## Gestion des erreurs

| Cas | Comportement |
|-----|--------------|
| Permissions read/write refusées sur `settings.json` | Print erreur + chemin, exit 1 |
| JSON invalide (fichier corrompu) | Print `settings.json invalide, corrige-le manuellement`, exit 1 |
| `$HOME` non défini | Print erreur, exit 1 |
| Input intervalle invalide (négatif, non-numérique) | Redemander sans quitter |
| Round-trip post-écriture échoue | Print erreur critique, exit 1 |

## Architecture

Nouveau module `src/install.rs`. API interne :

```rust
pub fn run();                                      // entry point TTY mode
fn settings_path() -> Result<PathBuf, Error>;
fn read_settings(path: &Path) -> Result<Value, Error>;
fn update_settings(current: Value, interval: u32) -> Value;  // pure, testable
fn write_settings(path: &Path, value: &Value) -> Result<(), Error>;
fn prompt_interval(default: u32) -> u32;
fn preview() -> String;                            // pipe `{}` via build_powerline
```

`main.rs` devient :

```rust
fn main() {
    if std::io::stdin().is_terminal() {
        install::run();
        return;
    }
    // ... existing statusline flow
}
```

## Testing

- Unit tests purs sur `update_settings(value, interval)` — vérifier :
  - création du bloc `statusLine` si absent
  - preservation des autres clés
  - mise à jour sans duplication
  - intervalle respecté
- Pas de test automatisé du flow interactif (toucher à `~/.claude/settings.json`
  en test serait destructeur). Test manuel suffisant.

## Non-goals (v0.1.1)

- Pas de configuration granulaire (segments affichés, couleurs) — prévu pour
  une release ultérieure, le preview live sera réutilisé à ce moment-là.
- Pas d'option « uninstall » — l'utilisateur édite `settings.json` à la main
  ou lance `brew uninstall ccfaststatus` (le binaire s'en va, la ligne dans
  settings.json reste mais inoffensive).
- Pas de support de `settings.local.json` — trop rare pour v0.1.1.

## Performance

Overhead ajouté au hot path : un `isatty(0)` syscall, ~0.5 µs. Ratio 0.005 %
du budget 9.75 ms actuel. Sera re-validé post-implémentation avec le bench
`EPOCHREALTIME` existant.
