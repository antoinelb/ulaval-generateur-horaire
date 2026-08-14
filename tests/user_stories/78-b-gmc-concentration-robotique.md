# US-78 — B-GMC, concentration « Robotique »

**Persona** : Pénélope, en génie mécanique, qui veut concevoir des robots industriels.
**Intention** : combler les 18 crédits de sa concentration.

## Préconditions

- Programme « B-GMC », session d'admission « A26 ».

## Ce que la concentration ajoute

- Un cours obligatoire : `GMC-3351`.
- Règle 1 : 12 crédits parmi 11 cours (`GEL-4250`, `GIF-1003`, `GIF-4101`, `GLO-4001`, `GMC-1300`…).
- `credits_required` vaut 18.

## Scénario

1. Pénélope passe du cheminement sans concentration à « Robotique ».
2. Elle place `GMC-3351` et quatre cours de la Règle 1.
3. Elle lit le bilan.

## Résultats attendus

- Le panneau passe de 51 cours à 12 : la reconstruction est complète, sans reliquat de l'ancienne concentration.
- La grille n'est pas vidée par le changement.
- Les cours `GEL-`, `GIF-` et `GLO-` gardent la teinte qu'ils avaient sous l'autre concentration : les teintes sont attribuées sur tous les sigles du fichier de programme.
- L'en-tête de section affiche `Robotique : 15 cr. / 18 cr.` avec `GMC-3351` plus quatre cours à trois crédits.

## Repères pour le test e2e

- Après le changement, le nombre de `.course-line` chute et le nombre de `.dropped-tile` ne bouge pas.
- La couleur de fond d'une pastille `GIF-1003` est identique avant et après.

## Variantes et cas limites

- Comme pour le cheminement sans concentration, les 18 crédits déclarés dépassent la somme du cours obligatoire et du maximum de la règle : la section ne peut pas atteindre son total.
- « Robotique » figure en dur dans les options du `index.html` statique : ce vestige doit être remplacé au premier chargement et ne jamais apparaître pour un autre programme (US-22).
