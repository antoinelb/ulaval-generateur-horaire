# Hauteur minimale du ruban pour cinq cours

Date : 2026-08-27

> **Remplacé le 2026-08-30** par `2026-08-carte-de-session-tronquee-en-lignes-entieres` : la promesse « sigles toujours tous visibles » ne tenait pas — le plancher était calculé pour des sigles à 0.6875rem alors que le CSS les rend à 0.875rem, et `.ribbon` (en `overflow-x: auto`, donc de taille minimale automatique nulle) était comprimé par la coquille en `height: 100vh`. La carte a désormais une hauteur fixe et compte ce qu'elle ne montre pas.

## Contexte

L'audit LAY-2 (V1, la plus grave) a constaté qu'une carte de session affichait « à planifier » (une ligne) quand `card.codes` était vide, puis 5-6 lignes de sigles dès que le placement automatique arrivait.
Le déclencheur est asynchrone et prouvé : `card.codes` dérive de `plan.displayed_placement`, écrit par la réponse du worker (`mod.rs:429`) qu'`auto_propose` déclenche seul, 500 ms après la dernière saisie — sans aucun geste de l'utilisateur.
`.ribbon-card` ne portait qu'un `min-height: 3rem` : un plancher trop bas, franchi dès le premier placement.
La coquille de l'écran est `height: 100vh; overflow: hidden` en colonne flex (`main.css`) : les rem gagnés par le ruban sont pris à `.main-split`, donc statut, panneau et grille descendaient et rétrécissaient d'un coup.
Un premier correctif avait imposé une hauteur fixe (`height: 7rem`) avec défilement interne des sigles ; Antoine l'a refusé — cacher des sigles derrière un défilement dans une carte aussi petite coûte plus que l'invariance ne rapporte.

## Décision

Exception LAY-2 assumée, décidée par Antoine : `.ribbon-card` porte un `min-height: 7.75rem` — un plancher dimensionné pour l'en-tête plus **cinq** sigles, la portion typique d'une session GEX (4 à 6 cours) — et grandit au-delà si la session en compte davantage.
Le calcul du plancher : bordure 0.25 + padding 0.75 + en-tête 1.03 + retrait 0.25 + 5 × 1.03 (sigles à 0.6875rem, interligne 1.5) + 4 × 0.0625 (espacements) ≈ 7.69rem, arrondi à 7.75rem.
Conséquence : dans le cas typique (≤ 5 cours), le placement automatique ne change pas la hauteur du ruban — l'invariance LAY-2 tient de fait ; au-delà de cinq, la carte grandit et la page se redispose, ce qui est accepté en échange de sigles toujours tous visibles et saisissables (ils se glissent vers la grille).
La carte reste une colonne flex : `.ribbon-card-head` et `.ribbon-card-special` gardent leur taille naturelle (`flex: none`), `.ribbon-card-codes` (ou `.ribbon-card-empty`) occupe le reste du plancher et porte la croissance.

## Alternatives rejetées

- **Hauteur fixe avec défilement interne des sigles** (le premier correctif) : garantit l'invariance totale, mais cache des cours derrière un défilement dans une carte de quelques rem — le coût d'ergonomie dépasse le gain, surtout que les sigles sont la source du glisser-déposer.
- **Garder `min-height: 3rem`** : le plancher est franchi dès le premier placement automatique, la violation reste entière dans le cas *typique* — le plancher à cinq cours la réduit au cas rare.
- **Plancher au pire cas (8 cours et plus)** : gaspille la hauteur de l'écran en permanence pour un cas rare ; la grille en dessous est la zone la plus précieuse.
