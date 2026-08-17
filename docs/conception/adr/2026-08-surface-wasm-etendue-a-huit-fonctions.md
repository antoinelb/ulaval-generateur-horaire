# Surface wasm étendue à huit fonctions

Date : 2026-08-15 (amende `2026-08-module-wasm-quatre-fonctions-js`)

## Contexte

L'interface JS (`CORRECTIFS-AMONT.md` item 12) ne pouvait offrir ni les puces de sessions admissibles, ni le feedback statique par ligne de cours, et gardait un bilan de crédits transitoire ainsi qu'un double encodage de l'horizon — tout cela existait déjà dans core, mais `crates/wasm` n'exposait que les quatre fonctions de génération/vérification.
Corollaire : le bilan « total gonflé » des règles négociées (item 10) et les règles par référence croisée « vides » (item 11) sont des artefacts du calcul transitoire côté JS — `coverage_report` de core rapporte déjà une règle négociée en `reported` (jamais bloquante, `raw` garanti) et résout les références à chaque appel.

## Décisions

Quatre exports s'ajoutent aux quatre existants, même motif (fonction pure testée en natif, glue `boundary.rs` wasm32-only, entrée à `deny_unknown_fields`) :

- `admissible_sessions(input: OrganigrammeInput, code)` → `number[]` (1-based, la forme de `pinned`) — sondes de `place` par session (core `admissible_sessions`) ; `solve` et lui partagent la construction de la requête (`with_request`).
- `prerequisites_met({course, satisfied, credits})` → `{met, assumed}` — `PrereqStatus` de core aplati pour JS (core ne sérialise pas ce type ; `Unmet` devient `met: false`, `assumed` vide).
- `coverage_report({program, concentration?, profile?, selection, courses})` → `CoverageReport` — le bilan seul, sur une grille **partielle** ; `verify_organigramme` continue d'exiger un placement complet (question entière ≠ bilan partiel).
- `horizon_sessions({start, study_sessions})` → `Semester[]` (« A26 », « H27 », « E27 », …) — l'arithmétique de calendrier de `session_semesters` déménage de `crates/ui/state.rs` vers `core::intake` : une seule autorité, l'UI Dioxus la ré-exporte.

Les codes d'entrée passent par `normalize_codes` (majuscules, doublon = erreur remontée).

## Alternatives rejetées

- Assouplir `verify_organigramme` pour accepter une grille partielle : le refus « un cours sans session = question incomplète, jamais un faux verdict » est un choix de conception, pas un manque.
- Sérialiser `PrereqStatus` dans core : personne d'autre n'en a besoin ; l'aplatissement appartient à la frontière JS.
- Dénormaliser les règles par référence croisée au scrape : dupliquerait 25 sigles dans chaque concentration et perdrait le lien « même liste que » ; la résolution à l'appel suffit.
