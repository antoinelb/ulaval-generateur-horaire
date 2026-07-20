# Un scrape complet supprime les snapshots qu'il ne produit plus

Date : 2026-07-19

## Contexte

`write_courses` écrivait les fichiers de session sans jamais retirer ceux qu'un run ne produisait plus.
Un cours change pourtant de session quand son offre change : GCI-7077 ne figurait dans `a2020.json` que parce que son bloc Automne 2026 était illisible (mode « Hybride »).
Le mode corrigé, il est passé dans `a2026.json` — et s'est retrouvé dans **les deux**, `a2020.json` n'ayant jamais été effacé.

`data/cours/` étant commité, un tel fichier survit indéfiniment et annonce aux étudiants une session où le cours n'est pas offert.

## Décision

- **Seul un run complet** (sans `--subjects`) supprime les fichiers `data/cours/*.json` qu'il n'a pas produits.
  Lui seul a parcouru tout le catalogue, donc lui seul peut juger qu'une session n'existe plus.
  Le cron est toujours complet, donc les snapshots commités restent exacts.
- **Un run `--subjects` ne supprime rien.**
  Il n'a vu qu'une partie du catalogue : les sessions des autres matières lui sont invisibles, et les juger périmées effacerait un snapshot complet d'une seule commande étroite.
- **`{session}.manuel.json` est épargné** — contribution manuelle, jamais touchée par le scraper (ADR `2026-07-contribution-de-cours-manuels`).
  De même, tout fichier qui n'est pas un `.json` reste en place.
- **Lister avant d'écrire, supprimer après** : rien n'est effacé tant que les nouveaux snapshots ne sont pas en place, si bien qu'un run interrompu ne laisse aucun trou.

## Alternatives rejetées

- **Supprimer toujours** : une seule règle, mais `--subjects gex` effacerait alors tous les fichiers de session sans cours GEX.
- **Interdire à un run scopé d'écrire dans `data/`** : sûr, mais le livrable du jalon 1 *est* un snapshot GEX.
- **Ne rien supprimer, documenter le résidu** : le doublon est silencieux, et une session périmée serait servie telle quelle.

## Plafond connu

Un run complet dont beaucoup de pages échouent peut vider une session et donc supprimer son fichier, alors que la donnée existe toujours en amont.
C'est la même faille que le plancher de rétrécissement à 90 % prévu pour `data/catalogue.json` et pas encore implémenté (ADR `2026-07-catalogue-artefact-commite`) : les deux se traiteront ensemble.
