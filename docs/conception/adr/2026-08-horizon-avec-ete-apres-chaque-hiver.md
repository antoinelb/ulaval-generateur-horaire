# L'horizon insère un été après chaque hiver, le dernier inclus

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

Le solveur B prend une liste explicite de saisons (`2026-07-schema-des-fixtures-de-placement` a rejeté l'horizon numérique), mais le seul générateur, `alternating_sessions`, refusait d'émettre un été.
Avec les stages restreints aux étés (`2026-08-stage-place-en-ete-sauf-epinglage`), l'horizon doit en contenir — sinon tout stage bloque.

## Décision

- `intake::alternating_sessions` (aucun appelant) est remplacé par `horizon_sessions(start, study_sessions)` : la même alternance A/H depuis `start` (un départ été coule vers l'automne, comme avant), puis un été inséré **après chaque hiver, le dernier inclus** — un stage trouve toujours un été, y compris après la dernière session d'études.
- `study_sessions` compte **seulement les sessions d'études** (l'alternance) ; les étés insérés viennent en sus. `horizon_sessions(Fall, 4)` = `[A, H, E, A, H, E]`.
- Un départ été (session `e<année>`) compte comme session d'études, apparaît en tête et obéit à `open_summers` comme tout été.
- Nouveau `summer_indices(sessions)` : les indices 1-based des étés — la forme que parlent `open_summers` et `pinned` ; l'UI s'en sert pour offrir les cases « ouvrir cet été ».
- Le solveur garde la liste explicite : l'insertion est une affaire d'intake, pas de recherche.

## Alternatives rejetées

- **Paramètre en années** (chaque année produit A, H, E) : rompt avec le schéma de fixtures gelé et avec le vocabulaire du bac (8 sessions d'études, pas 4 ans).
- **L'appelant liste tout lui-même** : chaque consommateur réinventerait l'insertion des étés ; le générateur est le seul endroit où la convention vit.
- **Été seulement sur demande** (drapeau « avec étés ») : un horizon sans été fait bloquer tout stage — le cas par défaut serait le cas cassé.
- **Pas d'été après le dernier hiver** : le stage obligatoire se fait souvent après la dernière session d'études ; le tronquer forcerait un épinglage artificiel.
