# US-15 — Étalement sur cinq ans avec emploi à temps partiel

**Persona** : Samuel, au B-GIN, qui travaille vingt heures par semaine et étale son bac sur cinq ans.
**Intention** : construire une grille longue, avec des sessions creuses et une interruption complète.

## Préconditions

- Programme « B-GIN », grille par défaut de onze colonnes de session.

## Scénario

1. Samuel ajoute quatre colonnes avec le bouton « + » au-dessus du tableau.
2. Il laisse la session A28 entièrement vide : il ne s'inscrit pas cette année-là.
3. Il réduit le nombre de rangées visibles à trois avec le bouton « − » sous le tableau, pour tenir dans son écran.
4. Il fait défiler horizontalement avec la barre de défilement synchronisée au-dessus du tableau.

## Résultats attendus

- La grille accepte au moins quinze colonnes de session; la largeur minimale du tableau suit le nombre de colonnes.
- Une session vide ne produit aucune alerte.
- Réduire les rangées visibles n'efface aucune pastille : seule la hauteur visible du conteneur change.
- La barre de défilement du haut et le tableau restent synchronisés dans les deux sens.

## Repères pour le test e2e

- `#btn-ajouter-colonne` cliqué quatre fois porte `thead th` à 16.
- `#btn-retirer-rangee` réduit `max-height` de `.table-wrapper` sans changer le nombre de `tbody tr`.
- Faire défiler `.table-wrapper` change `scrollLeft` de `#scroll-sync-bar`, et réciproquement.

## Variantes et cas limites

- Une interruption d'une année complète (trois colonnes vides) doit se supprimer proprement si Samuel préfère raccourcir la grille : le bouton « − » refuse tant qu'une pastille occupe la dernière colonne.
- Le nombre de rangées visibles ne descend jamais sous 1 et ne dépasse jamais le nombre de rangées réelles.
- Sur un écran étroit, la taille des pastilles et leur police sont recalculées au redimensionnement (US-53).
