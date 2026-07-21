# Pavillons retirés du modèle

Date : 2026-07-21

## Contexte

Le plan listait « distance entre pavillons » parmi les préférences de classement des combinaisons (v2 / jalon 10) et « pavillons » parmi les données extraites par section.
Classer selon la distance entre pavillons exige des distances ou des temps de marche inter-bâtiments fiables, que les pages de l'ULaval ne fournissent pas.
Le nom du pavillon, lui, figure bien sur la page ; mais sans modèle de distance il ne servait qu'à ce critère, et il est jugé préférable de ne pas transporter une donnée sans usage plutôt que de la garder « au cas où ».
Les fixtures gelées n'ont d'ailleurs jamais encodé le pavillon (`slots` = `{day, start, end}`) : la prose du plan avait dérivé devant le contrat réel des tests.

## Décision

- « distance entre pavillons » est retirée des préférences de classement (description v2 et jalon 10 de `docs/project_plan.md`).
- Le pavillon est retiré du modèle de données par section et des listes de vocabulaire du domaine (`docs/project_plan.md` et `CLAUDE.md`).
- Aucun changement de fixture ni de parseur : les fixtures n'encodaient déjà pas de pavillon, donc la prose du plan est simplement réalignée sur le contrat des tests.
- Les documents de `docs/conception/` (historique) ne sont pas réécrits : le plan les supplante.

## Alternatives rejetées

- **Garder le pavillon comme donnée affichée sans classement** : écarté ; sans modèle de distance il ne pesait dans aucun critère, et Antoine a choisi de le retirer entièrement.
- **Construire un modèle de distance / temps de marche** : écarté faute de source de données fiable.
