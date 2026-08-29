# Plancher typographique de 14 px à l'écran

Date : 2026-08-29

## Contexte

L'échelle typographique de `crates/ui/assets/main.css` descendait jusqu'à 0.625rem (10 px) : sur 82 règles `font-size`, 31 étaient à 0.6875rem (11 px), 17 à 0.75rem (12 px) et 25 à 0.8125rem (13 px).
Le texte le plus petit se trouvait précisément là où il porte le plus d'information — cellules de l'organigramme, pastilles de règles, blocs de l'horaire.

Antoine a demandé « la taille de police à 14 px partout », arbitré comme un **plancher** : rien sous 14 px, ce qui est déjà au-dessus garde sa taille (le `h1` de l'entête à 1 rem, le titre de section à 0.9375rem).

## Décision

- Toute règle `font-size` de `main.css` sous 0.875rem passe à **0.875rem**, jamais à `14px` : `html { font-size: 100% }` est le pivot du zoom texte à 200 % (AIR INP-8), et une valeur en pixels ne suivrait pas. 79 des 82 règles sont désormais à 0.875rem ; restent `100%` (racine), `0.9375rem` et `1rem`.
- **Une seule exception** : `.status-undo kbd` reste à `0.7em`. L'ADR `2026-08-raccourci-imprime-sur-le-bouton` exige que le raccourci imprimé soit « nettement plus discret et plus petit que le libellé » qu'il double ; en `em` il reste relatif à son parent et grandit avec lui. C'est le seul texte de l'application sous 14 px, et il double un libellé déjà lisible — il n'est jamais la seule source de l'information.
- **La grille de l'horaire s'agrandit avec le texte.** `.grid-axis` et `.grid-day-col` passent de `40.5rem` à `60rem` (14 heures à ~4.3rem au lieu de ~2.9rem), `.grid-block` resserre son rembourrage vertical à `0.125rem` et `.grid-block-detail` son interligne à `1.2`.
- **Le texte est centré dans la hauteur de la plage** : `.grid-block` est déjà un `flex` en colonne, donc `justify-content: safe center`. Le mot-clé `safe` n'est pas décoratif — la boîte est en `overflow: hidden`, et un `center` nu déborderait des *deux* côtés dès que le contenu est trop haut, rognant le titre par le haut, hors d'atteinte. `safe` retombe sur `flex-start` dans ce cas : le comportement de dégradation reste celui d'avant (on perd le bas, jamais le début du titre).
- **Les feuilles d'impression sont exclues** du plancher (`print.css`, `print-horaire.css`, `print-organigramme.css`).

## Pourquoi la grille devait grandir

Le duo de `.grid-block` (titre 0.75rem/1.2 = 14.4 px + sigle 0.6875rem/1.5 = 16.5 px = 30.9 px) remplissait **exactement** les 31 px utiles d'une plage de 50 minutes (bloc de 39 px, moins 8 px de rembourrage) : l'échelle y était déduite de la géométrie, pas choisie.
À 14 px il en faut 38 px. Mesuré sur le B-GEX/A1 en 1440×900 : « Eaux vives » coupé de 7 px, « Matériaux de construction » de 24 px sur 39 — du texte tranché à mi-hauteur des lettres.

Hauteurs de grille mesurées, blocs coupés : 40.5rem (46 px/h) → 2 ; 54rem (62 px/h) → 1 ; **60rem (69 px/h) → 0**.
L'entête de jour est déjà `position: sticky` « pour rester lisible pendant le défilement de la grille » : la grille était conçue pour défiler, la rallonger ne change pas sa nature, seulement la quantité de défilement.

## Pourquoi l'impression est exclue

Ce n'est pas un choix esthétique : le plancher s'y annule lui-même, mesuré dans le navigateur au moment où `window.print()` est appelé.

- **Organigramme** : `browser::shrink_to_fit` réduit la taille racine de 90 % à 60 % par pas de 5 % (`for step in 2..=8`, `16.0 * (1.0 - 0.05 * f64::from(step))`) jusqu'à ce que la feuille de hauteur fixe `8.5in` cesse de déborder. Aujourd'hui elle tient au **premier** pas (racine 14.4 px). Avec un plancher à 14 px, la boucle épuise toute l'échelle et s'arrête à **9.6 px**, son dernier pas, sans avoir réussi. La base imprimée effective devient 0.875rem × 0.6 = **8.4 px, soit moins que les 9 px d'aujourd'hui** — et la hiérarchie de tailles est aplatie en prime.
- **Horaire** : aucun ajustement automatique ne s'y applique (`components/print/mod.rs` : `if kind == PrintKind::Organigramme`). Les blocs de cours à 0.4rem vivent dans une grille de jour `height: 15rem` où « le cas de 50 minutes a besoin de chaque pixel de sa hauteur » ; les porter à 0.875rem (×2,2) déborderait sans compensation.

Les tailles des feuilles d'impression sont donc une **fonction du papier**, pas une préférence de lecture : elles restent calibrées sur la page.

## Alternatives rejetées

- **Tout aplatir à 14 px** (titres compris) : supprime toute hiérarchie de taille ; le plancher préserve les trois niveaux au-dessus.
- **`html { font-size: 14px }`** : ancrerait l'échelle *sous* 14 px partout (0.8125rem deviendrait 11,4 px) — l'inverse de la demande, et une racine en pixels casse INP-8.
- **Exempter `.grid-block-title` / `.grid-block-detail`** (les laisser à 12/11 px) : mesuré à 0 problème et deux lignes de CSS, mais laisse le texte le plus dense de l'application sous le plancher — exactement ce que la demande visait.
- **Accepter les blocs coupés** : « Matériaux de construction » perdait 24 px sur 39.
- **Passer `-webkit-line-clamp` de 2 à 3 lignes** dans `.grid-block-title` : effacerait la dernière troncature en 1440×900, mais réintroduit un risque de coupure latent dans une plage de 50 minutes dont le titre tiendrait sur trois lignes. On garde 2 : la troncature avec points de suite est le mécanisme de dégradation prévu du composant, et le titre complet reste au `title` du bloc (`components/grid.rs`), qui n'est pas la seule affordance.

## Conséquences

- Vérifié par comparaison A/B (même DOM, feuille de style permutée à chaud) en 1440×900, 1024×768, zoom texte 200 % et 390×844 : aucun nouveau débordement de page, aucune nouvelle troncature — sauf une, ci-dessous.
- En 1440×900, « Génie des eaux : introduction à la profession » demande désormais trois lignes dans sa colonne et se tronque à deux (`-webkit-line-clamp: 2`). C'est le mécanisme existant du composant, qui se déclenchait déjà à 1024 px et au zoom 200 % avant ce changement.
- La semaine complète ne tient plus dans la hauteur visible : la grille défile davantage. C'est le prix accepté pour que les blocs de 50 minutes lisent à 14 px.
- Le débordement horizontal en 390 px de large (466 px avant, 512 px après) est antérieur à ce changement et n'est pas traité ici.
