# US-40 — Préalable simultané (sigle suivi d'une étoile)

**Persona** : Benoît, qui veut prendre deux cours liés dans la même session.
**Intention** : comprendre pourquoi certains cours acceptent leur préalable dans la même colonne et d'autres non.

Un préalable noté `SIGLE *` est un préalable **concomitant** : il peut être suivi en même temps.
Un préalable sans étoile doit être strictement antérieur.

## Préconditions

- `MAT-1910` a pour préalables `MAT-1900 OU MAT-1920*`.

## Scénario

1. Benoît place `MAT-1910` et `MAT-1920` dans la même colonne, `H27`.
2. Il déplace ensuite `MAT-1920` vers `A27`, après `MAT-1910`.

## Résultats attendus

- À l'étape 1, `MAT-1910` n'est pas signalé : le préalable simultané est satisfait par la même colonne.
- Un préalable simultané est aussi satisfait par une colonne antérieure, ou par la colonne « Cours complétés ».
- À l'étape 2, `MAT-1910` est signalé : le préalable simultané n'est satisfait ni par la même colonne ni par une colonne antérieure.
- L'infobulle nomme le sigle manquant avec son étoile, par exemple `MAT-1920 *`.

## Repères pour le test e2e

- Pas de `prerequis-manquants` sur `MAT-1910` quand `MAT-1920` est dans la même colonne.
- Après déplacement, `title` contient `MAT-1920 *`.

## Variantes et cas limites

- Un cours dont **tous** les préalables sont simultanés peut être placé dès la première session, à condition d'y placer aussi ses préalables.
- Une expression mêlant préalables normaux et simultanés doit être évaluée avec les deux règles à la fois.
- L'ordre des rangées dans une colonne n'a aucune importance : seule la colonne compte.
- Le mécanisme repose sur la présence littérale de `*` dans le texte des préalables; un changement de notation côté ULaval le casserait silencieusement.
