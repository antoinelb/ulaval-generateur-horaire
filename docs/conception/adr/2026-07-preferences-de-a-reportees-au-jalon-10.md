# Les opérations de préférence de A sont reportées au jalon 10

Date : 2026-07-28

## Contexte

La Phase 0 de `docs/next_steps.md` listait `before_noon_free`, `has_midday_gap` et `day_transitions` « signatures d'abord, sémantique à préciser avec le classement de A ».
Or les conventions du dépôt interdisent une signature sans corps testé : `make test` exige 100 % de couverture et clippy `-D warnings` refuse le code mort.
Une signature ne peut donc exister qu'accompagnée d'une sémantique implémentée — précisément ce que la ligne disait ne pas vouloir figer.

## Décision

Rien n'est écrit en Phase 0 : ni signature, ni corps.
Les trois opérations naissent au jalon 10 (classement par préférences), où leur sémantique se calibre contre des données réelles, comme prévu par « Encore à planifier ».
La ligne de `next_steps.md` est annotée en ce sens plutôt que cochée.

## Alternatives rejetées

- **Implémenter une sémantique provisoire testée** : dérisquait la représentation `WeekMask` contre l'itération par jour, mais figeait des choix (fenêtre de la pause dîner, définition d'une transition) que le jalon 10 doit calibrer — du rework garanti pour un risque de représentation faible (le bitset par jour est un simple découpage d'indices).
- **Signatures avec `todo!()` exclues de la couverture** : un 100 % par non-mesure, exactement ce que `2026-07-couverture-100-et-frontiere-io` rejette.
