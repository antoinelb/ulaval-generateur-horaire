# US-64 — B-GCI, concentration « Eau et environnement »

**Persona** : Ariane, en génie civil, orientée vers l'hydraulique et l'assainissement.
**Intention** : combler les 15 crédits de sa concentration.

## Préconditions

Le millésime A26 du B-GCI est présent dans les instantanés livrés.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 12 crédits parmi 10 cours (`FOR-2020`, `GAE-3006`, `GCI-3101`, `GCI-4004`, `GCI-4201`, `GCI-4301`, `GEX-1000`, `GEX-2001`, `GEX-3001`, `GGL-2600`).
- Règle 2 : 3 crédits **à prendre dans la Règle 1 du cheminement sans concentration** — le fichier l'exprime par une référence croisée, pas par une liste.
- `credits_required` vaut 15.

## Scénario

1. Ariane choisit la concentration « Eau et environnement ».
2. Elle place quatre cours de la Règle 1.
3. Elle cherche les cours de la Règle 2.

## Résultats attendus

- La Règle 1 affiche ses dix cours et se comble normalement à 12 crédits.
- La Règle 2 affiche les 25 cours de la Règle 1 du cheminement sans concentration, dans leur ordre source et sans doublon.
- Les quatre cours placés en Règle 1 réapparaissent dans la Règle 2, sélectionnés et non sélectionnables, sous-titrés « compté dans la Règle 1 » — un cours ne compte jamais deux fois dans la même portée.
- La Règle 2 affiche `0/3 cr`, puisqu'aucun cours qui lui reste propre n'est sélectionné.
- L'en-tête du groupe affiche `Concentration — Eau et environnement` et `12/15 cr`.
- Aucune bannière « sans comptage » n'apparaît.

## Repères pour le test e2e

Les sélecteurs `.course-line` et `.rule-card` sont ceux du DOM de l'application JS soeur (`grille-de-cheminement-interactive`).
Les textes cités entre guillemets — « Concentration — … », « compté dans la Règle 1 », la bannière de dépassement de la section « Variantes et cas limites » — sont ceux de l'UI Rust (`crates/ui/src/components/panel.rs`) ; l'application JS peut ne pas encore les porter mot pour mot.

- La carte de la Règle 1 contient 10 `.course-line`.
- La carte de la Règle 2 contient 25 `.course-line` et conserve le texte source de la référence.
- Les quatre `.course-line` des cours placés apparaissent aussi dans la carte de la Règle 2, sans bande de choix, avec le sous-texte « compté dans la Règle 1 ».
- Le badge de la Règle 2 affiche `0/3 cr`, celui du groupe `12/15 cr`.

## Variantes et cas limites

- La même résolution s'applique aux concentrations « Géotechnique » (US-65) et « Structures et matériaux » (US-66).
- Un cinquième cours placé en Règle 1 fait dépasser son maximum de 12 crédits (15 crédits sélectionnés) : le panneau affiche la bannière « ⚠ Règle 1 de la concentration : les cours sélectionnés y totalisent 15 crédits, au-dessus de son maximum de 12. Retirez-en un (ou déplacez une entente) ; en attendant, les règles s'affichent sans comptage. », à la même taille de police que les autres textes du panneau.
