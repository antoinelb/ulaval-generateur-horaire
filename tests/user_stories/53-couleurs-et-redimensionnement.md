# US-53 — Couleurs des pastilles et redimensionnement

**Persona** : Léa, qui consulte sa grille tantôt sur un grand écran, tantôt sur un portable.
**Intention** : que la grille reste lisible dans les deux cas.

## Préconditions

- Un programme chargé, plusieurs matières représentées (`GCI`, `MAT`, `CHM`, `GEX`…).

## Scénario

1. Léa observe les couleurs des pastilles.
2. Elle change de spécialisation.
3. Elle réduit la largeur de la fenêtre à 900 px, puis à 1600 px.

## Résultats attendus

- Toutes les pastilles d'une même matière ont exactement la même couleur; deux matières voisines dans l'alphabet ont des teintes distinctes.
- La clarté et le chroma sont fixes : seule la teinte varie, donc toutes les pastilles ont le même poids visuel et le texte foncé reste lisible.
- Changer de spécialisation ne repeint pas la grille : les teintes sont attribuées sur tous les sigles du fichier de programme.
- Un cours ajouté au programme reçoit sa couleur immédiatement, sans intervention manuelle.
- Au redimensionnement, la taille des pastilles et leur police sont recalculées à partir de la taille réelle d'une cellule.

## Repères pour le test e2e

- La couleur de fond suit le motif `oklch(82% 0.11 <teinte>)`.
- Les variables CSS `--cell-width`, `--cell-height` et `--tile-font-size` changent après `setViewportSize`.
- La police des pastilles ne descend jamais sous 6 px.

## Variantes et cas limites

- Un cours hors catalogue n'a pas de couleur calculée : sa pastille reste au gris par défaut.
- La fenêtre de grille horaire utilise les mêmes couleurs, avec repli sur une palette indexée par hachage du sigle.
- Le résumé « Cours du programme » est construit à chaque chargement mais sa boîte est masquée en dur dans le HTML : il n'est jamais visible. À supprimer ou à rendre accessible.
- Aucune vérification d'accessibilité n'est faite sur le contraste; les pastilles ne sont pas atteignables au clavier.
