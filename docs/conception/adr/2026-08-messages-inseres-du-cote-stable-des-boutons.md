# Messages insérés du côté stable des boutons

Date : 2026-08-27

## Contexte

Deux groupes d'en-tête portent un message conditionnel à côté de boutons fixes, et dans les deux cas le message apparaissait du mauvais côté.

Dans `StatusStrip` (`crates/ui/src/components/header.rs`), `SolverStatus` rendait en premier dans la rangée flex `.status-strip`, avant les boutons « ↶ Annuler » et « ↷ Rétablir ».
`SolverStatus` rend `rsx! {}` quand rien ne tourne, et un `span.status-running` (« recherche d'un organigramme - N s » + « Annuler la recherche ») pendant une recherche.
Au départ ou à la fin d'une recherche, ce span apparaissait ou disparaissait à gauche des boutons et les déplaçait.

Dans `.grid-head` (`crates/ui/src/components/grid.rs`), `main.css:1474-1479` donne `margin-right: auto` au `h2` : le titre reste seul à gauche, et le reste du groupe (statut, boutons d'export) est tassé contre le bord droit.
Le span conditionnel « ⚠ {off_grid} cours hors grille — détail sous l'horaire » était rendu après les deux boutons d'export, en bout de groupe.
Comme ce groupe est ancré à droite, son apparition ou sa disparition (résultat asynchrone du solveur) décalait tout le groupe — y compris les boutons — vers la gauche ou la droite.

Dans les deux cas : un message conditionnel placé du côté mobile d'un groupe ancré déplace tout ce qui partage l'ancrage — un flash de boutons déplacés, en violation de LAY-2 (rien à l'écran ne bouge sans geste de l'utilisateur).

## Décision

Un message conditionnel s'insère du côté du groupe où il ne déplace rien : à l'opposé de son ancrage.

- `StatusStrip` est ancré à gauche (les boutons sont les premiers éléments fixes) : `SolverStatus {}` se rend après les deux boutons, à droite — rien de fixe n'existe à sa droite pour être déplacé.
- `.grid-head` est ancré à droite (par `margin-right: auto` sur le `h2`) : le span « cours hors grille » se rend avant `.grid-status`, juste après le `h2` — rien de fixe n'existe à sa gauche pour être déplacé, et le message grandit dans l'espace libre du milieu.

Dans les deux cas, la bande ou l'en-tête garde sa hauteur fixe ; seule la position du message conditionnel change selon le côté où le groupe est ancré.

## Alternatives rejetées

- Largeur réservée pour le message, boutons alignés du côté opposé : garde l'ordre visuel actuel mais gaspille l'espace au repos avec un vide qui ne sert jamais.
- Position absolue pour le message : sort du flux normal et risque un chevauchement avec les boutons ou avec d'autres éléments du groupe.

## Conséquence (2026-08-27)

Repositionner le message ne suffit pas seul : `.grid-head` est aussi `flex-wrap: wrap`, et un groupe trop large en largeur étroite retombe sur deux lignes plutôt que de se décaler latéralement — un décalage vertical, pas horizontal, mais toujours interdit par LAY-2.
Le message « ⚠ {off_grid} cours hors grille — détail sous l'horaire » a donc été raccourci à « ⚠ {off_grid} cours hors grille », la phrase « détail sous l'horaire » passant en `title` (complément, jamais seule affordance) puisque le détail est déjà toujours visible sous la grille (`GridFootnotes`) et redit par la légende.
Une estimation par largeur de caractères montre que même raccourci, la combinaison réaliste hors grille + conflit + session forcée peut encore dépasser une ligne vers 1280-1440 px de fenêtre (le panneau fixe de 19,5 rem laisse le reste à la grille, sans plafond de largeur) ; `.grid-head` réserve donc `min-height: 4.5rem` (deux lignes) en tout temps, pour que le passage à une ou deux lignes ne décale jamais la légende ni la grille.
