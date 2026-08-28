# « Annuler » fige l'écran restauré

Date : 2026-08-27

Amende `2026-08-organigramme-en-continu-sans-bouton`.

## Contexte

Rapport étudiante 2026-08-27 (S2) : après un épinglage qui pousse une session à 19 crédits pour un plafond de 17, « Annuler » ne rend pas l'écran précédent — il en faut deux.

L'historique n'est pas en cause : `History` clone le `Plan` entier, `displayed_placement` compris, donc `undo` restaure bien l'agencement.
C'est la suite qui le défait.
Le plan restauré viole encore la contrainte que l'épinglage avait introduite, `auto_verify` répond « aucune solution », et `auto_propose` déclenche alors sa branche de réparation.
`apply_proposal` réécrit `displayed_placement` **hors historique** — par choix assumé : une proposition est une correction dérivée, pas un acte étudiant.
L'étudiant voit donc son annulation aussitôt écrasée, et doit annuler une seconde fois.

## Décision

Un écran restauré par « Annuler » ou « Rétablir » n'est jamais réparé automatiquement.

- `History` porte un booléen privé `restored` : `apply` le remet à faux, `undo` et `redo` le posent à vrai — après un dépilage réussi seulement.
- `solve::propose_needed(unplaced, verification_empty, restored_screen)` porte la décision : un cours flottant déclenche toujours une proposition (il n'a aucune place à préserver) ; la réparation d'une grille refusée ne part que si l'écran n'a pas été restauré.
- `auto_propose` lit `history.peek().restored()` — le plan est déjà l'abonnement de l'effet.
- Une bascule de document (`swap_document`, `import_organigramme`) réinstalle `History::default()` : la réparation se ré-arme, ce qui est voulu — l'écran du document suivant n'est la restauration de personne.

## Alternatives rejetées

- Faire passer `apply_proposal` par `edit_plan` : déjà rejeté par l'ADR amendé — à fréquence automatique, l'historique se remplirait de non-actes et chaque annulation surprendrait.
- Neutraliser la réparation dès qu'une vérification échoue : c'est précisément le cas où une grille cassée par une édition doit se réorganiser.
- Comparer le plan restauré au plan d'avant pour deviner l'origine de l'écran : une heuristique là où un drapeau explicite suffit.

## Risque assumé

Après un rechargement de page, `History` est vide et donc `restored` est faux : la réparation reste armée sur un écran que l'étudiant n'a pas construit dans cette session.
C'est le comportement d'avant, conservé — l'historique ne survit pas au rechargement (`2026-08-historique-par-document-vide-a-la-bascule`).
