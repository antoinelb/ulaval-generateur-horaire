# Une proposition qui désassoit un cours affiché est refusée

Date : 2026-08-27

## Contexte

Rapport étudiante 2026-08-27 (S3) : le compteur du bandeau passait de 105/120 à 99/120 sans qu'aucun geste ne retire de cours, et seul un rechargement le réparait.

`apply_proposal` remplace `displayed_placement` **en entier** par le placement de la réponse.
Un cours que la réponse porte dans `left_out` en tombe donc, ses crédits sortent de `panel::selection` et le compteur les perd.
Rien ne le rattrape : l'écriture est une correction dérivée, hors historique, et l'empreinte `proposed` étant posée à l'envoi, la même requête n'est jamais renvoyée — l'état reste figé jusqu'au prochain rechargement.

## Décision

Une réponse qui retirerait de la grille un cours qu'elle affiche déjà n'est **jamais** adoptée.

- `solve::adoption_regressions(displayed, left_out)` nomme l'intersection, triée.
- Si elle n'est pas vide, `apply_proposal` pousse `solve::proposal_kept_note(codes)` (sujet `ProposalKept`, cause `Document`) et **retourne avant** `overlay_pins` et toute écriture du placement ou des injectés.
- `SolverState.left_out` ne reçoit que les sigles qui flottent réellement, et les toasts `LeftOut` ne parlent que de ceux-là : un cours qui garde sa place n'est laissé de côté de rien de visible. Le verdict « grille vide » est tu dans ce cas, pour la même raison.
- La convergence tient : l'empreinte a été posée à l'envoi, donc la même requête ne repart pas.

C'est une défense **derrière** la minimalité du solveur au seed — pas un substitut : le cœur reste responsable de proposer l'agencement le plus proche de la grille.
Dans la même passe, la note « recherche écourtée » (`node-budget`) cesse de promettre l'agencement le plus proche : le seed ne garantit cela que d'une recherche menée à terme.

## Alternatives rejetées

- Compter quand même les `left_out` : le compteur mentirait sur ce que la grille montre — exactement la dérive inverse.
- Re-vérifier la proposition avant de l'adopter : une requête de plus, et une latence de plus, pour un prédicat purement local.
- Fusionner la proposition avec l'ancienne grille (garder les sièges perdus) : produirait un agencement que le solveur n'a jamais validé, sur lequel la prochaine vérification échouerait.
