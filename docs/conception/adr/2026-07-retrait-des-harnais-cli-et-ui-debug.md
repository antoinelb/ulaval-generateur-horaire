# Retrait des harnais CLI et ui-debug, pipelines conservés dans core

## Contexte

Les deux harnais intérimaires ont rempli leur rôle : `crates/cli` (jalon 2, ADR `2026-07-harnais-cli-en-crate-dedie`) a validé les solveurs en natif, et `crates/ui-debug` (ADR `2026-07-harnais-ui-de-debogage-en-crate-dedie`) les a validés en WASM dans le navigateur (conflits sur grille, placement GEX complet en ~30 ms, couverture avec concentration/profil).
La vraie UI (jalons 3–9) démarre dans `crates/ui` et les harnais ne seraient plus que du poids mort à maintenir.

## Décision

Supprimer `crates/cli` et `crates/ui-debug` (membres du workspace, cibles make, entrée `.gitignore`) — l'historique git conserve tout.
Avant la suppression, les compositions de pipeline qui ne vivaient que dans les harnais (dupliquées entre les deux) sont promues dans `core::intake` :

- `schedule_intake(all, session, codes)` — session parsée, codes normalisés, cours sélectionnés avec équivalences résolues ; l'UI enchaîne avec `schedule_report`.
- `placement_intake(program, electives, passed, pins, all)` — entrée tapée strictement validée, obligatoires sans offre écartés bruyamment (`set_aside`), sélection de couverture portant la liste complète réussis inclus ; l'UI enchaîne avec `place` et `coverage_report`.

Ces règles (ensemble `explicit`, sélection incluant les réussis, ordre de la liste) étaient du métier dissimulé dans la glue ; elles sont désormais testées inline dans `intake.rs` à 100 %.
Les libellés français des enums (saisons, jours, statuts, raisons) meurent délibérément avec les harnais : la vraie UI choisira sa propre présentation, l'historique git garde les versions du CLI.
Les tests e2e du CLI disparaissent avec lui ; ce qu'ils protégeaient (le pipeline d'intake) est couvert par les tests des deux nouvelles compositions, et les solveurs restent verrouillés par les fixtures gelées.

## Alternatives rejetées

- Garder le CLI comme second consommateur de `core` : personne ne l'utilise une fois l'UI livrée, et chaque évolution d'intake paierait deux harnais.
- Extraire aussi les libellés français dans core : de la présentation, pas du métier — tranché de laisser la vraie UI décider.
