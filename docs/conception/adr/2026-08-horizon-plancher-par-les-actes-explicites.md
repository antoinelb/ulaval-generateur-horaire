# L'horizon est planchéié par les actes explicites, les sièges automatiques s'évincent

## Contexte

Réduire le bouton « Sessions » ne faisait que poser `study_sessions`.
Un cours laissé en « automatique » gardait son siège au-delà du nouvel horizon dans `displayed_placement`, la grille l'affichait « placé en ? », et la vérification — qui épingle tout ce qui est affiché — refusait le plan entier : « GEX-2001 is pinned to session 11, outside 1..=9 » (2026-08-26).

## Décision

`state::set_horizon` devient le seul point de changement de `study_sessions` :

- Le plancher `state::horizon_floor` tient les actes explicites : la plus haute session épinglée, la plus haute session manuelle non vide, et les sessions complétées du relevé plus une encore ouverte — jamais moins que 2, plafonné à `MAX_STUDY_SESSIONS` pour qu'un enregistrement corrompu ne fasse pas paniquer `clamp`.
- La réduction s'arrête au plancher ; l'attribut `min` du bouton l'affiche.
- Les sièges *automatiques* au-delà du nouvel horizon sont évincés de `displayed_placement` : le cours retombe en « automatique » et la prochaine proposition le replace dans l'horizon.
  Les épinglés sont sous le plancher par construction, donc `displayed ⊇ pinned` survit.
- `persist::restore_plan` ré-affirme l'horizon sauvegardé au chargement : les sauvegardes d'avant la règle guérissent d'elles-mêmes.

## Révision 2026-08-26 — le plancher s'annonce

Un bouton qui borne en silence a été lu comme « un bogue qui m'empêche de réduire le nombre de sessions ».
Quand la valeur demandée est bornée, une alerte nomme le fait qui fixe le plancher — les sigles épinglés avec leur session, les cours manuels, ou les sessions du relevé — et le geste qui le libère (`state::horizon_floor_note`) ; le champ est remonté pour réafficher l'horizon réellement posé au lieu du nombre refusé.

## Révision 2026-08-26 — le plancher compte des sessions d'étude, les sièges comptent des cases

Le plancher lisait un indice de case comme un nombre de sessions d'étude.
Les deux unités diffèrent : `core::horizon_sessions` insère un été après chaque hiver, donc 5 sessions d'étude ouvrent 7 cases, 7 en ouvrent 10.
Un organigramme du B-GEX dont les derniers épinglages siègent en case 7 se plancheait donc à 7 sessions d'étude — 10 cases — alors que 5 les tiennent (antoine, 2026-08-26).
La même confusion évinçait les sièges automatiques au-delà de `study_sessions` plutôt qu'au-delà du nombre de cases : un cours placé en case 8 d'un horizon de 7 sessions disparaissait de la grille au rechargement.

`state` nomme désormais les deux unités et convertit en un seul endroit :

- `binding_slot` — la case qui contraint : plus haut épinglage, plus haute session manuelle non vide, sessions complétées plus une.
- `study_sessions_for_slot` — la plus petite alternation qui atteint cette case, par la marche bornée de `core::transcript::grow_horizon` ; `MAX_STUDY_SESSIONS` en repli, pour qu'un enregistrement corrompu ne fasse toujours pas paniquer `clamp`.
- `set_horizon` évince sur le nombre de cases, celui-là même que le solveur numérote.
- `horizon_floor_note` étiquette la case qui contraint et annonce le plancher en sessions d'étude — chaque nombre dans son unité.

## Alternatives rejetées

- **Évincer aussi les épinglés hors horizon** : un épinglage est un acte explicite de l'étudiant (pins souverains) ; le détruire par un réglage voisin serait une perte silencieuse.
- **Refuser la réduction tant que des cours siègent au-delà** : bloque le bouton sans dire quoi faire ; le plancher plus l'éviction rendent le même service sans impasse.
- **Filtrer les sièges hors horizon dans la requête seulement** : la grille continuerait d'afficher « placé en ? » — l'interface mentirait sur ce que le solveur voit.
