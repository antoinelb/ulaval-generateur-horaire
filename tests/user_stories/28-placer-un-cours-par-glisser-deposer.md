# US-28 — Placer un cours par glisser-déposer

**Persona** : Maxime, qui construit sa grille cours par cours.
**Intention** : déposer un cours du panneau de droite dans la session de son choix.

## Préconditions

- Un programme chargé, panneau de règles affiché.

## Scénario

1. Maxime saisit la ligne de `GCI-1000` dans le panneau, ou sa petite pastille de couleur.
2. Il la glisse dans la cellule A26 de la première rangée.

## Résultats attendus

- Une pastille apparaît dans la cellule, portant le sigle, la couleur de la matière et les données du cours (`data-code`, `data-title`, `data-credits`).
- La ligne correspondante du panneau prend l'apparence « placée ».
- La vérification complète se relance : préalables, offre, conflits, bilan.
- La cellule survolée est mise en évidence pendant le glissement, et la mise en évidence disparaît au dépôt ou à la sortie.

## Repères pour le test e2e

- Le glisser-déposer HTML5 n'est pas simulable par `dragTo` seul dans tous les navigateurs : passer par `dataTransfer` (`dispatchEvent` de `dragstart`, `dragover`, `drop`) avec la charge utile `application/json` `{code, title, credits}`.
- Après le dépôt, `.dropped-tile[data-code="GCI-1000"]` existe dans la bonne cellule.
- `.rules-panel .course-tile[data-code="GCI-1000"].placed` existe.
- `.drop-target.drag-over` existe pendant le survol et disparaît après.

## Variantes et cas limites

- Une charge utile illisible retombe sur `text/plain` interprété comme un sigle nu; sans sigle, rien ne se passe.
- Déposer un cours déjà présent ailleurs le déplace au lieu de le dupliquer (US-29).
- Déposer dans la colonne « Cours complétés » est permis et n'ajoute aucun menu de section.
- Toutes les cellules du tableau sont des cibles de dépôt, y compris celles créées après coup par l'ajout d'une colonne ou d'une rangée.
