# Un cours compte dans une seule règle par portée

Date : 2026-08-23

Amende `2026-07-schema-du-rapport-de-couverture-en-fixtures`.

## Contexte

Une référence croisée (« tous les cours de la Règle 1 du cheminement X ») ou une liste de règles ordinaires peut recouper la liste d'une autre règle de la même concentration ou du même profil.
Avant cette décision, `coverage_report` comptait un cours sélectionné dans **chaque** règle qui le liste : un même cours créditait deux règles à la fois, gonflant le total de la portée au-delà de ce que le répertoire permet (rapport étudiante 2026-08-13, cas du B-GCI où la Règle 2 d'une concentration référence la Règle 1 du cheminement sans concentration, superset de la Règle 1 de la concentration elle-même).

## Décision

`scope_reports` évalue les règles d'une portée dans l'ordre du programme, avec un accumulateur `claimed` qui démarre vide à chaque portée.
Chaque règle *contrainte* (liste + `constraint`) retire de son propre compte les codes déjà `claimed` par une règle précédente de la même portée, les verse dans un nouveau champ `RuleReport.elsewhere` (omis quand vide, `#[serde(skip_serializing_if = "Vec::is_empty")]`), puis ajoute ses propres `counted` à l'accumulateur avant de passer à la règle suivante.
Une règle sans contrainte (« Scolarité préparatoire ») est comptée mais ne réclame jamais de code : elle ne prive aucune règle suivante de sa portée.
Les portées restent indépendantes : l'accumulateur de la concentration et celui du profil sont deux `scope_reports` distincts, donc un cours compte à la fois dans une règle de la concentration et dans une règle du profil.
Le panneau (`mark_counted_elsewhere`) affiche chaque code d'`elsewhere` comme une rangée sélectionnée mais non sélectionnable, sous-titrée « compté dans la Règle N » (le titre de la règle qui le réclame réellement dans la même portée) — la bande de choix (`CourseChoice`, avec son ✕ de retrait) disparaît, mais l'entente (`RuleAttach`) et le crédit sans session (`CreditedToggle`) restent offerts.
Une entente (`granted_program` / `strip_from_other_lists`) ne retire donc plus le cours que des autres règles de la **même portée** que la règle ciblée par la clé `p/…`, `c/…` ou `f/…` : déplacer un cours vers la Règle 2 de la concentration ne le fait plus disparaître de la Règle 1 du profil, puisque les deux portées comptent séparément.

## Alternatives rejetées

- **Le débordement vers la règle suivante** quand la première est pleine — une attribution « intelligente » choisirait à la place de l'étudiante quelle règle réclame le cours; refusé par Antoine le 2026-08-23, l'entente reste le seul moyen explicite de déplacer un cours d'une règle à l'autre.
- **L'exclusivité limitée à la concentration seule** — n'appliquer l'accumulateur qu'à la concentration aurait fait disparaître le cours du profil dès qu'une règle de la concentration le réclamait, alors que les deux portées se comblent indépendamment.
