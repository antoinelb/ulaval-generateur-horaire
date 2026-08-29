# Un jeton « ⇄ N » signale au repos les blocs qui ont des plages alternatives

Date : 2026-08-27

## Contexte

Dans l'horaire hebdomadaire, cliquer un bloc plein révèle en pointillé les autres plages possibles du cours (ses autres options complètes).
Rien au repos n'indiquait quels blocs avaient des alternatives à révéler : un cours sans aucune autre option se cliquait exactement comme un cours qui en offrait cinq.
Le `title` du bloc promettait en plus systématiquement « Voir les autres plages de {code} », même quand `course.alternatives` était vide — un mensonge d'interface (AIR : l'interface ne ment jamais sur ce qu'elle sait).

## Décision

Chaque bloc plein (jamais un fantôme) porte le compte de ses options alternatives, dans `Block.alternatives: usize` (`crates/ui/src/present.rs`), calculé une fois par cours (`course.alternatives.len()`) et copié sur chacun de ses blocs pleins ; les blocs fantômes reçoivent `alternatives: 0` puisqu'un fantôme n'annonce jamais ses propres frères.
`GridBlock` (`crates/ui/src/components/grid.rs`) affiche, quand `alternatives > 0`, un jeton textuel `⇄ {N}` (span `.grid-block-alts`, flottant à droite pour céder la place au titre — le titre a déjà `overflow: hidden`, il se redimensionne autour du flottant plutôt que de le chevaucher, y compris sur les blocs de 50 minutes) avec un `aria-label` accordé au singulier/pluriel.
Le `title` du bouton est corrigé pour dire le vrai : « {N} horaires alternatifs — cliquer pour les voir » (« 1 horaire alternatif — cliquer pour le voir » au singulier), et « {code} — aucun horaire alternatif » quand `N` est 0.
La légende de la grille gagne une clause : « ⇄ N = N horaires alternatifs (cliquer le bloc pour les voir) ».
Forme (jeton textuel) et texte portent le sens, jamais une couleur seule ; le survol n'est jamais le seul vecteur, le jeton et le `title` sont visibles/lisibles sans survol ni interaction.

## Alternatives rejetées

- **Pile de cartes en `box-shadow`** (empiler des ombres pour suggérer plusieurs plages sous le bloc) — maquettée, rejetée : purement visuelle, aucun compte lisible, et invisible en niveaux de gris ou à faible contraste.
- **Ligne récapitulative sous la grille** (une liste « GEX-1000 a 3 alternatives » sous l'horaire) — maquettée, rejetée : sépare l'information du bloc qu'elle concerne, oblige un aller-retour entre la grille et la liste pour chaque cours.
- Antoine a choisi le jeton textuel sur le bloc (« design A ») : l'information reste au même endroit que l'action qui la révèle.

## Conséquence (2026-08-28)

Le flottant droit décrit ci-dessus ne fonctionne pas : `.grid-block-title` a `overflow: hidden` sans `text-overflow`, donc sur deux lignes de titre le dernier élément du flux — le `.grid-block-detail` (sigle + section) — déborde du bouton, que le bloc absolu suivant peint par-dessus (rapport étudiante cégep 2026-08-27, rapport étudiante GEX 2026-08-27).
Le jeton et le titre sont donc regroupés dans une rangée `.grid-block-top` (flex) au lieu d'un flottant : le titre se comprime en premier (`-webkit-line-clamp: 2`), le jeton garde une largeur fixe à sa droite, et `.grid-block-detail` suit toujours en dessous, tronqué proprement (`white-space: nowrap` + ellipse) plutôt que débordant.
`.grid-block:focus-visible { z-index: 1 }` évite qu'un bloc voisin peint par-dessus masque l'anneau de focus clavier.

## Conséquence (2026-08-29)

Le texte du bloc est désormais centré verticalement dans la hauteur de la plage, et le jeton demandé au coin haut-droit du bloc : les deux ne peuvent plus partager une rangée.
`.grid-block` devient une grille à deux colonnes, `minmax(0, 1fr) auto` — le texte (`.grid-block-text`, une colonne flex qui centre titre et sigle) à gauche, le jeton à droite en `align-self: start`.

La colonne `auto` est ce qui remplace la rangée flex : **elle réserve d'elle-même la largeur du jeton**, donc le titre se comprime en premier (`-webkit-line-clamp: 2`) au lieu de passer dessous, exactement la garantie que la conséquence du 2026-08-28 avait établie.
Une réserve écrite à la main (`padding-right` fixe) a été essayée puis abandonnée : dimensionnée pour « ⇄ 4 » elle laissait « ⇄ 12 » chevaucher le titre de 3 px, et de 7 px au zoom texte 200 %, la largeur rendue du jeton ne suivant pas le `rem` linéairement.
Mesuré après coup : jeton à 0 px du coin haut-droit, écart de 4 px avec le texte (8 px à 200 %), aucun chevauchement, de 900 à 1440 px de large et au zoom 200 %.
