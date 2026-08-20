# Les rangées de règles sont dédoublonnées avant le rendu

## Contexte

Le répertoire répète parfois un sigle dans la liste d'une règle : la « Règle 1 » du « Cheminement sans concentration » du B-GMC porte GEL-4799 deux fois (tous millésimes), la « Règle 4 » du B-GEX-A26 a trois doublons.
Le panneau rend chaque rangée avec `key: "{row.code}"` ; deux frères à clé identique font paniquer le diff à clés de Dioxus (`keyed siblings must each have a unique key`) — pas au premier rendu, mais à la première rediffusion de la section (changement de concentration avec la règle dépliée, rapport étudiante-cegep du 2026-08-19).
`core` déduplique déjà ses comptages (`BTreeSet` dans `split_selection`) ; seule l'UI lisait la liste brute.

## Décision

Toute liste de codes qui devient des rangées passe par `panel::unique_rows` : première occurrence gagnante, ordre conservé.
Quatre sites : `rule_section`, `bare_section`, les obligatoires d'`uncounted_panel` et `mandatory_section`.

## Alternatives rejetées

- **Corriger les données** (dédoublonner au scraper ou dans `cours.manuel.json`) : le doublon reviendrait au prochain scrape ; la règle « ne jamais inventer ni retrancher ce que la page dit » vaut pour le snapshot — c'est la couche de rendu qui exige l'unicité, c'est elle qui la garantit.
- **Clés composées (`code`-index)** : masque le symptôme mais rend deux rangées identiques cliquables aux effets ambigus (le même cours actionné deux fois).
