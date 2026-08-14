# US-61 — B-ANT, concentration « Études autochtones »

**Persona** : Kevin, au baccalauréat en anthropologie, orienté vers les études autochtones.
**Intention** : vérifier que passer d'une concentration à l'autre change bien la liste des cours sans toucher sa grille.

## Préconditions

Mêmes préconditions de déploiement qu'en US-60.

## Ce que la concentration ajoute

- Trois cours obligatoires : `ANT-1500`, `ANT-1501`, `ANT-2307`.
- Règle 1 : 3 crédits parmi les 16 cours de méthodes, la même liste que la concentration « Environnement » mais avec un minimum de 3 au lieu de 9.
- Règle 2 : 30 à 36 crédits parmi 45 cours propres aux études autochtones (`ANT-1502`, `ANT-1701`, `ANT-2312`…).
- Règle 3 : 0 à 6 crédits de langue, identique à l'autre concentration.
- Règle 4 : 9 à 18 crédits hors discipline (`ARL-1101`, `CRI-2110`, `FOR-4045`…).
- `credits_required` vaut 60.

## Scénario

1. Kevin charge le B-ANT en concentration « Environnement » et place trois cours.
2. Il passe à « Études autochtones ».
3. Il compare les deux panneaux.

## Résultats attendus

- Le panneau change complètement : `ANT-1500`, `ANT-1501` et `ANT-2307` apparaissent en cours obligatoires de la concentration.
- La Règle 1 partage sa liste avec l'autre concentration mais affiche un minimum différent : les deux règles ne sont pas interchangeables.
- La grille n'est pas vidée : les trois cours placés restent en place.
- Un cours placé qui n'appartient plus à aucune règle de la nouvelle concentration cesse de compter au bilan tout en restant affiché.

## Repères pour le test e2e

- Le nombre de `.dropped-tile` est identique avant et après le changement.
- Le nombre de `.rule-card` change.
- La couleur de fond des pastilles est inchangée : les teintes sont attribuées sur tous les sigles du fichier, concentrations comprises.

## Variantes et cas limites

- Les cours communs aux deux concentrations gardent la même couleur, puisque la teinte dépend de la matière et non de la règle.
- Une concentration à 45 cours dans une seule règle produit une carte très longue : la recherche du panneau est le seul moyen de s'y retrouver (US-33).
