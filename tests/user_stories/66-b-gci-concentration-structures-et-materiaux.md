# US-66 — B-GCI, concentration « Structures et matériaux »

**Persona** : Josée, en génie civil, qui vise la conception de bâtiments et d'ouvrages d'art.
**Intention** : combler sa concentration à partir de la liste la plus longue des trois.

## Préconditions

Mêmes préconditions de déploiement qu'en US-63.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 12 crédits parmi 12 cours (`FOR-2020`, `GBO-2040`, `GBO-4015`, `GBO-4070`, `GCI-3003`, `GCI-3300`, `GCI-4004`, `GCI-4074`, `GCI-4090`, `GCI-4100`, `GCI-4401`, `GEX-3001`).
- Règle 2 : 3 crédits par référence croisée vers la Règle 1 du cheminement sans concentration.
- `credits_required` vaut 15.

## Scénario

1. Josée choisit « Structures et matériaux ».
2. Elle place quatre cours de la Règle 1, dont deux `GBO-` (génie du bois).
3. Elle lit le bilan.

## Résultats attendus

- La Règle 1 est comblée à 12 crédits avec quatre cours de trois crédits.
- Les cours `GBO-` reçoivent une teinte distincte des `GCI-` : la couleur suit la matière, pas la règle.
- L'en-tête de section plafonne à 15 crédits.

## Repères pour le test e2e

- La carte de la Règle 1 contient 12 `.course-line`.
- Deux pastilles de matières différentes n'ont pas la même couleur de fond.

## Variantes et cas limites

- Le même défaut de référence croisée qu'en US-64 s'applique.
- Cette concentration recoupe fortement le « Profil développement durable » (`GBO-2040`, `GCI-4201`) : un cours placé une fois compte dans les deux règles si les deux sont affichées, ce que l'interface ne permet pas — un profil et une concentration ne peuvent pas être sélectionnés ensemble (US-68).
