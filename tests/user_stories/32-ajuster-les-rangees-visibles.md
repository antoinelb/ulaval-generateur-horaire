# US-32 — Ajuster le nombre de rangées visibles

**Persona** : Philippe, qui consulte la grille sur un portable à petit écran.
**Intention** : réduire la hauteur du tableau sans perdre de contenu.

## Préconditions

- Une grille comptant au moins six rangées.

## Scénario

1. Philippe clique « − » sous le tableau jusqu'à ne voir que deux rangées.
2. Il fait défiler le tableau verticalement.
3. Il clique « + » pour tout réafficher.

## Résultats attendus

- Le bouton ne supprime aucune rangée : il fixe la hauteur maximale du conteneur à l'en-tête plus N rangées.
- Le contenu masqué reste accessible par défilement vertical.
- Le nombre de rangées visibles est borné entre 1 et le nombre réel de rangées.
- Quand toutes les rangées tiennent, la hauteur maximale est retirée.

## Repères pour le test e2e

- `.table-wrapper` a un `max-height` calculé après un clic sur `#btn-retirer-rangee`, vide après assez de clics sur `#btn-ajouter-rangee`.
- `tbody tr` garde le même nombre d'éléments avant et après.

## Variantes et cas limites

- Un dépôt qui ajoute une rangée pendant que l'affichage est réduit ne doit pas faire sauter la hauteur.
- Un redimensionnement de la fenêtre recalcule la hauteur et la taille des pastilles.
- Le réglage n'est pas persisté : il repart à cinq rangées à chaque chargement.
