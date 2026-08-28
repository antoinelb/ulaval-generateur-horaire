# B minimise la distance au seed quand on lui en donne un

**Date :** 2026-08-28
**Statut :** accepté.
Supersède en partie `2026-08-organigramme-en-continu-sans-bouton`, dont la décision « le moins de changements possible **est** le seed » n'est plus suffisante.
Révise la sortie de `2026-07-b-placement-par-satisfaction-fait-main` (satisfaction pure) pour le seul cas seedé.

## Contexte

Le rapport UX du 2026-08-27 (`docs/ux/rapport-directeur-gci-2026-08-27.md`) démontre le contre-exemple : déplacer un cours vers une session déjà pleine fait redistribuer la moitié du cheminement, alors que le bandeau promet que le placement proposé « suit votre cheminement actuel du plus près ».

Le seed n'était qu'un **ordre de valeurs** : chaque domaine trié par distance à la session seedée, première solution rendue.
Sans collision, la première descente reseat tout le monde à sa place et le résultat est effectivement minimal.
Avec une collision de plafond, la descente échoue sur le cours évincé, remonte, et **reconstruit tout ce qui suit** — un ordre de valeurs n'a aucune mémoire de ce qu'il a déjà fait bouger.
La proximité n'était donc jamais prouvée, seulement espérée là où rien ne poussait.

## Décision

- **Deux champs sur `PlacementRequest`.**
  `minimize_seed_distance: bool` transforme la recherche en séparation-évaluation (*branch and bound*) sur Σ|session − ancre| ; `balance: bool` est la post-passe d'équilibrage, décrite dans son propre ADR.
  Le contrat JSON `OrganigrammeInput` ne bouge pas : c'est `place_escalating` (crate `wasm`) qui pose les drapeaux, l'appelant JS n'a rien à savoir.
- **Sémantique du drapeau, écrite sur le champ.**
  Il est de fait désactivé sous `allow_unplaced` — un cours laissé de côté ne siège nulle part et n'a pas de distance ; `Completion::Complete` veut alors dire « optimum **prouvé** », pas seulement « pile épuisée » ; `SolutionCap` n'est jamais émis et `max_solutions` est ignoré, une seule solution revenant.
- **Borne admissible, jamais optimiste.**
  `suffix_min_cost[d]` = somme, pour les candidats `d..`, de la plus petite distance que leur domaine permet encore.
  Elle ne surestime jamais, donc élaguer dessus ne perd jamais l'optimum.
  Le test se fait au *dépilage* et pas seulement au moment d'empiler : c'est ce qui purge les frames posées avant une amélioration de l'incumbent.
- **Anytime.**
  Un budget épuisé rend la meilleure feuille atteinte — jamais pire que la première solution qu'elle remplace — et le dit (`NodeBudget`).
  Quand le coût atteint la borne globale `suffix_min_cost[0]`, la recherche s'arrête sur-le-champ : c'est ce qui garde le cas sans collision aussi rapide qu'avant, un seed qui tient coûtant zéro et se prouvant optimal à la première feuille.
- **Escalade.**
  `minimize = !request.seed.is_empty()` : une génération seedée est une *re*-génération, l'étudiant a une grille à l'écran.
  Les quatre passes contraintes (exacte, crédits assouplis, étés ouverts, les deux) le portent ; la passe relâchée finale, non.
- **L'ordre de valeurs de la passe assouplie garde l'ancre.**
  Un cours à seuil de crédits était trié « le plus tard possible » ; avec un seed il est désormais trié `(été démoté, distance à l'ancre, session la plus tardive)` — la passe assouplie ne doit pas défaire la grille qu'on lui a remise sous prétexte qu'un seuil préfère la fin de l'horizon.
- **Coût = somme des distances**, pas nombre de cours déplacés.

Mesuré sur B-GCI A26 (33 cours, 12 sessions, plafond 17) : retarder un cours d'une session de même saison déplace maintenant 1 à 3 cours, optimum prouvé, en quelques millisecondes ; l'ancien ordre de valeurs en déplaçait jusqu'à dix.

## Alternatives rejetées

- **Déduire l'intention de `seed` non vide + `max_solutions == 1`** : la même requête sert aussi à des sondes d'existence, et un comportement implicite qui change la sortie ne se lit dans aucune signature.
- **Chaîner le seed entre les passes** (repartir de la solution de la passe précédente) : sans objet — une passe ne court que si la précédente n'a rendu aucune solution.
- **Coût = nombre de cours déplacés** : deux grilles à un déplacement chacune ne se valent pas si l'une décale d'une session et l'autre de huit ; et l'objectif perd son additivité par candidat, donc sa borne de préfixe, donc l'élagage.
- **Statu quo (ordre de valeurs seul)** : c'est exactement ce que le rapport a mis en défaut.
