# US-65 — B-GCI, concentration « Géotechnique »

**Persona** : Dominic, en génie civil, attiré par la mécanique des sols et les fondations.
**Intention** : voir les cours de sa concentration et ce qui la distingue des autres.

## Préconditions

Le millésime A26 du B-GCI est présent dans les instantanés livrés.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 12 crédits parmi 10 cours (`FOR-2020`, `GCI-2101`, `GCI-4004`, `GCI-4007`, `GCI-4101`, `GCI-4201`, `GEX-3001`, `GGL-2600`, `GMN-2000`, `GMN-2001`).
- Règle 2 : 3 crédits par référence croisée vers la Règle 1 du cheminement sans concentration.
- `credits_required` vaut 15.

## Scénario

1. Dominic compare les trois concentrations spécialisées du B-GCI.
2. Il choisit « Géotechnique » et place `GCI-2101`, `GCI-4101` et `GMN-2000`.

## Résultats attendus

- Les trois concentrations spécialisées ont la même forme : dix à douze cours en Règle 1 pour 12 crédits, plus 3 crédits par référence.
- Quatre cours sont communs aux trois : `FOR-2020`, `GCI-4004`, `GCI-4201` et `GEX-3001` selon la concentration; changer de concentration ne les repeint pas.
- Les cours `GMN-` (génie des mines) n'apparaissent que dans cette concentration et dans le cheminement sans concentration.
- Les trois cours placés (`GCI-2101`, `GCI-4101`, `GMN-2000`) réapparaissent dans la Règle 2, sélectionnés et non sélectionnables, sous-titrés « compté dans la Règle 1 ».
- La Règle 2 affiche `0/3 cr` et l'en-tête du groupe affiche `Concentration — Géotechnique` et `9/15 cr`.

## Repères pour le test e2e

Les sélecteurs `.course-line` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Les textes cités entre guillemets — « Concentration — … », « compté dans la Règle 1 » — sont ceux de l'UI Rust (`crates/ui/src/components/panel.rs`) ; l'application JS peut ne pas encore les porter mot pour mot.

- La carte de la Règle 1 contient 10 `.course-line`, dont `GMN-2000` et `GMN-2001`.
- La Règle 2 contient les 25 cours résolus de la Règle 1 du cheminement sans concentration.
- Les trois `.course-line` des cours placés apparaissent aussi dans la carte de la Règle 2, sans bande de choix, avec le sous-texte « compté dans la Règle 1 ».

## Variantes et cas limites

- La progression déduplique les cours présents dans les deux règles et plafonne à `15/15 cr`.
- `GMN-2901` n'est offert qu'à l'hiver : une concentration en géotechnique se heurte plus souvent que les autres à l'alerte de cours non offert (US-39).
