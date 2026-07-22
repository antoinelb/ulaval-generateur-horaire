# Le moteur de l'organigramme (B) devient un embranchement tranché par spike

**Date :** 2026-07-21
**Statut :** remplacé le jour même par `2026-07-b-placement-par-satisfaction-fait-main` (la porte est fermée : satisfaction, placement seul, fait main, sans spike) ; révisait la décision D5 de `docs/conception/initial/ADR.md` pour le seul problème B.

## Contexte

La conception initiale (D5) fixait un moteur unique écrit à la main — énumérer, élaguer, classer — pour tout « le solveur », au titre de la plus riche surface d'apprentissage Rust.
La conception détaillée (`docs/conception/solveur-conception.md`) sépare deux problèmes : A (horaire hebdomadaire, *student sectioning*, instance triviale — moyenne 1,21 option par cours en a2026, 90,3 % à option unique) et B (organigramme, RCPSP à activités optionnelles et précédence ET/OU, NP-difficile mais petit).
Pour A, l'énumération à la main est la méthode indiquée, pas un compromis.
Pour B, un solveur PPC déclaratif optimiserait sur tout l'espace là où le moteur à la main ne classe que ce qu'il énumère ; le choix dépend d'une question de produit (satisfaction ou optimisation ?) et d'objectif (l'apprentissage de Rust s'applique-t-il à B ?).

## Décision

- A reste fait main (arrêté, inchangé par cette révision).
- Le moteur de B devient un **embranchement explicite** : Piste 1 (générer-et-tester à la main) contre Piste 2 (modèle COP sur Pumpkin, recommandée pour « rapide, de qualité, pas nécessairement optimal »), tranché par un spike time-boxé (~1 jour) **après** la construction du substrat partagé (`docs/solveur-plan-implementation.md`, Phase 3).
  Un seul moteur sera implémenté.
- Si la Piste 2 l'emporte, la dépendance est **`pumpkin-core`** (épinglée 0.4.0), jamais l'ombrelle `pumpkin-solver` : celle-ci tire `signal-hook` et une build-dep `cc` pour son CLI, incompatibles WASM.

Faits vérifiés le 2026-07-21 : `pumpkin-core` 0.4.0 (MIT/Apache-2.0, releases régulières) est testé en CI amont sur `wasm32-unknown-unknown` (`wasm-pack`, shim d'horloge `web-time`, `getrandom` backend `wasm_js`, aucun thread) ; l'API couvre l'optimisation *anytime* (`Solver::optimise`, `TimeBudget`, incumbent via callback) et l'ajout incrémental de contraintes après un solve (boucle à nogoods sans reconstruction du modèle).

## Alternatives rejetées

- **Garder D5 tel quel (moteur main partout)** : figeait le moteur de B sans preuve, alors que l'objectif d'apprentissage ne s'étend pas nécessairement à B et que l'*anytime* PPC correspond mieux à « rapide, de qualité, pas optimal ».
- **Trancher tout de suite pour Pumpkin sans spike** : la qualité/latence du modèle réel et notre propre build `wasm-pack` restent à mesurer ; un jour de spike borne le risque.
- **clingo-wasm (ASP)** : le plus expressif, mais module C-compilé-en-WASM hors de `core` — brise l'invariant « toute la logique métier en `core` pur ».
- **good_lp + microlp (PLNE)** : *branch-and-bound* faible, encodage *big-M* maladroit pour ET/OU ; plancher, pas choix de qualité.
- **OR-Tools CP-SAT et autres FFI C/C++** : inutilisables au navigateur ; pertinents seulement si un crate `server` se matérialisait.
- **Huub** (autre solveur LCG en Rust, CP 2025) : frère de lignée crédible, gardé comme repli si Pumpkin fléchissait.
