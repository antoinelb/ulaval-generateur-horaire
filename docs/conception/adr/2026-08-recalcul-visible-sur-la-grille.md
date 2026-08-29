# L'état « en recalcul » se voit sur la grille, jamais seulement dans le panneau

Date : 2026-08-29

## Contexte

Le rapport de contre-vérification du directeur du B-GCI (`docs/ux/rapport-directeur-gci-2026-08-29.md`, scénarios « départ hiver » et « double échec ») a montré deux fois, indépendamment, le même piège.
Pendant qu'une recherche tourne (`SolverState.running`, statut « recherche d'un organigramme - N s » dans le panneau de gauche), la grille peut afficher un état transitoire tout à fait plausible — un décalage de session, ou une violation apparente de préalable et de plafond — pendant que le panneau affiche encore « Placement vérifié ✓ » calculé sur l'ancienne solution.
Le seul indice de cet état provisoire vivait dans une ligne de texte sous le panneau, à l'opposé de la grille sur grand écran, facile à manquer.
Un directeur pressé qui capture l'écran juste après avoir changé le Début pourrait publier un cheminement obsolète sans le savoir.

## Décision

Le fait « une recherche tourne » (`solver.read().running.is_some()`) devient visible à trois endroits, tous dérivés de fonctions pures et testées de `present.rs`.

- **La grille** (`WeeklyGrid`) : le statut déjà affiché dans l'entête (`.grid-status`, ligne réservée qui tronque plutôt que de passer à la ligne — LAY-2) commence par « ⟳ recalcul en cours… — » tant que la recherche court (`present::grid_status_label`).
  Aucun nouvel élément, donc aucune hauteur nouvelle à réserver.
  Le corps de la grille (`div.grid`) porte en plus la classe `grid--searching`, un voile discret (opacité réduite) qui ne change ni la mise en page ni `pointer-events` — les blocs restent cliquables.
- **Le ruban** (`SessionRibbon`) porte la même classe (`ribbon--searching`), pour la même raison : les cartes de session peuvent elles aussi montrer un placement transitoire.
- **Le verdict du panneau** (`OrganigrammeControls`) : le texte et la classe CSS du verdict « Placement vérifié ✓ » viennent désormais de `present::verification_verdict(searching)`, qui renvoie soit le ✓ habituel, soit « ⟳ recalcul en cours… (le verdict précédent ne s'applique plus) » avec la classe neutre `panel-verdict--pending` — jamais les deux à la fois, et jamais un ✓ tant que `running` est `Some`.

Les trois décisions (quel texte, quelle classe) sont des fonctions pures de `bool`, testées nativement dans `present.rs`.
Les composants ne font que les appeler et poser la classe (câblage, AP-5).

## Alternatives rejetées

- **Un bandeau neuf dans l'entête de grille**, comme le suggérait littéralement le rapport (« petit bandeau ») — l'entête réserve déjà une hauteur calculée pour une combinaison connue de statuts (titre, hors-grille, conflit, boutons d'export, sections forcées) ; un élément de plus risquait de la faire passer à trois lignes sur les largeurs où plusieurs statuts coïncident déjà, un saut de hauteur (LAY-2).
  Réutiliser la ligne de statut existante (qui tronque, ne s'enroule pas) obtient la même visibilité sans nouvelle hauteur.
- **Bloquer les interactions de la grille pendant la recherche** (`pointer-events: none`) — la recherche automatique se redéclenche à chaque frappe (`auto_propose`, 500 ms) ; bloquer la grille pendant ce délai aurait rendu l'interface saccadée pour un geste qui n'a rien de risqué à laisser continuer.
- **Ne marquer que le panneau plus fort** (gras, couleur) sans toucher à la grille — n'aurait pas répondu au constat précis du rapport : l'œil du directeur était sur la grille, pas sur le panneau, au moment du piège.
