# Un cours compte dans une seule règle par portée

## Goal

Un cours sélectionné est compté par la première règle de sa portée qui le liste, apparaît dans les règles suivantes de la même portée comme « déjà compté » (montré sélectionné, non sélectionnable), compte indépendamment dans la concentration et dans le profil, et l'erreur « au-dessus de son maximum » nomme la portée de la règle.

## Out of scope

- Aucune attribution « intelligente » (débordement vers une règle suivante quand la première est pleine) : décision d'Antoine 2026-08-23, strictement la première règle qui liste le cours ; une entente reste le seul moyen de déplacer un cours.
- Les candidats non sélectionnés d'une règle restent tous proposés, même ceux qu'une règle précédente de la portée compterait — pas de filtrage.
- Le solveur de placement (`organigramme.rs`) n'est pas touché.
- Le dépôt JS soeur évolue en parallèle : pas de compatibilité à préserver.

## Constraints

- Toute la logique métier dans `core` (`crates/core/src/rules.rs`), rien dans la vue ; l'UI ne fait que rendre le rapport.
- Les fixtures `tests/fixtures/test_cases/rules/*.json` sont figées : aucune ne doit changer (le nouveau champ est omis quand vide) ; toute nouvelle fixture est dérivée par la référence `tests/reference/solveur_b/verify_rules.py` (mode `fill`), qui doit être mise à jour en miroir et repasser en mode `check` au bit près (ADR `2026-07-reference-b-versionnee-jusqua-larbitrage`).
- Identifiants et clés JSON en anglais ; prose et copie UI en français, sans espace avant `;` `?` `!`, sans point médian.
- Rust : pas de `.unwrap()` nu, pas de `while`, pas de récursion, fonctions courtes ; les commentaires expliquent le *pourquoi*.
- Dioxus 0.7 : lire `.claude/dioxus.md` avant de toucher `crates/ui/src/components/panel.rs` ; règles AIR (INP-3 : l'état porté par le texte, jamais la couleur seule).
- Une décision = un ADR sous `docs/conception/adr/`.
- `make test` doit rester à 100 % de couverture (attention à la double compilation unit/intégration, ADR `2026-07-couverture-par-instanciation-le-plus-petit-ecart`).

## Items

1. `crates/core/src/rules.rs` — `coverage_report` attribue chaque cours sélectionné à la première règle *évaluée* (liste + contrainte) de sa portée qui le liste, dans l'ordre des règles ; les règles suivantes de la même portée le reportent dans un nouveau champ `RuleReport.elsewhere: Vec<String>` (`skip_serializing_if = "Vec::is_empty"`, trié comme `counted`), exclu de `counted` et de `candidates` ; les portées sont indépendantes (un cours compte dans la concentration et dans le profil) ; les règles sans contrainte (« Scolarité préparatoire ») et les obligatoires gardent le comptage global ; tests unitaires : recouvrement R1/R2 compté une fois, indépendance des portées, une règle sans contrainte non affectée.
2. `crates/core/src/rules.rs` — `CoverageError::CreditsOverMax` et `CountOverMax` portent la portée (`scope: Scope`) en plus du titre ; le `Display` la mentionne ; les tests existants qui construisent ces variantes sont ajustés.
3. `tests/reference/solveur_b/verify_rules.py` — la référence Python reproduit l'attribution par portée (et n'émet `elsewhere` que non vide) ; une nouvelle fixture `tests/fixtures/test_cases/rules/concentration-overlap-counted-once.json` sur le B-GCI A26 (concentration « Eau et environnement », quatre cours de la Règle 1 sélectionnés) est dérivée en mode `fill`, vérifiée en mode `check`, et ajoutée à `FIXTURES` dans `crates/core/tests/integration/rules.rs` ; les quinze fixtures existantes restent identiques au bit près.
4. `crates/ui/src/panel.rs` — un nouvel état `RowState::CountedElsewhere` pour les lignes listées dans `elsewhere` : montrée comme sélectionnée, sous-texte « compté dans la Règle N » (le titre retrouvé dans la même portée du rapport, celle dont `counted` contient le code), sans action de choix ; `coverage_error_message` nomme la règle avec sa portée (« Règle 2 de la concentration », « du profil », rien pour le programme) ; tests du modèle.
5. `crates/ui/src/panel.rs` — `granted_program` / `strip_from_other_lists` ne retire plus le cours entendu que des autres règles de la *même portée* que la règle cible, pour qu'il reste compté dans la concentration et dans le profil à la fois ; test.
6. `crates/ui/src/components/panel.rs` + `crates/ui/assets/main.css` — `RowView` rend `CountedElsewhere` avec la classe `panel-course--chosen` mais sans bande de choix (`CourseChoice`), en gardant `RuleAttach` (l'entente est le moyen de déplacer le cours) et `CreditedToggle` ; la taille de police de `.warning` est alignée sur le corps du panneau (`0.8125rem`) au lieu d'hériter de `1rem`.
7. Documentation — deux ADR : `2026-08-un-cours-compte-dans-une-seule-regle-par-portee.md` (attribution première règle stricte, portées indépendantes, ententes désormais locales à la portée, alternatives rejetées : débordement vers la règle suivante, exclusivité concentration seule) et `2026-08-erreur-de-comptage-nommee-par-portee.md` ; `docs/livre/src/domaine/programmes.md` documente `elsewhere` ; `tests/user_stories/64-b-gci-concentration-eau-et-environnement.md` (et 65, 66) attend les cours de la Règle 1 montrés « compté dans la Règle 1 » dans la Règle 2 et un comptage `12/15 cr` après quatre cours.

## Acceptance

- Sur B-GCI A26, concentration « Eau et environnement », profil « Profil développement durable » : placer quatre cours de la Règle 1 donne Règle 1 `✓`, Règle 2 `0/3 cr` avec ces quatre cours affichés sélectionnés et non sélectionnables (« compté dans la Règle 1 »), en-tête `12/15 cr`, aucune bannière « sans comptage » ; `GCI-4201` placé compte à la fois dans la Règle 1 de la concentration et dans la Règle 1 du profil.
- Cinq cours de la Règle 1 placés donnent la bannière « ⚠ Règle 1 de la concentration : les cours sélectionnés y totalisent 15 crédits, au-dessus de son maximum de 12. … », dans la même taille de police que les autres textes du panneau.
- Une entente déplaçant `GCI-3101` vers `c/Règle 2` le fait compter en Règle 2 et toujours dans la Règle 2 du profil.
- `make lint && make test` verts, couverture 100 %, les quinze fixtures `rules/` inchangées, `python tests/reference/solveur_b/verify_rules.py check` sans écart.

## Check

`make lint && make test`
