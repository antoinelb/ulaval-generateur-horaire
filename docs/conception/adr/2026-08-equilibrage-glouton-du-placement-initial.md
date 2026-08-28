# Le premier placement est équilibré par une descente gloutonne

**Date :** 2026-08-28
**Statut :** accepté.
Compagnon de `2026-08-b-minimise-la-distance-au-seed` : les deux drapeaux sont posés par la même escalade, et jamais tous les deux.

## Contexte

Sans seed, l'ordre de valeurs de B est « la session offerte la plus tôt d'abord » (`2026-07-b-placement-par-satisfaction-fait-main`).
La première solution est donc la première descente : elle remplit A1 jusqu'au plafond, puis H1, et laisse la queue de l'horizon vide.
C'est la toute première grille qu'un étudiant voit, et elle est fausse comme conseil : personne ne planifie 17 crédits en A1 et 6 en H4.

## Décision

- **Une post-passe `balance_first_solution`**, appelée depuis `place` après la recherche, sur `solutions[0]` seulement.
- **Gardée par `balance && !allow_unplaced && !allow_credit_shortfall`.**
  Les solutions de ces deux passes portent des preuves par session — `left_out`, `credit_shortfalls` — qu'un déplacement périmerait en silence ; les recalculer est le travail de la recherche, pas d'une post-passe.
- **Descente la plus raide sur Σ(charge de session)².**
  Le carré est ce qui étale : déplacer 3 crédits d'une session à 9 vers une session à 3 fait passer 81 + 9 à 36 + 36.
  Boucle bornée par le nombre de candidats ; chaque mouvement accepté baisse strictement la somme, donc la borne ne sert que de garde-fou.
- **Un mouvement n'est retenu que si tout tient encore** : plafond de crédits, veto hebdomadaire de A (cache réutilisé), et **tous** les arbres de préalables réévalués en entier.
  La vérification incrémentale de la recherche n'est correcte que parce qu'on ajoute des placements sans jamais en déplacer ; une post-passe qui déplace doit donc tout revoir.
  `finalize` redérive ensuite la solution : les opérandes présumés dépendent de la branche d'un nœud `any` qui tient, et un déplacement peut la changer.
- **Un cours épinglé n'est jamais déplacé** — l'épinglage est un acte de l'étudiant.
- **Départage déterministe** : à gain égal, le candidat le plus tôt dans l'ordre, puis l'ordre de son domaine.
- **L'escalade pose `balance` sur la passe exacte seulement, et seulement quand il n'y a pas de seed** (`balance: request.seed.is_empty()`) : avec un seed il y a une grille à suivre, et l'équilibrage la contredirait.

## Alternatives rejetées

- **Ordre de valeurs dynamique** (préférer la session la moins chargée pendant la descente) : la charge d'une session n'est connue qu'une fois les candidats suivants placés ; l'ordre serait myope, et il changerait le premier résultat de *toutes* les recherches, y compris celles que les fixtures gèlent.
- **Objectif secondaire dans le branch and bound** : additionner deux objectifs demande une pondération arbitraire, et le cas équilibré est exactement celui où il n'y a **pas** de seed, donc pas de branch and bound du tout.
- **Statu quo** : la grille front-loaded est un mauvais conseil livré comme une proposition, et l'étudiant la corrige à la main cours par cours.
