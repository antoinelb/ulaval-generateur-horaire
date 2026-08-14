# US-16 — Cours au choix et options à déterminer

**Persona** : Alice, au B-GEX, qui n'a pas encore choisi ses cours à option de quatrième année.
**Intention** : réserver les cases dans sa grille sans s'engager sur un sigle précis.

Les pseudo-cours de `b-gex/cours/cours-hors-catalogue.csv` servent exactement à cela : `OPT-ION1` à `OPT-ION4` (cours à option à déterminer) et `AUC-HOIX` (cours de premier cycle au choix).

## Préconditions

- Programme « B-GEX », cheminement type A26 chargé.

## Scénario

1. Alice glisse `OPT-ION1` et `OPT-ION2` en A29.
2. Elle glisse `AUC-HOIX` en H30.
3. Plus tard, elle remplace `OPT-ION1` par `GAE-3006`, un vrai cours de la Règle 3.

## Résultats attendus

- Les pseudo-cours portent 3 crédits chacun et comptent au bilan.
- Ils n'ont ni horaire ni préalables : aucun conflit, aucune erreur de préalable.
- Le remplacement se fait en retirant la pastille du pseudo-cours vers le panneau, puis en glissant le vrai cours.
- La « Règle 5 » du B-GEX A26, dont les cours valent `any` (n'importe quel cours de premier cycle), n'affiche aucune liste : c'est `AUC-HOIX` qui la représente en pratique.

## Repères pour le test e2e

- `.rule-card` correspondant à la Règle 5 contient le texte `Aucun cours défini pour cette règle.`
- `.dropped-tile[data-code="OPT-ION1"][data-credits="3"]` existe.

## Variantes et cas limites

- **Comportement observé à trancher** : ces pseudo-cours sont marqués `cours-non-offert` dans toute colonne de session, faute de saison enregistrée. Le même faux positif qu'en US-03.
- Un pseudo-cours n'est pas dans `data/cours.json` : il disparaît si le fichier hors catalogue du programme est absent ou mal formé.
- Les crédits d'un cours au choix devraient idéalement compter dans la règle correspondante; ils comptent aujourd'hui uniquement si le pseudo-sigle figure dans la liste de cette règle.
