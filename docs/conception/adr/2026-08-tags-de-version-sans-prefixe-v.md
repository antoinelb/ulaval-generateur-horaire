# Les tags de version n'ont plus de préfixe `v`

Date : 2026-08-24

## Contexte

Les deux premiers tags (`v0.1.0`, `v0.2.0`) portaient le préfixe `v`, et `release.yml`/`ci.yml` ne se déclenchaient que sur `v*`.
Le skill `bump` (nouvellement créé pour automatiser le cycle version/commit/tag) pose désormais des tags de la forme `2.1.3`, sans préfixe — convention plus courante pour un dépôt sans autre outil qui dépendrait du `v` (pas de crates.io, pas de package npm publié).

## Décision

Les tags de version sont désormais `X.Y.Z`, sans `v`.
`release.yml` et `ci.yml` déclenchent sur le pattern `[0-9]*.[0-9]*.[0-9]*` au lieu de `v*`.
`v0.1.0` a été recréé sous `0.1.0` (même commit, même release GitHub republiée) ; `v0.2.0` a été supprimé sans remplacement (la release `0.1.0` reste la seule référence historique).
Ceci amende `2026-08-release-semver-du-binaire-du-scraper` sur ce seul point : le tag de déclenchement, pas le mécanisme de release lui-même.

## Alternatives rejetées

- **Garder `v*` et faire poser des tags préfixés par le skill `bump`** : rejeté par l'utilisateur, sans préfixe est la convention voulue.
- **Faire matcher les deux formes (`v*` et `X.Y.Z`)** : cruft inutile une fois que plus aucun tag `v*` n'existe dans le dépôt ; matcher un seul format garde le workflow simple.
