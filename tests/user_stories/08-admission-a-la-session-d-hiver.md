# US-08 — Admission à la session d'hiver

**Persona** : Jade, admise au B-GIN à l'hiver 2027.
**Intention** : obtenir une grille qui commence en H27 et non en A26.

Une admission d'hiver décale toute la séquence : la première année se lit H → E → A, et les cours offerts à l'automne seulement se déplacent d'un rang.

## Préconditions

- Programme « B-GIN », millésimes disponibles dans `data/programmes/index.json` (A23 à H27).

## Scénario

1. Jade choisit « B-GIN » puis « H27 » dans « Session d'admission ».
2. Elle constate que les en-têtes de colonnes deviennent H27, E27, A27, H28, …
3. Elle ouvre « Charger un cheminement » pour voir les cheminements types offerts pour H27.

## Résultats attendus

- Changer la session d'admission régénère les en-têtes à partir de la session choisie, en gardant au moins onze colonnes de session.
- **Le changement vide la grille** : toutes les pastilles placées sont supprimées et les cours du panneau redeviennent non placés.
- Le panneau de règles se recharge sur le millésime H27 : titre, crédits exigés et règles peuvent différer de A26.
- La liste de la fenêtre « Charger un cheminement » est filtrée par la session d'admission; s'il n'existe aucun cheminement type pour H27, le message « Aucun cheminement défini pour la session d'admission H27. » s'affiche.

## Repères pour le test e2e

- `#admission-select` liste les millésimes du programme, du plus récent au plus ancien.
- Après changement, le deuxième `thead th` vaut `H27` et la séquence suit `H27, E27, A27, H28`.
- `#programme-subtitle` contient le nom du programme, le millésime et la spécialisation.
- `#modal-cheminements-dynamiques` contient soit des boutons, soit le message d'absence.

## Variantes et cas limites

- La perte silencieuse du travail en cours au changement de millésime est un piège : une confirmation serait souhaitable, ce que le comportement actuel ne fait pas.
- Un programme n'admettant qu'à l'automne (`possible_semester_start: ["A"]`) ne devrait pas proposer de millésime d'hiver; l'index des fichiers, lui, en contient.
- Le B-GEX n'a qu'un millésime (A26) : le menu ne contient qu'une entrée et le changement est un non-événement.
