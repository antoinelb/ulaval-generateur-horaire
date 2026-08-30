# La carte de session devient un châssis et une face, et son corps vaut sept lignes

Date : 2026-08-30

Précise l'ADR `2026-08-carte-de-session-tronquee-en-lignes-entieres` (le budget de lignes et la hauteur qui en découle).

## Contexte

Deux demandes d'Antoine du 2026-08-30 tombent sur la même carte.

1. « La barre d'organigramme n'est pas assez grande pour contenir tous les cours. Il faudrait qu'elle puisse montrer 7 cours. »
   Le budget de `CARD_BODY_LINES = 5` a été fixé le 2026-08-29 sur le cas d'Élodie (A1 de génie physique, 6 cours) : il comptait « +2 » là où la carte aurait dû montrer une session type ordinaire en entier.
2. « Geler dans les sessions d'organigramme devrait être entre le titre (H2-H27) et le nombre de crédits et ça devrait un checkbox là. »
   Or `.ribbon-card` est **un seul `<button>`** qui porte l'en-tête, les insignes et les sigles. Une case à cocher dans un `<button>` est du HTML invalide : le contenu permis d'un `button` est du contenu de flux *sans contenu interactif*. En pratique, le clic sur la case remonte au bouton parent — qui change la session affichée — et l'ARIA ignore les descendants interactifs d'un `role=button`, si bien que la case n'existe pas pour un lecteur d'écran.

Le plancher typographique est par ailleurs passé de 14 px à 12 px la veille (ADR `2026-08-plancher-typographique-de-14-px-a-lecran`, révision 2026-08-30) : le commentaire qui documentait la hauteur de `.ribbon-card` parlait encore de sigles « à 0.875rem », valeur qu'ils n'ont plus.

## Décision

**La carte cesse d'être un bouton unique : elle devient un châssis `div` portant deux contrôles.**

```
div.ribbon-card          châssis — bordure, hauteur, états, glisser-déposer
  div.ribbon-card-head   libellé, case du gel, crédits
  button.ribbon-card-face  tout le reste : insignes, sigles, annotation
```

- Le châssis garde `ondragover` / `ondragleave` / `ondrop` : l'en-tête reste une zone de dépôt, elle ne devient pas une bande morte au milieu de la cible.
- La face est le bouton qui affiche la session — sans bordure ni fond, elle occupe tout ce qui reste de la carte, donc reste la grande cible que l'ancienne carte offrait (INP-1). C'est elle qui porte `aria_current`, puisque c'est elle qui représente la session affichée.
- L'en-tête, lui, n'est plus cliquable pour afficher la session : 1.125rem sur 10.75rem, contre le fait de rendre valide un contrôle imbriqué dans un contrôle.

**`CARD_BODY_LINES` passe de 5 à 7**, et la hauteur CSS est recalculée sur les valeurs réelles d'aujourd'hui — tout est à `0.75rem` d'interligne 1.5, soit **1.125rem la ligne** :

| poste | calcul | rem |
| --- | --- | --- |
| bordure | 2 × 0.125 (la carte affichée porte la plus épaisse) | 0.25 |
| padding | 2 × 0.375 | 0.75 |
| en-tête | une ligne, la case de 0.875 y tient | 1.125 |
| retrait de `.ribbon-card-codes` | `padding-top` | 0.25 |
| corps | 7 × 1.125 | 7.875 |
| gouttières | 6 × 0.0625 | 0.375 |
| | | **10.625** |

arrondi à `height: 10.75rem`, la marge de 0.125rem couvrant exactement la bordure de 0.1875rem que prend une carte survolée pendant un glissement (`.ribbon-card--landing`). `overflow: hidden` reste le garde-fou : la troncature vient de Rust, jamais d'un débordement.

Une ligne d'annonce coûte exactement une ligne de corps et sa gouttière (1.125 + 0.0625 de `padding-top` = 1.1875), donc le compte de `present::ribbon_body` tient quoi que la carte annonce.

## Alternatives rejetées

- **Garder le bouton unique et poser la case en frère, hors de la carte** : la case perdrait sa place « entre le titre et les crédits », qui est précisément la demande.
- **Un `div role="button"` avec `tabindex` et un gestionnaire clavier maison** : refait à la main ce qu'un `<button>` fait déjà (Entrée, Espace, focus, annonce du rôle), et le clic sur la case remonterait encore.
- **Rendre la case cliquable en arrêtant la propagation dans le bouton** : le HTML resterait invalide et l'ARIA continuerait d'ignorer la case; un correctif qui ne corrige que ce qui se voit.
- **Neuf ou dix lignes de corps** : la rangée mange la grille, qui est la zone utile de l'écran. Sept couvre les sessions types de tous les programmes livrés; au-delà, le « +N » fait son travail.
