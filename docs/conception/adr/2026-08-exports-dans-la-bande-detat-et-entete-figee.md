# Les exports rejoignent la bande d'état, l'entête de l'horaire cesse de défiler

Date : 2026-08-29

## Contexte

« Exporter l'organigramme » et « Exporter l'horaire » vivaient dans `.grid-head`, la rangée qui porte aussi le titre de session (« A1 — Automne 2026 ») et ses statuts.
Cette rangée est à l'intérieur de `.grid-panel`, qui défilait en entier : dès qu'on descendait dans la journée, le titre, les statuts, la légende **et** les deux boutons d'export sortaient de l'écran.

Deux commandes d'application vivaient donc dans un conteneur qui défile, alors que les autres — Annuler, Rétablir — sont dans `.status-strip`, une région fixe de la coquille.

## Décision

- Les deux boutons d'export passent dans `StatusStrip` (`crates/ui/src/components/header.rs`), groupés dans `.status-exports` que `margin-left: auto` pousse au bout droit de la bande. Ils y sont figés sans rien ajouter : `shell.rs` rend `StatusStrip` en frère *au-dessus* de `.main-split`, et `.shell` est `height: 100vh; overflow: hidden` en colonne.
- `.grid-panel` cesse d'être le défileur : il devient une colonne flex en `overflow: hidden`. `.grid-head` et `.grid-legend` sont `flex: none`, et un nouveau `.grid-scroll` (`flex: 1; min-height: 0; overflow-y: auto`) porte la grille et les notes.
  Le titre de session, ses statuts et la légende restent donc en vue quelle que soit l'heure regardée ; seule la grille défile.
- Les entêtes de jour (`.grid-day-head`, déjà `position: sticky; top: 0`) collent désormais au haut de `.grid-scroll` plutôt qu'à celui de `.grid-panel` — c'est-à-dire juste sous la légende, ce qu'elles visaient déjà.
- `grid.rs` perd son `use_context::<PrintTarget>` et l'import `print` devenus inutiles ; `header.rs` les acquiert.

## Alternatives rejetées

- **Rendre `.grid-head` `position: sticky` dans le panneau défilant** : la rangée resterait visible, mais la légende continuerait de défiler sous elle et les boutons resteraient dans un conteneur qui défile — le problème d'origine, déplacé.
- **Laisser les exports où ils étaient et ne figer que la rangée** : ne répond pas à la demande (« au même endroit que les boutons annuler et rétablir »), et laisse deux familles de commandes dans deux régions au comportement différent.

## Conséquences

- Mesuré après un défilement de 500 px : `stripTop`, `headTop` et `legendTop` inchangés au pixel près ; seul `.grid-scroll` bouge.
- La réserve de hauteur de `.grid-head` (`min-height: 4.5rem`, LAY-2 : la hauteur de deux lignes est réservée en tout temps pour que l'arrivée asynchrone du statut « hors grille » ne décale jamais la grille) reste juste, mais son seuil change : sans les boutons d'export, la rangée ne retombe à deux lignes que sous ~1100 px de fenêtre, contre ~1280-1440 px avant. Mesuré : 31 px au cas courant à toute largeur, 41 px au pire cas (hors grille + conflit + sections forcées) jusqu'à 1280 px, 72 px à 1100 px et moins.
  Elle réserve donc 72 px pour un cas courant de 31 px, dans une région désormais figée en permanence. Resserrer cette réserve par paliers de largeur est possible et mesuré, mais n'a pas été fait ici : ce serait un changement de mise en page qui n'a pas été demandé.
- « Exporter l'horaire » exporte toujours la session couramment affichée. Le bouton n'est plus adjacent au titre de cette session : le lien entre les deux est maintenant porté par la vue seule, pas par la proximité. Aucun changement de comportement, seulement de voisinage.
