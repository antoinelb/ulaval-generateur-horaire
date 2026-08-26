# Le relevé Capsule fixe un plancher au plafond de crédits

## Contexte

Le plafond de crédits par session vaut 17 par défaut (ADR `2026-08-plafond-par-defaut-17-credits`).
Un relevé Capsule réel peut porter des sessions plus chargées — l'A24 du relevé d'exemple a 22 crédits inscrits.
Après import, ces cours sont épinglés à leur session réelle : un plafond sous cette charge rend le propre passé de l'étudiant infaisable et bloque toute proposition.

## Décision

`apply_transcript` calcule `max_session_credits` : la plus lourde charge par session parmi les cours dont le sort final est « épinglé à cette session » — un échec ou la première tentative d'une reprise ne comptent pas.
`apply_to_plan` relève le plafond à ce plancher, sans jamais le baisser : `credit_cap = max(credit_cap, max_session_credits)` — un relevé léger ne réduit pas un réglage que l'étudiant a pu monter lui-même.
Le plafond reste un réglage modifiable ensuite.

## Alternatives rejetées

- **Fixer le plafond exactement à la charge maximale** : écraserait un réglage volontairement plus haut, et un relevé léger abaisserait le défaut de 17.
- **Exempter les sessions passées du plafond dans le solveur** : plus invasif (une contrainte conditionnelle par session dans core) pour le même effet visible, et le plafond affiché mentirait sur ce que la grille contient.
