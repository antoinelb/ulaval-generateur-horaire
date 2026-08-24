# US-66 — B-GCI, concentration « Structures et matériaux »

**Persona** : Josée, en génie civil, qui vise la conception de bâtiments et d'ouvrages d'art.
**Intention** : combler sa concentration à partir de la liste la plus longue des trois.

## Préconditions

Le millésime A26 du B-GCI est présent dans les instantanés livrés.

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
- Les quatre cours placés réapparaissent dans la Règle 2, sélectionnés et non sélectionnables, sous-titrés « compté dans la Règle 1 ».
- La Règle 2 affiche `0/3 cr` et l'en-tête de section affiche `12/15 cr`.

## Repères pour le test e2e

Les sélecteurs `.course-line` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Le texte cité entre guillemets « compté dans la Règle 1 » est celui de l'UI Rust (`crates/ui/src/components/panel.rs`) ; l'application JS peut ne pas encore le porter mot pour mot.

- La carte de la Règle 1 contient 12 `.course-line`.
- La carte de la Règle 2 contient les 25 cours résolus de la Règle 1 du cheminement sans concentration.
- Les quatre `.course-line` des cours placés apparaissent aussi dans la carte de la Règle 2, sans bande de choix, avec le sous-texte « compté dans la Règle 1 ».
- Deux pastilles de matières différentes n'ont pas la même couleur de fond.

## Variantes et cas limites

- La progression déduplique les cours présents dans les deux règles et plafonne à `15/15 cr`.
- Cette concentration recoupe fortement le « Profil développement durable » (`GBO-2040`, `GCI-4201`) : les portées concentration et profil sont indépendantes, si Josée sélectionne les deux à la fois (l'UI Rust offre un sélecteur de concentration et un sélecteur de profil séparés) un cours placé une seule fois — `GCI-4201` — compte dans la règle de la concentration et dans celle du profil (US-68).
