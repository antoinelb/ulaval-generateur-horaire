# Badge de règle en fraction, puce de contrainte supprimée

## Goal
Le badge d'une règle affiche toujours le compte contre la contrainte — « ✓ 13/12 cr », « 6/9 cr », « ✓ 1/1 » — jamais le cours sélectionné, et la puce de contrainte de l'en-tête disparaît.

## Out of scope
La progression de portée (« x/12 cr » du groupe) et le badge des Obligatoires (« 3/5 ») restent tels quels.
Le lead « Choisissez de 3 à 9 crédits… » reste tel quel.
Aucun changement dans `core` ni dans le rapport de couverture.

## Constraints
Dénominateur = le max de la contrainte, partout (satisfait comme incomplet) ; quand min == max ils coïncident.
Numérateur = le compte réel (cours ou crédits comptés), jamais clampé — « 13/12 cr » est légal.
Le crochet reste sur le badge satisfait : « ✓ 13/12 cr » ; une règle satisfaite sans contrainte garde « ✓ » seul.
Le suffixe « - en sus » (`credits_in_addition`) ne doit pas disparaître avec la puce : il suit le libellé du badge.
`bare_section` (règle sans rapport, ADR verdicts-honnetes) ne compte rien : son badge neutre porte le libellé de contrainte (« 12 cr », « 1–8 parmi - en sus ») au lieu de « — », sans inventer de numérateur.
AIR : l'état reste porté par le texte du badge, pas seulement sa couleur.

## Items
1. `incomplete_badge` (`crates/ui/src/panel.rs`) : dénominateur min → max pour les deux constraintes (cours et crédits).
2. `rule_badge` satisfait : remplacer « ✓ {cours} »/« ✓ » par « ✓ {compté}/{max} » (« cr » pour les crédits) quand la règle a une contrainte ; « ✓ » seul sinon.
3. Suffixe « - en sus » : appliqué au libellé du badge quand `credits_in_addition`, en réutilisant la logique de `constraint_label`.
4. Suppression de la puce : champ `Section.constraint` retiré du modèle et son rendu (`panel-rule-constraint`, `crates/ui/src/components/panel.rs`) supprimé, CSS orphelin inclus.
5. `bare_section` : le badge neutre devient le libellé de contrainte quand la règle en a une, « — » sinon.
6. Tests de `crates/ui/src/panel.rs` mis à jour (dont celui qui lit `constraint`) et couverture ramenée à 100 %.

## Acceptance
Une règle « 12 cr » satisfaite avec 13 cr comptés affiche « ✓ 13/12 cr » ; une « 3–9 cr » à 6 cr affiche « 6/9 cr » ; une « 1 parmi » satisfaite affiche « ✓ 1/1 » sans code de cours.
Aucune puce de contrainte dans l'en-tête des règles ; une règle sans rapport garde sa contrainte visible dans son badge neutre.
`make lint && make test` verts, couverture 100 %.

## Check
`make lint && make test`
