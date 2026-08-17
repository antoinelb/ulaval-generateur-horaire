# Fusion de `ui-calculations` dans `wasm` : un crate de frontière, deux surfaces

## Contexte

Deux crates portaient une frontière navigateur au-dessus de `core`, et réimplémentaient la même orchestration du solveur B :

- `crates/wasm` — huit fonctions `JsValue` pour l'interface JavaScript (`grille-de-cheminement-interactive`) ;
- `crates/ui-calculations` — un protocole JSON (`place` / `verify` / `admissible-sessions`) pour le worker de l'app Dioxus, plus deux modules purs (`credits`, `merge`) appelés nativement par `ui`.

La duplication était littérale : `intake()` identique jusqu'au commentaire, même chaîne d'erreur « verification needs a session for every course left to place », même règle d'étés, même assemblage de `PlacementRequest`.
`PlaceQuery` était `OrganigrammeInput` moins `courses` ; `protocol::Report` était `OrganigrammeReport` moins `coverage`.
Environ 150 lignes d'orchestration et deux suites de tests prouvaient deux fois le même comportement.

L'ADR `2026-08-crate-ui-calculations-et-worker` avait écarté la réutilisation du paquet pour une raison technique exacte — ses fonctions prennent le snapshot *dans chaque appel*.
Cette raison a cessé d'être un argument contre la fusion le jour où l'interface JavaScript s'est mise à en souffrir elle aussi : elle expédie les 8 834 cours à chaque appel.
Le correctif vivait dans l'autre crate ; la fusion le rend disponible aux deux (ADR `2026-08-snapshot-en-cache-dans-le-module-wasm`).

## Décision

Un seul crate de frontière, `crates/wasm`, décidé avec Antoine (2026-08-17) :

- **`courses` devient un paramètre de fonction**, plus un champ des structs d'entrée.
  C'est le geste qui fait tomber la duplication : `organigramme::generate/verify/admissible(input, courses)` sert les deux surfaces, `PlaceQuery` disparaît au profit d'`OrganigrammeInput`, et `protocol::handle` n'est plus qu'un aiguillage.
  Le champ `courses` reste au format de fil, en `Option`, lu par la seule frontière.
- **Deux frontières dans un `boundary.rs`**, wasm32 seulement : les huit exports `JsValue` et le couple `init_snapshot` / `handle_message`, partageant le même `thread_local!` de catalogue.
- **Les modules purs suivent** : `credits` et `merge` habitent le crate fusionné, que `ui` consomme en rlib.
- **Le nom du paquet ne bouge pas** (`ulaval-scheduler-wasm`) : il fixe `ulaval_scheduler_wasm.js`, l'URL codée dans le `js/config.js` du dépôt frontend.
  Un nom plus juste coûterait une coordination de déploiement pour un gain cosmétique.
- **Un seul artefact** : `make wasm` publie le paquet, `make ui-calc` construit le même crate dans les assets du `ui`. Une passe clippy wasm32 en moins.

`protocol.rs` passe de 524 à 250 lignes, tests compris ; ses tests ne prouvent plus que l'enveloppe (l'id qui revient, le refus qui ne disparaît pas), les comportements de placement étant prouvés une fois dans `organigramme`.

Conséquence assumée : `verify` par le protocole calcule désormais la couverture des règles, que l'app Dioxus compte aussi de son côté (ADR `2026-08-verification-automatique-du-cheminement`).
`coverage_report` est un comptage sans recherche — 0,4 ms sur le catalogue en cache — et la réponse grossit d'un objet. Un drapeau pour l'éviter coûterait plus que le calcul.

## Alternatives rejetées

- **Garder deux crates et ne dédupliquer que l'orchestration** (`ui-calculations` dépendant de `wasm` en rlib) : la duplication tombait, mais il restait deux crates, deux paquets à construire, deux passes clippy — et le cache de snapshot serait resté du seul côté Dioxus.
- **Renommer le crate fusionné** (`calc`, `frontiere`) : le nom `wasm` décrit mal un crate dont deux modules sont appelés nativement, mais le renommer change le nom du fichier publié et donc l'URL que le dépôt frontend a codée. Le nom exact vaut moins qu'un déploiement sans coordination.
- **Supprimer le `ui` Dioxus** puisque l'interface JavaScript est le frontend livré : écarté par Antoine (2026-08-17), les deux frontends vivent en parallèle.
