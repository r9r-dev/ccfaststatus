# ccfaststatus

"The Fastest Status Line for Claude Code!"

## Motivation

A la base, je souhaitais m'amuser à créer une statusline en 60 fps (parce que pourquoi pas). Au final, l'intervalle le plus court autorisé par Claude Code étant de 1 seconde, je m'en contente mais garde le même objectif : mettre a jour la status line en moins de 16,6ms (soit 60 fps théorique).

Pour permettre ce refresh à la seconde sans perdre en fonctionnalités, toute la logique est en Rust et aucun "subprocess" n'est autorisé. Donc pas d'appel à `git`, `ps` ou `stty` par exemple.

## Perf observée

Après quelques itérations et fix, la première version est tout juste au premier lancement mais ensuite passe largement sous les 16ms.

| Chemin | Temps mesuré (macOS arm64) |
|--------|----------------------------|
| Premier lancement | ~18 ms |
| Cache git froid | ~11 ms |
| Cache git chaud | ~10 ms |

Mesures réalisées via `zsh/datetime` (`EPOCHREALTIME`) sur Apple Silicon. Le fait de lancer la commande `datetime` a sans doute son propre impact donc en réel on est probablement sous ces valeurs.

## Installation

### Homebrew (macOS arm64 uniquement !)

```sh
brew install r9r-dev/tap/ccfaststatus
```

### Depuis les sources

```sh
cargo build --release
ln -sf "$PWD/target/release/ccfaststatus" ~/.local/bin/ccfaststatus
```

## Configuration

Après installation, lancer `ccfaststatus` depuis un terminal pour installer la status line :

```sh
ccfaststatus
```

## Tests

```sh
cargo test
```
