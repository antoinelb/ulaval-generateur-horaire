# Placement au mieux en repli : remplir la grille plutôt que se taire

Date : 2026-08-17

## Contexte

L'organigramme était tout ou rien.
`place` court-circuitait à zéro solution dès qu'un seul cours était écarté par le pré-écran, une feuille ne comptait que si *tous* les candidats étaient affectés, et l'UI renonçait en silence (`let Some(solution) = … else { return; }`) sans toucher la grille.

Le troisième essai utilisateur (`docs/ux/rapport-etudiante-2026-08-14.md`) l'a payé comptant : scolarité préparatoire décochée, « l'organigramme ne bouge pas d'un cours […] puis rien ne change et **aucun message ne me dit si la recherche a abouti ou échoué** ».
La testeuse le classe parmi les trois raisons qui l'empêchent de se fier à l'outil, et note que c'est « la situation de plusieurs de mes camarades admis avec des cours d'appoint ».

## Décisions

- **Des trous, jamais une faute.** Quand aucun agencement complet n'existe, la grille est remplie au maximum et **chaque case placée respecte toutes les contraintes** (préalables, plafond, saison d'offre, étés fermés, faisabilité hebdomadaire). Ce qui ne rentre nulle part reste hors grille et est nommé. Aucune case ne ment — un cours posé « en faute » aurait vidé de son sens le ✓ « vérifié » que l'ADR `2026-08-verdicts-honnetes-et-panneau-jamais-vide` venait de rendre honnête.
- **La sentinelle `0`.** Les sessions étant en base 1 partout, `0` est libre comme valeur de domaine « pas placé ». Sous `PlacementRequest.allow_unplaced`, chaque candidat la gagne **en dernier**. Deux conséquences gratuites : un candidat au domaine vide n'a plus qu'elle comme enfant, et un candidat à l'arbre insatisfiable voit ses enfants réels élagués et la garde seule. Le pré-écran `blocked_candidates` cesse d'être un court-circuit et redevient purement informatif.
- **Un cours laissé de côté contraint dans un seul sens.** Ses dépendants doivent le voir absent (`course_leaf` rend `Verdict::False` sur la sentinelle — sans quoi le saut se propagerait en mensonge, et le saut *cascade* honnêtement sur ce qui l'exigeait). Mais **son propre arbre de préalables cesse de s'appliquer** : rien n'est requis pour *ne pas* suivre un cours. Évaluer les deux sens tuait toutes les branches d'un candidat déjà connu insatisfiable, et la recherche relâchée retombait silencieusement à « rien trouvé ».
- **Repli automatique, à un seul endroit.** `wasm::organigramme::place_filling` tente l'agencement exact — inchangé, il répond dès qu'il le peut — et n'enchaîne sur la passe au mieux que si elle ne rend rien. `verify` et `admissible_sessions` ne la traversent jamais : prouver reste prouver, et une sonde relâchée déclarerait toute session admissible en sortant tout le reste. La seconde passe est quasi gratuite : la sentinelle étant disponible à chaque profondeur, la première feuille arrive en ~un développement par cours.
- **`Solution.left_out`**, et non « unplaced » : `ui::solve::unplaced_codes` désigne déjà autre chose (les cours du plan sans session, quel qu'en soit l'auteur).
- **Le verdict ne contredit pas la grille.** `completion_note` se tait sur une réponse au mieux — son `completion` décrit l'énumération *relâchée*, donc « d'autres agencements équivalents existent » comme « rien trouvé » mentiraient. `left_out_note` prend le relais et nomme chaque cours avec sa raison (celle de `blocked` quand le pré-écran l'a désigné, « aucune place ne restait » sinon), et la ligne « N cours sans session — **proposez un organigramme** » est reformulée : c'est justement ce que l'étudiant vient de faire.

## Plafond assumé

Le DFS étant à ordre de candidats fixe (lui-même semé par le cheminement type) et la sentinelle explorée en dernier, la première feuille est un remplissage **glouton** — maximal, pas de cardinalité maximum prouvée.
C'est suffisant pour « corriger les quelques détails », et la passe tourne donc avec `max_solutions: 1` : les feuilles suivantes sont strictement pires.
Sortie si un cas réel exige la différence : séparation-évaluation sur le nombre de sauts.

## Conséquence sur la référence

`tests/reference/solveur_b/place.py` **n'est pas étendue** et saute les fixtures `allow_unplaced` : elle plie la frontière complète, une sentinelle par cours y multiplierait l'espace par (n+1).
Les quatre fixtures relâchées sont écrites à la main et gardées par le test d'intégration Rust seul ; l'oracle continue d'arbitrer bit pour bit la famille exacte, ce pour quoi il a été commité (ADR `2026-07-reference-b-versionnee-jusqua-larbitrage`).

## Alternatives rejetées

- **Placer chaque cours quand même, en faute marquée ⚠** (plafond dépassé, conflit, préalable manquant). La grille mentirait case par case, et le ✓ « Placement vérifié » perdrait tout sens ; le style ⚠ existe pour ce que l'étudiant fait *sciemment*, pas pour ce que l'outil lui impose.
- **Relâchement toujours actif, une seule passe.** Plus simple, mais désactive l'élagage par capacité (`suffix_credits`) et change le comportement du cas qui marche aujourd'hui — un prix inacceptable pour un cas de repli.
- **Un second bouton « Remplir au mieux ».** Charge l'étudiant d'un choix qu'il ne peut pas faire éclairé : il ne sait pas avant de cliquer si l'agencement exact existe.
- **Une boucle gloutonne d'appels à `place` sur une liste qui rétrécit.** n recherches complètes, dont la mauvaise à 1 M de nœuds — onglet gelé.
