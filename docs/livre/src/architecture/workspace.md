# Vue d'ensemble du workspace

Un seul workspace Cargo, tout en Rust, quatre crates :

| Crate | Type | Rôle |
|---|---|---|
| `core` | bibliothèque | toute la logique du domaine, zéro IO, zéro async ; compile en natif et en WASM ; porte aussi le parseur des pages ULaval, derrière la feature `parser` (activée par défaut, désactivée pour `wasm`) |
| `scraper` | binaire natif async | télécharge les pages ULaval et les parse via `core`, en snapshots JSON |
| `wasm` | `cdylib` + rlib | le crate de frontière : `core` exposé au JavaScript nu **et** au worker de l'app Dioxus, plus les fonctions pures que celle-ci appelle nativement — voir [La frontière WASM](frontiere-wasm.md) |
| `ui` | binaire WASM | l'interface Dioxus |

## Les invariants porteurs

Ce sont des contraintes, pas des préférences :

- **Toute la logique d'affaires vit dans `core`**, aucune dans la vue ni dans la glue.
  Les solveurs sont des fonctions pures testées en natif ; le navigateur n'exécute que la même chose, compilée autrement.
- **Aucune entrée non reconnue n'est perdue en silence.**
  Ce que la grammaire ne comprend pas est gardé verbatim (`raw`) et remonté ; un champ inconnu en entrée est refusé.
- **Remplacement atomique des snapshots** : les fichiers servis restent valides pendant un scrape ; le `rename` final est le seul moment où le contenu change.
- **Statique et sans serveur** : les snapshots sont produits par un cron de CI, jamais par l'application ; l'état utilisateur vit dans `localStorage` ; le partage passe par l'URL.

## Vérification

- `make static` : `cargo fmt` + clippy natif (`--all-features`) + clippy du crate wasm sur la cible `wasm32-unknown-unknown`, tout avertissement étant une erreur.
- `make test` : `cargo +nightly llvm-cov` — la couverture doit être à 100 % une fois une fonctionnalité terminée, et la CI l'exige (`--fail-under-lines 100`).
- `make wasm` : `wasm-pack build crates/wasm --target web` — le paquet ES publié sur Pages.
- `make ui-calc` : le même crate construit dans les assets du `ui`, pour son Web Worker — un seul crate, un seul artefact.
- `make docs` : construit ce livre (`mdbook build docs/livre`).

## Décisions

Chaque décision est consignée dans un ADR sous `docs/conception/adr/` — un fichier par décision : contexte, décision, alternatives rejetées.
Le plan (`docs/project_plan.md`) porte le *quoi* ; l'ADR préserve le *pourquoi*.
