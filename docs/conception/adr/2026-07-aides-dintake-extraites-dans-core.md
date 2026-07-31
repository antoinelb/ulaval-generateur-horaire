# Aides d'intake extraites du CLI dans `core::intake`

## Contexte

Le harnais UI (`crates/ui-debug`) a besoin des mêmes préparatifs que le CLI : parser une session (`a2026`), normaliser les codes, résoudre l'offre à travers les équivalences, sélectionner les cours connus du snapshot, parser les épinglages, sommer les crédits.
Ces fonctions étaient privées dans `crates/cli/src/cli.rs` avec des erreurs `anyhow`.

## Décision

Les neuf fonctions pures (`parse_session`, `alternating_sessions`, `normalize_codes`, `parse_pins`, `course_list`, `select_known`, `select_courses`, `effective_course`, `credit_total`) déménagent dans `crates/core/src/intake.rs` avec une erreur typée `IntakeError` (thiserror).
La résolution d'équivalences est de la logique métier, pas de la glue : l'invariant « toute logique métier dans `core`, aucune dans la vue » impose ce déplacement, et CLI comme UI partagent désormais une seule implémentation.
Les textes de `IntakeError` reprennent mot à mot les messages `anyhow` d'origine (les tests e2e du CLI les assertent) ; `select_courses` porte une exception : l'erreur « not offered » ne nomme plus la session (que `core` ne connaît pas), c'est l'appelant qui la reformule.
Les tests unitaires correspondants migrent en tests *inline* dans `intake.rs` (contrainte de couverture par instanciation, ADR `2026-07-couverture-par-instanciation-le-plus-petit-ecart`).

## Alternatives rejetées

- Dupliquer les ~40 lignes dans `ui-debug` : deux copies de la logique d'équivalences à faire dériver.
- Un crate « harness » partagé entre CLI et UI : un troisième crate pour neuf fonctions pures que `core` peut porter sans dépendance nouvelle.
