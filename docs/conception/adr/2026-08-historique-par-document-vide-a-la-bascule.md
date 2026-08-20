# L'historique appartient au document et se vide à la bascule

## Contexte

L'historique (« Annuler ») empile des clones entiers du `Plan`. Avec l'étagère par (programme, millésime), un undo qui traverserait la bascule restaurerait le plan du programme A pendant que A est aussi sur l'étagère — deux copies divergentes du même document.

## Décision

- L'historique **ne traverse pas** la bascule : `swap_document` le remet à zéro avec la `View`. ACT-2 reste tenu — « changer » se défait en rechoisissant le programme, qui restaure l'étagère à l'identique (comme la version JS, qui se passe d'undo précisément grâce à la persistance par clé).
- `swap_document` est la **deuxième porte** à côté d'`edit_plan` (AP-6 amendé dans `.claude/dioxus.md`) : remplacer le document n'est pas l'éditer. Ordre interne : annuler la recherche en vol, écrire l'étagère (synchrone), `plan.set` en bloc, `history`/`view` à zéro, purge de `left_out`/`proposed` (qui appartiennent aux réponses de l'ancien document — l'effet plan-change ne les nettoie pas).
- **Exception : l'import par fragment** garde exactement **un** pas d'undo (historique remis à zéro puis `edit_plan` d'import) — l'issue de secours pour retrouver son propre document immédiatement. Il tablette d'abord le document courant **si** sa clé diffère de celle du plan partagé (`import_stash`) ; stasher la même clé laisserait un « changer » ultérieur écraser l'étagère par la version partagée. Limitation acceptée : importer un lien du même (programme, millésime) sans l'annuler, puis changer de programme, tablette la version partagée par-dessus l'ancienne étagère.

## Alternatives rejetées

- **Historique global traversant les documents** : le fork étagère/pile ci-dessus, et un « Annuler » qui téléporte l'étudiant vers un autre programme sans le dire.
- **Une pile d'historique par étagère** : de la mémoire et de la persistance pour un besoin que l'étagère couvre déjà.
