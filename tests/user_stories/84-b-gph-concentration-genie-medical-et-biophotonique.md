# US-84 — B-GPH, concentration « Génie médical et biophotonique »

**Persona** : Ludivine, en génie physique, qui vise l'instrumentation biomédicale.
**Intention** : combler la concentration à la liste la plus longue du programme.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 9 cours (`BPH-2001`, `GEL-4072`, `GML-4103`, `GML-4151`, `GPH-3003`, `GPH-4101`, `GPH-4102`, `GPH-4104`, `PHY-4000`).
- `credits_required` vaut 15.

## Scénario

1. Ludivine choisit cette concentration.
2. Elle place cinq cours en mêlant les matières `BPH`, `GEL`, `GML`, `GPH` et `PHY`.
3. Elle ouvre la grille horaire de la session concernée.

## Résultats attendus

- Cinq matières distinctes donnent cinq teintes distinctes dans la même carte de règle.
- L'en-tête de section atteint `15 cr. / 15 cr.`
- La légende de la fenêtre de grille horaire reprend les couleurs des pastilles.

## Repères pour le test e2e

- La carte de la Règle 1 contient 9 `.course-line`.
- Cinq couleurs de fond distinctes parmi les pastilles placées.
- `#legend .legend-item` compte autant d'entrées que de cours à horaire publié.

## Variantes et cas limites

- `GPH-4101`, `GPH-4102` et `GPH-4104` appartiennent aussi à la concentration « Photonique » (US-85) : trois des neuf cours sont partagés, ce qui rend le choix entre les deux concentrations moins tranché qu'il n'y paraît.
- `BPH-2001` est le seul cours de sa matière dans tout le programme : sa teinte est isolée dans le cercle des couleurs.
