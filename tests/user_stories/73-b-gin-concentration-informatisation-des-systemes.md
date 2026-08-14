# US-73 — B-GIN, concentration « Ingénierie de l'informatisation des systèmes d'entreprise »

**Persona** : Olivier, en génie industriel, orienté vers les systèmes d'information.
**Intention** : combler une concentration à quatre règles dont deux facultatives.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que la concentration ajoute

- Un cours obligatoire : `GIF-1003`.
- Règle 1 : 3 crédits parmi `IFT-1003` et `SIO-2103`.
- Règle 2 : 3 à 9 crédits parmi 10 cours (`GIN-4003`, `GIN-4021`, `GLO-2004`, `GLO-2005`, `GLO-2100`…).
- Règle 3 : 0 à 6 crédits parmi 12 cours (`GMC-1300`, `GMC-2007`, `GMC-3016`…).
- Règle 4 : 0 à 3 crédits, un seul cours (`MNG-3103`).
- `credits_required` vaut 15.

## Scénario

1. Olivier choisit cette concentration.
2. Il place `GIF-1003`, `SIO-2103` et trois cours de la Règle 2.
3. Il lit le bilan.

## Résultats attendus

- Les Règles 3 et 4, à minimum 0, ne sont jamais en avertissement.
- Le titre de la concentration contient une apostrophe : elle doit s'afficher correctement dans le menu, dans l'en-tête de section et dans chaque en-tête de carte, sans échappement visible.
- L'en-tête de section plafonne à 15 crédits.

## Repères pour le test e2e

- L'option de `#cheminement-select` porte exactement `Ingénierie de l'informatisation des systèmes d'entreprise`.
- La ligne de section du journal reprend ce titre à l'identique.
- Un sélecteur Playwright par texte doit gérer l'apostrophe typographique du fichier source.

## Variantes et cas limites

- `GIF-1003` est cours obligatoire ici, mais membre d'une règle à option dans trois autres concentrations : le rôle d'un sigle dépend de la spécialisation affichée.
- La Règle 4 est identique à celle de la concentration « chaîne logistique » : un seul cours, borné 0 à 3.
