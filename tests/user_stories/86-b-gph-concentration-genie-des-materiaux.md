# US-86 — B-GPH, concentration « Génie des matériaux »

**Persona** : Sandra, en génie physique, orientée vers la science des matériaux.
**Intention** : combler sa concentration à partir d'une liste presque entièrement d'une seule matière.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 7 cours (`GML-2003`, `GML-2007`, `GML-2251`, `GML-3001`, `GML-3020`, `GML-4103`, `GML-4150`).
- `credits_required` vaut 15.

## Scénario

1. Sandra choisit « Génie des matériaux ».
2. Elle place cinq des sept cours.
3. Elle lit le bilan.

## Résultats attendus

- Les sept cours partagent la matière `GML` : la carte est monochrome, comme celle de la concentration « Électricité » (US-82).
- L'en-tête de section atteint `15 cr. / 15 cr.` avec cinq cours de trois crédits.
- Sept cours pour 15 crédits laissent deux cours de latitude.

## Repères pour le test e2e

- La carte de la Règle 1 contient 7 `.course-line`, toutes de préfixe `GML-`.
- Toutes les pastilles placées ont la même couleur de fond.

## Variantes et cas limites

- `GML-3020` est partagé avec « Aéronautique et aérospatiale » (US-81) et `GML-4103` avec « Génie médical et biophotonique » (US-84).
- Une concentration monochrome est le cas où le sigle seul porte toute l'information : les pastilles n'affichant plus de titre court depuis l'ADR `2026-08-couleurs-derivees-de-la-matiere`, l'infobulle et le panneau sont les seules sources du titre.
