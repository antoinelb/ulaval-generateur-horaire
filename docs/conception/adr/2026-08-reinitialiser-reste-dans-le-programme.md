# « Réinitialiser » vide le document sans quitter son programme

Date : 2026-08-29

## Contexte

Le bouton passe par `edit_plan` depuis le 2026-08-14, donc « Annuler » restaure l'organigramme (ADR `2026-08-bouton-tout-reinitialiser`, règle ACT-2 : l'annulabilité *est* la protection, jamais un dialogue de confirmation).
Mais il remettait aussi `program` à `None` (`*plan = Plan { start, ..Plan::default() }`), ce qui renvoie l'étudiant au sélecteur de programme.
Or la seule sortie du sélecteur est « Choisir », c'est-à-dire `swap_document`, qui remet `History` à zéro (ADR `2026-08-historique-par-document-vide-a-la-bascule`) — et la réinitialisation venait justement de supprimer l'étagère du programme (ADR `2026-08-reinitialiser-le-document-courant-et-son-etagere`).

Résultat rapporté le 2026-08-29 : le pas d'annulation que la réinitialisation arme meurt au clic même qui ramène l'étudiant dans son programme. Concentration choisie et cours à option placés partent sans recours, alors que le toast promet le contraire.

## Décision

- « Réinitialiser » **garde le programme, son millésime et sa portée** (concentration, profil) : c'est l'identité du document, pas son contenu. Seul le contenu repart à zéro, par `persist::reset_document`, avec l'horizon qu'un document neuf de ce programme recevrait — exactement ce que calcule « Choisir ».
- L'étudiant ne traverse donc plus le sélecteur : aucun `swap_document` ne s'interpose, et « Annuler : Réinitialisation » reste vivant tant qu'il n'a pas édité autre chose.
- La suppression de l'étagère et la survie des fiches de cours manuels ne changent pas.
- Qui veut réellement revenir au sélecteur a « changer » dans le bandeau — c'est son rôle.
- Symétriquement, « changer » pousse désormais un ✓ nommant la façon de récupérer le cheminement tabletté : « Annuler » y est éteint par conception, et une infobulle au survol n'est pas une affordance.

## Alternatives rejetées

- **Faire survivre `History` à `swap_document`** : forke l'étagère et la pile (le problème que l'ADR `2026-08-historique-par-document-vide-a-la-bascule` a tranché), et téléporterait un « Annuler » vers un autre programme.
- **Garder l'étagère pré-reset pour que « Choisir » la restaure** : le bouton mentirait — c'est ce que l'ADR `2026-08-reinitialiser-le-document-courant-et-son-etagere` a déjà refusé.
- **Une confirmation avant de réinitialiser** : ACT-2 l'interdit, et cela ne rendrait pas l'acte annulable pour autant.
