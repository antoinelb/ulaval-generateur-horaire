# US-72 — B-GIN, concentration « Ingénierie en intelligence numérique des systèmes »

**Persona** : Mégane, en génie industriel, qui veut se former à l'analytique et à l'optimisation.
**Intention** : combler sa concentration en partant de son cours obligatoire.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que la concentration ajoute

- Un cours obligatoire : `MQT-3000`.
- Règle 1 : 3 crédits parmi `MQT-2101` et `STT-1100` — un choix entre deux cours seulement.
- Règle 2 : 6 à 9 crédits parmi 8 cours (`GIN-4003`, `GIN-4021`, `GLO-2005`, `GLO-4000`, `GSO-3105`…).
- Règle 3 : 0 à 3 crédits parmi 9 cours (`GIF-1003`, `GMC-1300`, `GMC-2007`…).
- `credits_required` vaut 15.

## Scénario

1. Mégane choisit cette concentration.
2. Elle place `MQT-3000` puis `STT-1100`.
3. Elle place trois cours de la Règle 2.

## Résultats attendus

- La Règle 1 se comble avec un seul cours et affiche alors `3 cr. / 3 cr.`
- Placer les deux cours de la Règle 1 ne fait pas dépasser son maximum : la contribution reste à 3 crédits.
- L'en-tête de section atteint `15 cr. / 15 cr.` avec cinq cours de trois crédits.

## Repères pour le test e2e

- La carte de la Règle 1 contient exactement 2 `.course-line`.
- Après avoir placé `MQT-2101` **et** `STT-1100`, la ligne du journal reste `Règle 1 : 3 cr. / 3 cr.`

## Variantes et cas limites

- `STT-1100` et `STT-1900` sont deux cours de statistique distincts : ne pas les confondre en écrivant le test.
- Une règle à deux cours pour un choix unique est la traduction en crédits d'un « Un cours parmi »; la conversion reste approximative quand les crédits des cours diffèrent (US-37).
- Cette concentration partage sa Règle 3 avec trois autres concentrations du programme, à la liste près.
