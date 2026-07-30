# L'implémentation de référence de B est versionnée jusqu'à l'arbitrage

Date : 2026-07-29

## Contexte

`2026-07-fixture-attendue-derivee-avant-le-parseur` exige qu'une sortie attendue écrite avant le code soit dérivée par une implémentation de référence indépendante, ancrée sur les fixtures déjà validées, et arbitrée contre la source quand le code de production diverge.
Pour les cours, la référence n'a pas été versionnée : l'arbitrage suivait de quelques jours.
Pour le solveur B, l'arbitrage est à plusieurs jalons de distance (jalons 7–9) : supprimer les scripts maintenant forcerait une réécriture au moment de l'arbitrage, et une référence réécrite n'est plus celle qui a dérivé les fixtures.

## Décision

Les scripts vivent sous `tests/reference/solveur_b/` (Python 3, stdlib seule, invisibles pour cargo et la couverture) :

- `common.py` — chargement, faisabilité hebdomadaire brute-force, évaluation de `PrereqTree`, écrivain JSON canonique (indentation 2, `ensure_ascii=False`, saut de ligne final — stable au bit) ;
- `check_anchor.py` — l'ancre : reproduire les verdicts `valid` des 18 fixtures `schedules/` avant toute dérivation ;
- `extract.py` — construction des objets embarqués depuis `data/` et `--verify` (identité en valeur avec les sources) ;
- `place.py` / `verify_rules.py` — modes `fill` (écrit `expected`) et `check` (régénère et compare au bit près).

Ils sont **supprimés après l'arbitrage** : quand l'implémentation Rust reproduit les fixtures (ou que chaque écart a été arbitré contre les organigrammes et les pages), les fixtures figées font seules autorité, comme pour les cours.

## Alternatives rejetées

- **Ne pas versionner (le précédent cours)** : correct quand l'arbitrage est imminent ; ici il détruirait la seule trace exécutable de la dérivation pendant des semaines.
- **Garder les scripts indéfiniment** : deux implémentations permanentes de la même sémantique divergent en silence ; après l'arbitrage, la référence n'a plus de rôle.
- **Écrire la référence en Rust dans le workspace** : elle entrerait dans la compilation, la couverture et les lints du code de production, et « indépendante du code sous test » devient douteux quand elle partage les types `core`.
