# US-29 — Déplacer une pastille dans une case occupée

**Persona** : Chloé, qui réorganise sa session d'automne.
**Intention** : déposer un cours sur une case déjà prise sans écraser ce qui s'y trouve.

## Préconditions

- Une colonne dont les trois premières cellules sont occupées.

## Scénario

1. Chloé glisse une pastille depuis une autre session vers la première cellule de cette colonne.
2. Elle observe ce qui arrive aux pastilles déjà présentes.

## Résultats attendus

- L'occupant de la cellule cible est poussé d'une rangée vers le bas.
- Si la cellule du dessous est elle aussi occupée, la poussée se propage en cascade jusqu'à trouver une case libre.
- Si la cascade atteint la dernière rangée, une rangée est ajoutée au tableau.
- La pastille déplacée perd ses classes d'erreur avant d'être réinsérée, pour que l'animation ne persiste pas à tort.
- Le menu de section de chaque pastille déplacée est recalculé pour sa nouvelle colonne.

## Repères pour le test e2e

- Avant/après : le sigle de la cellule (0, c) devient celui déplacé, celui d'origine descend en (1, c).
- Le nombre de `tbody tr` augmente si la colonne était pleine.
- Une pastille déplacée depuis une colonne où elle était `cours-non-offert` ne garde pas cette classe si le cours est offert dans la nouvelle colonne.

## Variantes et cas limites

- Déposer une pastille sur la cellule où elle se trouve déjà ne fait rien.
- La cascade est récursive : une colonne pleine sur dix rangées doit se décaler intégralement sans perte.
- Après le déplacement, les rangées vides en trop sont supprimées, sauf si elles contiennent une pastille.
- Le minimum de cinq rangées est toujours respecté.
