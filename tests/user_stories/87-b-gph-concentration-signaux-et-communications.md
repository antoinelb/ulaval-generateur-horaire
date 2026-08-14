# US-87 — B-GPH, concentration « Signaux et communications »

**Persona** : Bastien, en génie physique, orienté vers le traitement du signal.
**Intention** : combler sa concentration.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 7 cours (`GEL-2001`, `GEL-3003`, `GEL-3006`, `GEL-4200`, `GEL-4201`, `GEL-4202`, `GPH-4104`).
- `credits_required` vaut 15.
- C'est la dernière des sept concentrations dans le menu.

## Scénario

1. Bastien choisit la dernière concentration du menu.
2. Il place cinq cours.
3. Il lit le bilan.

## Résultats attendus

- Le menu conserve l'ordre du fichier de programme : les sept concentrations d'abord, puis « Profil distinction » et « Profil international ».
- L'en-tête de section atteint `15 cr. / 15 cr.`
- Six cours `GEL-` et un `GPH-` : deux teintes seulement.

## Repères pour le test e2e

- `#cheminement-select option` a `Signaux et communications` en septième position sur neuf.
- La carte de la Règle 1 contient 7 `.course-line`.

## Variantes et cas limites

- Sélectionner la dernière concentration puis changer de millésime fait retomber la sélection sur la première : le comportement de US-79 s'applique à tous les programmes.
- `GEL-2001` et `GEL-4201` sont partagés avec « Photonique » (US-85); `GPH-4104` l'est avec « Photonique » et « Génie médical et biophotonique ».
