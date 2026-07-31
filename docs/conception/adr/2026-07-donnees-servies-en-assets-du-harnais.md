# Données servies en assets du harnais (copies + fetch)

## Contexte

Le harnais `ui-debug` tourne en WASM : pas de système de fichiers, les snapshots (`data/cours.json`, 8,6 Mo, et `data/programmes/*.json`) doivent arriver par HTTP.
`dx serve` ne sert que les assets déclarés par `asset!()`, qui sont résolus à la compilation.

## Décision

Les snapshots sont copiés dans `crates/ui-debug/assets/data/` par la cible `make ui-debug-data` (enchaînée par `make serve-debug`), déclarés par `asset!()` fichier par fichier, et chargés au démarrage par `fetch` (gloo-net) puis `serde_json`.
Les copies sont gitignorées (données dérivées, jamais un livrable) et à rafraîchir après chaque scrape.
La liste des programmes est un manifeste en dur dans `data.rs` : `asset!()` étant compile-time, aucun listage d'annuaire n'existe au runtime.
Le snapshot des cours est chargé une seule fois dans `App` (`use_resource`) et partagé aux deux pages par le contexte ; les pages le lisent par référence (`.read()`), jamais par clone.

## Alternatives rejetées

- Symlinks vers `data/` : essayé d'abord ; `asset!()` canonicalise le chemin et refuse tout fichier hors du crate.
- `include_str!` : 8,6 Mo dans le binaire WASM et recompilation à chaque scrape.
- Servir `data/` par un serveur à part : le harnais doit rester `make serve-debug` sans autre processus.
