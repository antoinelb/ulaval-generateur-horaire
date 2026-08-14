# US-83 — B-GPH, concentration « Environnement »

**Persona** : Tristan, en génie physique, orienté vers les géosciences et l'environnement.
**Intention** : combler une concentration dont la liste est plus courte que ses 15 crédits ne le laissent croire.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi **6 cours seulement** (`GCI-1901`, `GCI-3001`, `GCI-3005`, `GGL-2600`, `GGL-2602`, `GGL-2609`).
- `credits_required` vaut 15.

## Scénario

1. Tristan choisit « Environnement ».
2. Il place les cinq premiers cours de la liste.
3. Il lit le bilan.

## Résultats attendus

- C'est la concentration la plus contrainte du programme : six cours pour 15 crédits laissent très peu de latitude.
- Si les six cours valent trois crédits, cinq suffisent et la règle atteint exactement son maximum.
- L'en-tête de section affiche `Environnement : 15 cr. / 15 cr.`

## Repères pour le test e2e

- La carte de la Règle 1 contient exactement 6 `.course-line`.
- Placer les six cours laisse la ligne du journal à `Règle 1 : 15 cr. / 15 cr.` : le plafond tient.

## Variantes et cas limites

- Le B-ANT a aussi une concentration « Environnement » : les tests doivent qualifier le programme avant le titre, jamais chercher la concentration par son seul nom.
- Une liste courte pour un maximum élevé est le cas où les conflits d'horaire mordent le plus : Tristan a peu d'alternatives s'il doit tout prendre à la même session (US-36).
- Les cours `GGL-` sont partagés avec le B-GEX : un étudiant qui change de programme les conserve (US-10).
