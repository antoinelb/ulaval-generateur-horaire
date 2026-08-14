# Architecture du crate `ui` : modules purs natifs, vue wasm32-only

## Contexte

La vraie UI (jalons 3–9) démarre dans `crates/ui` (ADR `2026-07-retrait-des-harnais-cli-et-ui-debug`), sous deux contraintes qui tirent en sens opposés : `make test` exige 100 % de couverture native, et le rsx Dioxus ne rend rien hors navigateur ; les dépendances gloo/web-sys ne compilent utilement que sous wasm32.

## Décision

- **Le crate est une lib + un `main.rs` lanceur.** Les modules purs — `data` (parse/fusion/provenance), `state` (Plan/View/History, arithmétique de sessions), `persist` (localStorage versionné, codec URL, journal), `solve` (orchestration des solveurs, récupération d'erreurs), `present` (géométrie de grille, libellés, erreurs 5 parties) — compilent et se testent nativement à 100 %.
- **La vue entière est wasm32-only** : `browser.rs` (IO : fetch, localStorage, timers) et `components/` (rsx) n'existent que sous `cfg(target_arch = "wasm32")` — le partage exact de `wasm/src/boundary.rs` appliqué au crate entier. `make static` les lint sur la cible wasm32 ; le regex de couverture exclut `components/` (le rsx est mécanique : chaque `if`/`for` y est nourri par une valeur d'un module pur testé).
- **La règle de la vue** : un composant lit des contextes (`Signal<Plan>`, `Signal<View>`, `Signal<History>`, alertes, snapshot), appelle des fonctions pures dans des mémos, et route chaque mutation par `edit_plan` → `state::apply` (étiquetée, annulable — ACT-2, aucun dialogue de confirmation).
- **`weekly_schedule` est totale** : `schedule_report` refuse en bloc dès qu'un cours est indessinable, alors l'orchestration retire le coupable nommé, le liste avec sa raison française (« horaire pas encore publié », « absent du catalogue actuel »…) et réessaie — borné par le nombre de causes retirables. Échec partiel = rendu partiel (ERR-5), jamais d'écran vide, jamais de perte muette.
- **Leçon de terrain (emprunts de signaux)** : un `signal.read()` temporaire vivant dans la condition d'un `if` ou l'expression d'un `match` survit jusqu'à la fin de l'instruction ; y appeler `signal.write()` panique (`AlreadyBorrowed`). Règle : matérialiser la lecture dans un `let` avant toute écriture — deux occurrences corrigées le jour même (panel, grid).

## Alternatives rejetées

- **Binaire pur avec `#[allow(dead_code)]` natif** : les types purs paraissent morts quand la vue est cfg-out ; une lib fait de chaque `pub` une API, pas du code mort.
- **Couvrir le rsx nativement** : il ne rend rien hors navigateur ; la couverture y serait un théâtre. La discipline est ailleurs : aucune logique dans le rsx.
- **Un cache d'horaires par session** : `weekly_schedule` coûte quelques millisecondes ; les mémos Dioxus (coupure par `PartialEq`) suffisent.
