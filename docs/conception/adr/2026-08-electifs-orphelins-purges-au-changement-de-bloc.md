# Les électifs orphelins sont purgés au changement de concentration ou de profil

## Contexte

L'ADR `2026-08-selection-concentration-et-profil-au-panneau` a décidé « changer ne vide rien : la grille placée, les électifs et les épinglages restent ».
Depuis, l'organigramme en continu **auto-place** des électifs de concentration (GMC-3351 pour Robotique) : au passage à « Génie du développement durable », le cours restait affiché et compté (108/120) sans appartenir à aucune règle du nouveau bloc — un chiffre auquel on ne peut plus se fier (contre-test étudiante-cegep 2026-08-20).

## Décision

- `panel::scope_orphans` (pur, testé) : au changement de bloc, les électifs du plan qui sont **listés par le bloc quitté** (obligatoires + règles `List`, une référence résolue à un saut) et que **rien sous la nouvelle portée ne liste** (programme + blocs choisis) sont purgés (`purge_codes`) **dans le même `edit_plan`** que le changement — un seul « Annuler » restaure tout — et annoncés par toast.
- La couverture est celle des listes explicites, jamais du mot-clé « tous les cours » : un cours que seule une entente pourrait rattacher est orphelin tant que l'entente n'existe pas.
- Ceci amende « changer ne vide rien » : la grille et les électifs **hors du bloc quitté** ne bougent toujours pas ; seuls les cours que le bloc quitté avait amenés et que rien ne couvre plus partent avec lui.

## Alternatives rejetées

- **Suivre qui a ajouté l'électif (solveur vs étudiante)** : un champ de provenance dans `Plan` pour distinguer deux cas que la même règle couvre — l'électif d'une étudiante listé seulement par l'ancien bloc est tout autant orphelin, et l'acte est annulable.
- **Laisser et avertir** : le total continuerait de compter un cours rattaché à rien — précisément le mensonge rapporté.
