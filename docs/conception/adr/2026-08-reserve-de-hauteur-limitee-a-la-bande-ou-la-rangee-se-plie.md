# La réserve de hauteur de l'entête d'horaire se limite à la bande où la rangée peut se plier

Date : 2026-08-29

## Contexte

`.grid-head` réservait `min-height: 4.5rem` (72 px) en tout temps, au nom de LAY-2 : le statut « ⚠ N cours hors grille » arrive de façon asynchrone (le solveur, 500 ms après la saisie), et s'il faisait passer la rangée à deux lignes, la légende et la grille descendraient sous les yeux de l'étudiante.

Le cas courant de cette rangée fait 31 px. La réserve laissait donc **50 px de vide entre l'entête et la légende, à toutes les largeurs** — un vide devenu bien visible depuis que l'entête est figée (ADR `2026-08-exports-dans-la-bande-detat-et-entete-figee`) : ce n'est plus de l'espace qu'on fait défiler, c'est de l'espace perdu en permanence.

## Ce que la mesure a montré

Hauteur naturelle de `.grid-head`, `min-height` neutralisé, de 980 à 1920 px, dans les quatre combinaisons de contenu :

| largeur | base | + hors grille | + forcées | les deux | décalage **asynchrone** |
|---|---|---|---|---|---|
| 1920 → 1200 | 31 | 31 | 41 | 41 | **0 px** |
| 1150, 1100 | 31 | 31 | 41 | 72 | **31 px** |
| 1024, 980 | 31 | 31 | 72 | 72 | **0 px** |

Deux choses que la réserve d'origine supposait sont fausses :

- **Le statut asynchrone ne change jamais la hauteur à lui seul.** 31 px avec comme sans lui, à toutes les largeurs : il tient toujours sur la rangée existante.
- **« Libérer les sections forcées » n'est pas asynchrone.** Il ne paraît qu'après un geste de l'utilisateur (forcer une section), et LAY-2 ne protège que des décalages que l'utilisateur n'a pas provoqués.

Le seul décalage réellement asynchrone du domaine tient dans une bande étroite autour de 1100-1150 px, et seulement si le bouton « forcées » est déjà là.

## Décision

`min-height` sort de la règle de base et passe sous `@media (max-width: 74.9375rem)` (1199 px), la bande où la rangée peut encore se plier à l'arrivée du statut asynchrone.
Au-dessus, aucune réserve : la rangée ne peut pas se plier, il n'y a rien à réserver.

Le seuil est en `rem` : une requête média en `rem` se mesure à la taille de police du navigateur, donc le seuil suit un réglage de police plus grande au lieu de le trahir (INP-8).

## Alternatives rejetées

- **`flex-wrap: nowrap` sur la rangée**, pour qu'elle ne puisse jamais se plier : mesuré, et pire. À 1100 px le statut « ⚠ 2 cours hors grille » se tronque (137 px rendus sur 161 requis) — un avertissement qu'on ampute — et la rangée *grandit* quand même (59 px, puis 82 px à 980 px), le bouton « forcées » repliant son propre libellé.
- **Retirer la réserve partout** : ramène le décalage asynchrone de 31 px dans la bande 1100-1150 px, une régression LAY-2 franche pour 50 px gagnés à des largeurs qui n'en avaient pas besoin de toute façon.
- **Réserver la hauteur exacte du pire cas par palier** (41 px au-dessus de 1200, 72 px en dessous) : la valeur haute reste nécessaire dans la bande, et la valeur 41 px ne protège de rien puisque le passage de 31 à 41 px est provoqué par l'utilisateur. Réserver pour un geste de l'utilisateur n'est pas ce que LAY-2 demande.

## Conséquences

- Vérifié aux onze largeurs de 980 à 1920 px, dans les deux états du bouton « forcées » : le haut de la légende est **identique au pixel** avant et après l'arrivée du statut asynchrone. Aucun décalage.
- Le vide au-dessus de la légende passe de 50 px à **8 px** au-dessus de 1200 px. Sous 1200 px il reste 50 px : la réserve y est nécessaire, et c'est le prix de LAY-2 dans cette bande.
- La table ci-dessus dépend de la longueur des libellés (nom de session, statuts). Si un libellé s'allonge notablement, la bande à réserver se déplace : c'est la mesure, pas la valeur, qu'il faut refaire.
