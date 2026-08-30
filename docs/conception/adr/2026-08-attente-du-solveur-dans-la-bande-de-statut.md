# L'attente du solveur se dit dans la bande de statut, plus dans le panneau

Date : 2026-08-30

## Contexte

Antoine, en regardant le panneau de gauche : « enlever l'espace blanc en-dessous de "Permettre un préalable en concomitance" ».

Cet espace n'était pas une marge oubliée : c'était `RecalcNotice`, premier enfant de `div.panel-verdicts`, dont `.panel-recalc { min-height: 2.25rem }` réservait deux lignes.
La ligne est vide au repos **par construction** — l'ADR `2026-08-etat-d-attente-du-solveur-visible` la monte toujours, précisément pour que son apparition ne déplace jamais les entêtes « Règle N ▸ » sous le curseur (LAY-1, LAT-7).
Le défaut qu'elle signale est donc aussi le défaut qu'elle causerait si elle apparaissait et disparaissait.

La supprimer sans plus n'était pas possible : l'annonce de l'attente serait retombée à `running.is_some()`, c'est-à-dire muette pendant les 500 ms de temporisation (`crate::solve::RECALC_DEBOUNCE_MS`) — exactement le trou que l'ADR précédent venait de boucher, et par lequel un « 30/120 cr » transitoire passait sans le moindre signe de calcul.

## Décision

**`RecalcNotice` est fusionné dans `SolverStatus`** (`components/header.rs`), qui vit déjà dans la bande de statut, entre « Rétablir » et « Exporter ▾ ».

- `SolverStatus` se conditionne désormais à `crate::solve::awaited_ms(awaited_since, running.started_ms, now_ms)` — la même expression que portait `RecalcNotice` — et non plus à `running` seul. Il couvre donc la temporisation, et ne rend rien quand `awaited_ms` rend `None`.
- Le libellé est décidé par `present::solver_status(kind, awaited_ms) -> (&str, u64)`, pure et testée : « recherche d'un organigramme » / « vérification du cheminement » quand une requête est partie, « recalcul du placement » pendant la temporisation, où aucun `QueryKind` n'existe encore — annoncer laquelle des deux requêtes suivra serait deviner (TRU-1). La même fonction tronque les millisecondes en secondes, propriété que son test tient.
- **« Annuler la recherche » ne se rend que quand `running.is_some()`** : pendant la temporisation il n'y a aucune requête à tuer, et un bouton sans effet serait un mensonge (TRU-1). C'est déjà la règle posée par l'ADR `2026-08-etat-d-attente-du-solveur-visible` — « on n'annule que ce qui est parti » — appliquée cette fois à l'intérieur du composant.
- `QueryKind` descend de `components/mod.rs` vers `crate::solve`, avec le reste du protocole du worker, `components` le réexportant. Sans ce déplacement, une fonction pure aurait dû importer un type du module de vue, ce que l'AP-7 interdit dans ce sens.
- `.panel-recalc`, `RecalcNotice` et `present::recalc_notice` disparaissent.

**La bande de statut absorbe l'annonce sans rouvrir LAY-1.**
`.status-strip` a déjà `min-height: 2.5rem` — elle est présente et pleine hauteur même vide (LAY-2) — et `.status-exports` porte `margin-left: auto`.
Le message pousse donc contre de l'espace libre : les deux boutons à sa gauche sont avant lui dans le flux, les exports restent collés à droite, rien ne bouge quand il paraît ou s'efface.
Aucune hauteur n'est à réserver, parce qu'aucune n'est prise à quoi que ce soit.

L'attente reste dite ailleurs qu'ici : le voile de la grille et les cartes du ruban suivent `awaited_since` (ADR `2026-08-recalcul-visible-sur-la-grille`), le verdict d'état du panneau affiche « ⟳ recalcul en cours… (le verdict précédent ne s'applique plus) » dès que `verification_stale` est levé — au même instant, dans `track_plan_change` — et les totaux de l'entête gardent leur dernière valeur arrêtée, atténuée et dite dans leur `title`.
Ce qui part n'est pas la voix de l'attente : c'est son doublon, celui qui coûtait une bande blanche permanente.

## Alternatives rejetées

- **Retirer seulement `min-height` de `.panel-recalc`** — la bande blanche partait, et le défaut de LAY-1 revenait entier : la ligne apparaît une seconde plus tard et pousse toute la liste des règles de 36 px sous le curseur, ce que l'ADR `2026-08-etat-d-attente-du-solveur-visible` a mesuré au navigateur.
- **Supprimer `RecalcNotice` sans rien mettre à la place** — la temporisation de 500 ms redevenait muette dans le panneau *et* dans la bande, puisque `SolverStatus` ne suivait que `running`. C'est le défaut d'origine, restauré.
- **Réserver la place dans la bande de statut** (largeur fixe pour le message) — inutile : `margin-left: auto` sur les exports fait déjà ce travail, et une largeur fixe aurait figé une phrase dont les trois formulations n'ont pas la même longueur.
- **Garder les deux, en vidant seulement la ligne du panneau au repos** — c'est l'état actuel, et c'est ce qu'Antoine demande de retirer.
- **Afficher « Annuler la recherche » pendant la temporisation, désactivé** — un bouton grisé pendant une demi-seconde puis actif est un scintillement, et il invite à cliquer sur ce qui n'existe pas encore. Le bouton paraît quand la requête part.

## Conséquences sur les ADR existants

- `2026-08-etat-d-attente-du-solveur-visible` : son point 2 (`RecalcNotice`, `.panel-recalc`, hauteur réservée dans le panneau) est remplacé par le présent ADR. Tout le reste — `awaited_since`, `awaited_ms`, `held_while_awaited`, le verdict d'état à hauteur réservée — reste en vigueur.
- `2026-08-plancher-typographique-de-14-px-a-lecran` : la ligne qui abaissait `.panel-recalc` à `2.25rem` décrit une règle qui n'existe plus.
