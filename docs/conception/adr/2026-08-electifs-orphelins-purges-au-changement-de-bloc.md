# Les électifs orphelins sont purgés au changement de concentration ou de profil

## Contexte

L'ADR `2026-08-selection-concentration-et-profil-au-panneau` a décidé « changer ne vide rien : la grille placée, les électifs et les épinglages restent ».
Depuis, l'organigramme en continu **auto-place** des électifs de concentration (GMC-3351 pour Robotique) : au passage à « Génie du développement durable », le cours restait affiché et compté (108/120) sans appartenir à aucune règle du nouveau bloc — un chiffre auquel on ne peut plus se fier (contre-test étudiante-cegep 2026-08-20).

## Décision

- `panel::scope_orphans` (pur, testé) : au changement de bloc, les électifs du plan qui sont **listés par le bloc quitté** (obligatoires + règles `List`, une référence résolue à un saut) et que **rien sous la nouvelle portée ne liste** (programme + blocs choisis) sont purgés (`purge_codes`) **dans le même `edit_plan`** que le changement — un seul « Annuler » restaure tout — et annoncés par toast.
- La couverture est celle des listes explicites, jamais du mot-clé « tous les cours » : un cours que seule une entente pourrait rattacher est orphelin tant que l'entente n'existe pas.
- Ceci amende « changer ne vide rien » : la grille et les électifs **hors du bloc quitté** ne bougent toujours pas ; seuls les cours que le bloc quitté avait amenés et que rien ne couvre plus partent avec lui.

## Amendements

- `2026-08-electifs-choisis-sous-le-bloc-partent-avec-lui` (2026-08-27) : la couverture seule laissait passer FOR-2020, choisi sous un bloc et listé par le suivant. Un électif **choisi sous** le bloc quittant part désormais avec lui, couvert ou non ; `scope_orphans` reste le filet des plans sans étiquettes. L'alternative ci-dessous est **renversée**.
- `2026-08-obligatoire-de-bloc-purge-meme-liste-ailleurs` (2026-08-29) : la couverture gardait GMC-3351, obligatoire de « Robotique » auto-placé (donc sans étiquette) et listé par la Règle 1 du bloc neutre. Un cours que le bloc quittant avait en `mandatory` ne survit désormais que si la nouvelle portée l'**impose** aussi ; la couverture par liste ne juge plus que ce que le bloc *offrait*.

## Alternatives rejetées

- **Suivre qui a ajouté l'électif (solveur vs étudiante)** : un champ de provenance dans `Plan` pour distinguer deux cas que la même règle couvre — l'électif d'une étudiante listé seulement par l'ancien bloc est tout autant orphelin, et l'acte est annulable. *(Renversé le 2026-08-27 : ce n'est pas « qui » mais « sous quel bloc » qu'il fallait suivre.)*
- **Laisser et avertir** : le total continuerait de compter un cours rattaché à rien — précisément le mensonge rapporté.
