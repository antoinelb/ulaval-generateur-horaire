# La case « scolarité préparatoire faite » est un invariant, pas un filtre

Date : 2026-08-14

## Contexte

Essai d'Antoine (2026-08-14) : un cours préuniversitaire restait dans l'horaire malgré la case cochée.
Le traçage a montré que core n'était pas en cause (le placeur exclut les `passed` de ses candidats) : `request_json` retirait les codes préparatoires des requêtes (`electives.retain`), mais la grille dessine `displayed_placement` brut.
Un 0xxx entré par n'importe quelle porte — rangée de règle encore actionnable, recherche, ajout par code, proposition faite case décochée puis recochée, vieille sauvegarde, lien partagé — y restait indéfiniment, pendant que la vérification disait ✓ d'un horaire qu'elle n'avait jamais vu en entier.
Le filtre côté requête masquait l'incohérence au lieu de la corriger.

## Décisions

- **L'invariant** : case cochée ⇒ aucun code de la règle « Scolarité préparatoire » n'occupe l'état de placement (`displayed_placement`, `pinned_sessions`, `electives`, `manual`, `chosen`).
- **Un effet de guérison unique** (`components::heal_preparatory`, motif de `auto_verify`) l'applique : dès que des restes existent, `state::purge_codes` les retire et une alerte les **nomme** avec l'issue (« Décochez la case pour les replacer ») — jamais silencieux, convergent (n'écrit que s'il reste des codes). Un seul mécanisme couvre toutes les portes d'entrée, présentes et futures.
- La purge est une **écriture directe, pas `edit_plan`** : une correction dérivée n'est pas un acte de l'étudiant. Aucune perte possible : annuler le cochage restaure le plan d'avant la case (l'instantané d'`edit_plan` précède la purge), et l'effet ne re-purge que si la case est encore cochée.
- **Les rangées deviennent `RowState::Acquired`** (règle *et* recherche, décidé dans `base_row` avant l'état « placé ») : ni « + », ni puces, ni select d'entente ; sous-titre « considéré comme déjà fait - décochez la case pour le placer ». `validate_new_code` refuse un code acquis avec la même issue.
- `solve::acquired_preparatory` exclut les codes sous entente en regardant `rule_grants` directement (l'entente *déplace* le cours hors de la préparatoire — ADR `2026-08-entente-cours-regle-et-scolarite-preparatoire`) plutôt qu'en clonant le programme « granted » par rangée.

## Alternatives rejetées

- Filtrer les codes préparatoires au rendu de la grille : cache l'état incohérent au lieu de le réparer — les crédits, la persistance et le partage transporteraient toujours le mensonge.
- Purger seulement au cochage de la case : ne guérit ni les vieilles sauvegardes ni les liens partagés ni les portes futures.
- Rendre la purge annulable (`edit_plan`) : une guerre d'annulation — défaire la purge la redéclencherait aussitôt, la case étant toujours cochée.
