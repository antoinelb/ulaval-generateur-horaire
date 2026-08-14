# US-81 — B-GPH, concentration « Aéronautique et aérospatiale »

**Persona** : Édouard, au baccalauréat en génie physique, qui vise l'aérospatiale.
**Intention** : combler les 15 crédits de sa concentration.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».
- C'est la **première** des sept concentrations : elle est sélectionnée d'office au chargement.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 8 cours (`GMC-2005`, `GMC-2008`, `GMC-4054`, `GMC-4150`, `GMC-4151`, `GMC-4250`, `GML-3020`, `GML-3250`).
- `credits_required` vaut 15.

## Scénario

1. Édouard charge le B-GPH.
2. Il place cinq cours de la Règle 1.
3. Il lit le bilan.

## Résultats attendus

- Les sept concentrations du B-GPH ont toutes la même forme : une seule règle de 15 crédits, sans cours obligatoire, et `credits_required` à 15.
- L'en-tête de section affiche `Aéronautique et aérospatiale : 15 cr. / 15 cr.` avec cinq cours de trois crédits.
- Aucun cheminement sans concentration n'est offert : l'étudiant est toujours dans l'une des sept.

## Repères pour le test e2e

- `#cheminement-select` contient sept concentrations puis deux profils, soit neuf options.
- La carte de la Règle 1 contient 8 `.course-line`.

## Variantes et cas limites

- La concentration ne contient que des cours `GMC-` et `GML-`, empruntés au génie mécanique et au génie des matériaux : leurs préalables peuvent citer des cours absents du B-GPH.
- `b-gph/cours/cours-hors-catalogue.csv` ne déclare que `LAN-GUES` : c'est le programme le plus démuni en pseudo-cours, donc celui où un cours à option non déterminé n'a rien à quoi se rattacher.
