# Les préalables corrigés du B-GCI A26 restent propres à ce millésime

Date : 2026-08-23

## Contexte

Le programme B-GCI A26 remplace `GCI-1009` par `GCI-1011` et `GCI-3001` par `GCI-3008` dans son tronc commun.
Trois préalables du répertoire courant empêchaient donc des cours obligatoires de ce millésime d'être placés.

## Décision

`data/cours.manuel.json` porte sous `vintages.A26.prerequisites` les expressions propres à `GCI-2003`, `GCI-2006` et `GCI-3333`.
Les autres millésimes continuent de lire le répertoire courant.

## Alternatives rejetées

- **Modifier `data/cours.json`** — le scraper écraserait la correction et tous les millésimes changeraient avec elle.
- **Ajouter les anciens sigles au cheminement** — ces cours n'appartiennent pas au programme A26.
