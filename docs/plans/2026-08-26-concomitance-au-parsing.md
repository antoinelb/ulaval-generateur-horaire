# L'étoile de concomitance survit au parsing et le solveur l'honore par feuille

## Goal
Le `*` du répertoire (« peut être suivi en concomitance ») est conservé dans l'arbre de préalables parsé, et le solveur accepte « même session » exactement pour les feuilles étoilées — le réglage global actuel n'est plus qu'une dérogation.

## Out of scope
Les horaires hebdomadaires (la concomitance n'y change rien : deux cours de la même session doivent déjà cohabiter).
Le retrait du réglage global `concomitant` (il reste, resémantisé en dérogation).
Le re-scrape lui-même (déclenché après la livraison, par le cron habituel).

## Constraints
La sérialisation de `data/cours.json` doit rester rétrocompatible pour le dépôt JS grille-de-cheminement : feuille chaîne = stricte, nouvelle feuille étiquetée = concomitance permise.
Rien d'étoilé n'est perdu ni avalé : une étoile sur un opérande hors grammaire reste dans le `raw` comme aujourd'hui.
Couverture 100 % maintenue à chaque item.

## Items
1. Modèle core : `PrereqTree` gagne une feuille concomitante, sérialisée `{"concomitant": "GCI-2010"}` à côté des feuilles chaîne ; `FlatNode::Course` gagne le drapeau correspondant.
2. Grammaire des préalables (`core/src/parser`) : le tokeniseur garde le `*` accolé au sigle et émet la feuille étiquetée ; le texte `raw` la montre déjà.
3. Fixtures parser : les `tests/fixtures/test_cases/courses/*.json` dont la page porte des étoiles régénérés, avec un cas dédié au `*`.
4. Solveur B : l'évaluation de précédence accepte session′ ≤ session pour une feuille étoilée ; le réglage global `concomitant` étend cette lecture aux feuilles non étoilées (dérogation), défaut inchangé ; `unmet_prerequisites`/le blâme distinguent « manquant » de « suivi en même temps sans étoile ».
5. Fonctions statiques : `prerequisites_met`/`unmet_prerequisites` reçoivent l'ensemble « même session » en plus de l'acquis strict, pour que la question par rangée du panneau et le diagnostic d'épinglage suivent la même lecture.
6. Import Capsule : la détection `history_needs_concomitance` et l'ouverture du toggle se retirent — les étoiles portent l'information ; l'ADR `2026-08-concomitance-ouverte-par-le-releve` est marquée remplacée.
7. Données : `data/cours.json` régénéré par le scraper (les corrections de `cours.manuel.json` acceptent l'étoile dans leur grammaire sans changement).
8. Aval : entrée au registre CORRECTIFS-AMONT du dépôt grille-de-cheminement pour la nouvelle feuille.
9. UI : l'affichage des préalables marque la concomitance (« GCI-2010 — concomitance permise »).

## Acceptance
Le relevé d'exemple s'importe sans ouvrir le réglage global et l'organigramme se propose complet (GEX-3001 et MAT-2910 placés à leurs vraies sessions).
Un cours étoilé se place en même session que son préalable sans dérogation ; un cours non étoilé ne le peut toujours pas par défaut.
Fixtures parser vertes, couverture 100 %.

## Check
`make lint && make test`
