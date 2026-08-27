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
