# Documentation mdBook en français publiée à /docs

Date : 2026-08-09

## Contexte

Le module wasm est consommé par un autre projet JS via l'import par URL depuis GitHub Pages (ADR `2026-08-ci-et-publication-sur-github-pages`), mais rien ne documentait son contrat : pas de rustdoc, pas de README de crate, et `docs/` ne contient que des documents de conception internes.
Toute la documentation du projet est en français.

## Décision

Un livre **mdBook** sous `docs/livre/` (source `src/`, sortie `book/` ignorée par git), publié sur le site Pages existant à **`/docs`**, à côté de `/pkg` et `/data` :

- trois parties : *Guide du consommateur JavaScript* (chargement par URL, les quatre fonctions, schémas, erreurs et budgets), *Architecture* (workspace, frontière WASM, données), *Domaine* (sessions, cours, préalables, programmes, organigramme) ;
- `create-missing = false` : un chapitre manquant au `SUMMARY.md` est une erreur dure, jamais un fichier vide créé en silence ;
- `make docs` construit localement ; le job CI `static` construit le livre sur chaque pull request (le job `deploy` ne tournant pas sur PR, un livre cassé échouerait sinon sans être vu) ; le job `deploy` le construit vers `_site/docs` ;
- les exemples du guide sont adaptés des fixtures de tests, jamais collés : les fixtures parlent la forme core (`"season": "fall"`), l'entrée wasm parle `"session": "a2026"`.

La documentation de l'API vue par l'éditeur du consommateur (JSDoc du `.d.ts`) vit dans les commentaires rustdoc des exports, pas dans le livre (ADR `2026-08-types-typescript-tsify-declaratif`) — le livre explique, le `.d.ts` fait référence.

## Alternatives rejetées

- **rustdoc (`cargo doc`) publié seul** : documente l'API Rust, que personne d'autre ne consomme ; le consommateur est JS et les pages rustdoc ne montrent ni les formes JSON ni le domaine.
- **Un README de crate seulement** : wasm-pack l'embarquerait dans `pkg/`, mais un fichier unique ne porte ni l'architecture ni le domaine, et il faudrait le dupliquer pour le servir sur Pages.
- **Un dépôt ou une branche `gh-pages` dédiés** : le site Pages est déjà assemblé par `ci.yml` depuis `main` ; une deuxième source de publication désynchroniserait doc et code.
