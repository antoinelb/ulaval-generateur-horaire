# Deux familles de fixtures pour le substrat B : `organigrammes/` et `rules/`

Date : 2026-07-29

## Contexte

Le solveur B (placement d'organigramme, jalons 7–9) et le vérificateur de règles (`rules.rs`, jalon 8) n'ont encore aucun code.
Comme `schedules/` l'a fait pour le solveur A, leurs cas de test sont écrits d'abord, dérivés des deux organigrammes PDF du bac GEX (admission automne A1→H8 et admission hiver H1→A8) et des sept programmes réels de `data/programmes/`.
Le solveur B place une liste de cours donnée ; la couverture des règles est une fonction pure séparée — deux contrats distincts, donc deux familles.

## Décision

Deux nouveaux répertoires sous `tests/fixtures/test_cases/`, nommés d'après les modules futurs (`organigramme.rs`, `rules.rs`) :

- `organigrammes/` — le placement : sessions, préalables, plafonds, veto hebdomadaire ;
- `rules/` — le rapport de couverture : règles × concentrations × profils.

Conventions héritées de `schedules/` : un phénomène par fichier, nom kebab-case, clés JSON anglaises, entrées en tête et `expected` en queue, sortie attendue dérivée par une implémentation de référence indépendante puis relue à la main (`2026-07-fixture-attendue-derivee-avant-le-parseur`).

Politique d'embarquement — tout objet est complet, jamais allégé :

- un cours réel = son enregistrement du snapshot 2026 le plus récent qui le contient, `seasons` = union des sous-arbres a2026/h2026/e2026 copiés verbatim ;
- un cours synthétique (`TST-*`) = un `Course` structurellement complet, une option présentielle par saison offerte, plages sans chevauchement ;
- un cours absent de tout snapshot (GCI-1011, retiré de l'offre mais présent au PDF) = fabriqué depuis le tableau du PDF, `seasons: {}`, utilisable en `passed` seulement ;
- un programme = le fichier `data/programmes/{slug}-2026.json` embarqué entier, identique en valeur.

L'identité avec les sources se vérifie au niveau des valeurs JSON (les fixtures ré-indentent), par `tests/reference/solveur_b/extract.py --verify`.

## Alternatives rejetées

- **Référencer les programmes par chemin plutôt que les embarquer** : plus léger, mais casse l'autonomie d'un fichier de fixture — la convention `schedules/` embarque, et un harnais qui suit des chemins acquiert une dépendance d'ordre de lecture.
- **Des cours minimaux synthétiques (code, crédits, saisons, préalables seulement)** : allège les fichiers mais crée une seconde forme de `Course` que le harnais devrait accepter ; la forme unique complète garde `serde` comme seul contrat.
- **Une seule famille mélangée** : les deux contrats n'ont ni les mêmes entrées ni la même sortie ; les mélanger forcerait des champs optionnels croisés vides partout.
