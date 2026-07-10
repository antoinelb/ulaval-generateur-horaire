# Nommage des crates du workspace : préfixe `ulaval-scheduler-`

Date : 2026-07-10

## Contexte

Le plan nomme les trois crates `core`, `scraper`, `ui`.
Deux de ces noms entrent en collision *localement* (indépendamment de toute publication), ce qui a été vérifié empiriquement :

- Un crate dont le nom d'importation est `core` masque le crate sysroot `core` chez tout crate qui en dépend.
Comme `scraper` et `ui` dépendent tous deux du crate de domaine, leur premier `core::mem`, `core::fmt`, `core::cmp::Ordering`… échoue avec `error[E0433]: cannot find ... in core`.
Le crate `core` compile pourtant très bien seul : la collision est purement en aval.
- Un paquet nommé `scraper` partage son nom avec sa dépendance `scraper` (le parseur HTML du plan), ce qui est au mieux déroutant.

La publication sur crates.io n'entre pas en jeu (site statique, jamais publié) ; l'unicité globale des noms est donc sans objet.
Le seul enjeu réel est la collision locale.

## Décision

Les répertoires gardent la nomenclature du plan (`crates/core`, `crates/scraper`, `crates/ui`) ; les **paquets Cargo** portent le préfixe descriptif :

- `ulaval-scheduler-core` — bibliothèque, importée `ulaval_scheduler_core`.
- `ulaval-scheduler-scraper` — binaire natif (cible `scraper`).
- `ulaval-scheduler-ui` — binaire WASM.

Le préfixe résout les deux collisions sans artifice : le nom d'importation n'est plus `core` (le `core::` sysroot reste disponible en aval), et le paquet ne partage plus son nom avec la dépendance `scraper`.

## Alternatives rejetées

- **Noms nus `core`/`scraper`/`ui`** : exige un `[lib] name = "domaine"` sur le crate de domaine pour libérer `core::` (vérifié fonctionnel), et laisse le paquet `scraper` homonyme de sa dépendance.
Fonctionne, mais chaque contournement est une ligne à expliquer plus tard.
- **Préfixe court `horaire-`** : uniforme et sans collision lui aussi, mais moins explicite ; le préfixe descriptif a été préféré pour la lisibilité, le surcoût de frappe étant jugé acceptable.
- **Préfixe `gex-`** : trop lié à un seul programme (GEX) alors que le domaine vise le catalogue complet.
