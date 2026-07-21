# Le cycle préuniversitaire n'existe que pour un cours

Date : 2026-07-21

## Contexte

En réintégrant les cours `0xxx` (ADR `2026-07-cours-dappoint-reintegres`), on découvre que leur page déclare le cycle **« Préuniversitaire »** (confirmé sur `CHM-0150`), une valeur qu'`cycle_level` ne connaît pas : sans traitement, chaque `0xxx` deviendrait une erreur d'analyse dure, pas un cours.

Le type `Cycle` (`First`, `Second`) est partagé par `Course` et `Program`.
Or un **programme** n'est jamais préuniversitaire : un diplôme est un baccalauréat, une maîtrise, un doctorat.
Le préuniversitaire est un niveau propre au **cours**.

## Décision

Un type distinct **`CourseCycle`** (`Preuniversity` = 0, `First` = 1, `Second` = 2) porte le cycle d'un cours ; `common::Cycle` (`First`, `Second`) reste celui d'un programme.

`Course.cycle` devient un `CourseCycle`.
`cycle_level` associe « Préuniversitaire » au niveau 0 ; `parse_cycle` rend alors `Some(CourseCycle::Preuniversity)` — dans le périmètre.
Le parseur de programme est inchangé : ni le type ni la désérialisation ne permettent un programme préuniversitaire.

## Conséquences

Les instantanés de cours `0xxx` portent `"cycle": 0`.
Les cours de premier et deuxième cycle sérialisent toujours `1`/`2` : les fixtures existantes sont inchangées.

L'empreinte de périmètre du cache (`scope_tag`), lue sur les niveaux qu'accepte le cycle du cours, passe à « 0,1,2 ».

## Alternatives rejetées

- **Un seul `Cycle` partagé, avec une variante `Preuniversity`** : le parseur de programme ne l'émettrait jamais, mais le type autoriserait un `programme.json` édité à la main à se dire préuniversitaire. Le périmètre demandé est « cours seulement », que seule la séparation des types garantit.
- **Faire correspondre « Préuniversitaire » au premier cycle** : mentirait sur la nature du cours (il ne compte pour aucun crédit de programme) et rendrait un cours d'appoint indiscernable d'un cours de 1er cycle dans l'instantané.
