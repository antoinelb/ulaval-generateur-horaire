# Un module WASM exposant quatre fonctions à JS

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

L'UI prévue est Dioxus (`crates/ui`), qui appelle `core` nativement en Rust.
Antoine a demandé ce qu'il faudrait pour qu'un consommateur **JavaScript** — un front hors Dioxus, l'architecture Elm-vanilla qu'il utilise ailleurs — puisse appeler le solveur : deux fonctions de vérification (horaire, organigramme) et deux de génération.

Constat : `core` compilait déjà vers `wasm32-unknown-unknown` sans modification — aucun IO, aucun async, aucune horloge, aucun `rand`, `serde` + `thiserror` pour seules dépendances, et des budgets explicites (`max_nodes`, `max_solutions`) au lieu de minuteries.
La contrainte « toute la logique métier dans `core`, zéro IO » a rendu la cible WASM gratuite.

## Décision

- **Une crate séparée `crates/wasm`** (`cdylib` + `rlib`), pas la crate `ui` : le module JS et l'app Dioxus sont deux consommateurs parallèles de `core`, aucun ne dépend de l'autre.
- **Les quatre fonctions sont des paires** : une fonction Rust pure (`schedule::generate/verify`, `organigramme::generate/verify`), testée nativement et couverte à 100 %, et une enveloppe `#[wasm_bindgen]` de trois lignes. Toutes les enveloppes vivent dans `boundary.rs`, compilé sous `cfg(target_arch = "wasm32")` seulement — donc absent de la compilation native (ni région non couverte, ni dépendance `wasm-bindgen` native) ; `make static` le passe quand même à clippy sur la cible wasm32.
- **Frontière en `serde-wasm-bindgen`**, sérialiseur `json_compatible` : les entrées et sorties sont des objets JS ordinaires, pas des chaînes JSON à reparser ni des `Map`.
- **Vérifier un organigramme = les deux moitiés à la fois** (choix d'Antoine) : `place` avec *tous* les cours restants épinglés prouve le placement (préalables, plafond de crédits, étés fermés, faisabilité hebdomadaire) au lieu d'en construire un, et `coverage_report` compte les règles ; le rapport rend `{sessions, placement, set_aside, coverage}`.
- **Vérifier suppose une question complète** : un cours sans session épinglée (organigramme) ou un cours sans option choisie (horaire) est une erreur nommant les coupables, jamais un verdict « invalide » — refuser de répondre plutôt que répondre faux.
- `Placement`, `Solution`, `Blocked`, `BlockedReason` et `Completion` gagnent `Serialize` dans `core` (les deux énumérations en kebab-case), seul changement au cœur ; un test épingle les noms JSON publiés.
- L'horizon est **décrit, jamais listé** : l'entrée porte `start` + `study_sessions`, `horizon_sessions` construit les sessions et le rapport les rend — la règle « un été après chaque hiver » reste dans `core`, hors de la vue.
- Budgets par défaut réduits (1 M nœuds, 100 solutions) : la recherche tourne sur le fil JS. La troncature n'est jamais silencieuse (`completion`), et l'appelant peut les relever.

## Alternatives rejetées

- **Exposer depuis `crates/ui`** : mêlerait la sortie WASM de Dioxus et un paquet npm indépendant dans une seule crate, avec des `crate-type` incompatibles.
- **Passer des chaînes JSON** (`fn(&str) -> String`) : simple, mais impose `JSON.parse`/`stringify` des deux côtés et perd le typage `.d.ts` que `wasm-bindgen` génère.
- **Une fonction de vérification dédiée dans `core`** : `place` avec tout épinglé *est* déjà le vérificateur de placement — complet, et il nomme les coupables via `blocked`. Écrire un second chemin risquerait d'en diverger.
- **Ouvrir les étés un par un** (`open_summers` en indices) : l'appelant JS ne connaît pas les indices avant d'avoir l'horizon ; un booléen `summers_open` (tout ou rien) suffit tant que l'UI ne demande pas plus.
