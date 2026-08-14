# US-82 — B-GPH, concentration « Électricité, électronique et puissance »

**Persona** : Coralie, en génie physique, orientée vers les systèmes électriques.
**Intention** : combler sa concentration et vérifier que le titre s'affiche correctement.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 8 cours (`GEL-2002`, `GEL-2003`, `GEL-3000`, `GEL-3001`, `GEL-4102`, `GEL-4150`, `GEL-4151`, `GEL-4152`).
- `credits_required` vaut 15.

## Scénario

1. Coralie choisit cette concentration.
2. Elle place cinq cours.
3. Elle vérifie le titre dans le menu, l'en-tête de section et l'en-tête de carte.

## Résultats attendus

- Le titre commence par une majuscule accentuée (`É`) et contient des virgules : il doit s'afficher tel quel partout, sans troncature ni échappement visible.
- Les huit cours partagent la matière `GEL` et donc la même teinte : la grille de cette concentration est monochrome, ce qui est le comportement attendu du calcul de couleur par matière.
- L'en-tête de section atteint `15 cr. / 15 cr.`

## Repères pour le test e2e

- L'option de `#cheminement-select` porte exactement `Électricité, électronique et puissance`.
- Toutes les `.dropped-tile` de cette concentration ont la même couleur de fond.

## Variantes et cas limites

- Une concentration entièrement monochrome rend la grille moins lisible : c'est une conséquence assumée du choix de dériver la couleur de la matière (ADR `2026-08-couleurs-derivees-de-la-matiere`), pas un défaut d'affichage.
- Les cours `GEL-` sont partagés avec les concentrations « Photonique » (US-85) et « Signaux et communications » (US-87) : changer de concentration ne les repeint pas.
