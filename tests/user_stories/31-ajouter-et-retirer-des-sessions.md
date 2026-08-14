# US-31 — Ajouter et retirer des sessions

**Persona** : Laurence, qui a besoin de deux sessions de plus que la grille par défaut.
**Intention** : allonger puis raccourcir la grille.

## Préconditions

- La grille par défaut affiche onze colonnes de session après « Cours complétés ».

## Scénario

1. Laurence clique « + » au-dessus du tableau, deux fois.
2. Elle place un cours dans la dernière colonne.
3. Elle clique « − » et constate que rien ne se passe.
4. Elle retire le cours, puis clique « − » deux fois.

## Résultats attendus

- Chaque « + » ajoute la session suivante : H → E même année, E → A même année, A → H année suivante.
- La largeur minimale du tableau suit le nombre de colonnes (140 px par colonne).
- « − » refuse de supprimer une colonne contenant une pastille : aucun cours n'est jamais perdu par ce bouton.
- « − » refuse aussi de descendre sous deux colonnes au total.
- Chaque ajout ou retrait relance la vérification et met à jour la barre de défilement synchronisée.

## Repères pour le test e2e

- Après deux « + », le dernier `thead th` vaut `E30` puis `A30` en partant de `H30`.
- Avec une pastille dans la dernière colonne, `#btn-retirer-colonne` laisse `thead th` inchangé.
- La propriété `min-width` du `table` vaut `nbColonnes × 140px`.

## Variantes et cas limites

- Si le dernier en-tête n'est pas un code de session valide, l'ajout retombe sur `H26`.
- L'année passe de 99 à 00 : la séquence doit rester cohérente sur un horizon très long.
- Les cellules ajoutées sont immédiatement des cibles de dépôt.
