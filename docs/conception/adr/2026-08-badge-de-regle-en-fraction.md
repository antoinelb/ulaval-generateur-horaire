# Le badge d'une règle affiche une fraction, la puce de contrainte disparaît

> **Amendé le 2026-08-30.** Le dépassement n'est plus refusé en amont : un badge « 15/12 cr » est désormais un état observable, en rouge, et c'est exactement ce que le numérateur non borné de `constraint_fraction` rendait déjà possible (ADR `2026-08-depassement-de-regle-en-statut-rouge`).

Date : 2026-08-24

## Contexte

Le badge satisfait nommait le cours compté (« ✓ GMN-1000 »), ce qui n'apprenait rien sur l'exigence.
La contrainte vivait dans une puce séparée de l'en-tête, dupliquant l'information du badge.

## Décision

Le badge d'une règle munie d'une contrainte affiche le comptage sur le maximum de cette contrainte (« ✓ 13/12 cr », « 6/9 cr », « ✓ 1/1 »).
`constraint_fraction` fait confiance au compte que porte le rapport : elle ne le revalide jamais contre le maximum, donc le numérateur n'y est jamais borné.
En pratique, `coverage_report` (core) refuse déjà un dépassement avant de produire un rapport `Satisfied` — `CountOverMax`/`CreditsOverMax` (`crates/core/src/rules.rs`) — si bien qu'aucun badge affiché aujourd'hui ne peut dépasser son maximum ; la garantie protège le contrat de `constraint_fraction` elle-même (elle ne doit pas se mettre à clamper), pas un état observable dans l'application telle qu'elle est livrée.
Trois cas restent sans fraction, faute de contrainte à compter contre : une règle satisfaite sans contrainte garde « ✓ » seul ; une règle incomplète sans contrainte nomme ce qui manque quand le rapport le donne (« 2 à combler », « 6 cr à combler ») et affiche « — » sinon ; une règle « reported » sans contrainte comptable affiche « — ».
Le crochet reste sur le badge satisfait.
Le suffixe « - en sus » (`credits_in_addition`) suit le libellé du badge.
La puce `panel-rule-constraint` est supprimée.
`bare_section` (règle affichée sans comptage, ADR `2026-08-verdicts-honnetes-et-panneau-jamais-vide`) porte le libellé de contrainte dans son badge neutre au lieu de « — », sans inventer de numérateur.
L'état reste porté par le texte du badge, pas par sa couleur (AIR INP-3).

## Alternatives rejetées

- **Garder le min au dénominateur** — un « 6/6 cr » sur une règle 6–9 laisserait croire la règle close alors que trois crédits y sont encore admissibles.
- **Garder la puce à côté du badge** — deux endroits pour la même exigence, et l'en-tête d'une règle longue est déjà chargé.
- **Borner le numérateur au max dans `constraint_fraction`** — même si `core` empêche aujourd'hui qu'un rapport satisfait dépasse son maximum, clamper dans la fonction d'affichage masquerait silencieusement un futur dépassement plutôt que de le rendre visible.
