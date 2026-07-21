# Couverture par instanciation : combler le plus petit écart

Date : 2026-07-19

## Contexte

`make test` affichait 99,45 % de régions sur `parser/course.rs` (5 régions et 2 lignes manquées) alors que les 906 régions du fichier étaient **toutes** exécutées par au moins un test : la somme des compteurs par région donne 906/906.

Le fichier est compilé deux fois, conséquence directe des tests unitaires en ligne (ADR `2026-07-tests-unitaires-en-ligne-et-couverture`) : une fois en bibliothèque simple, liée au binaire d'intégration, une fois avec `#[cfg(test)]` pour le binaire de tests unitaires.
Les deux compilations exercent des sous-ensembles différents du fichier — l'intégration ne voit que des fixtures valides, les tests en ligne ne montent que les fragments de HTML dont ils ont besoin.

llvm-cov replie un groupe d'instanciations en prenant le **maximum** et non l'union (`RegionCoverageInfo::merge` fait `Covered = std::max(Covered, RHS.Covered)`).
Le score d'un fichier est donc le meilleur score d'une seule compilation, jamais la réunion des deux :

```
intégration      : 744/906 régions
tests unitaires  : 899/906 régions
max              : 901/906  →  les 5 régions « manquées » annoncées
```

L'ADR `2026-07-couverture-100-et-frontiere-io` avait déjà rencontré le phénomène (« instanciations mortes de llvm-cov ») et l'avait réglé du côté intégration, avec `error_paths_stay_errors_through_the_public_api`.

## Décision

1. **Le 100 % exige qu'une seule compilation couvre tout le fichier.**
   C'est la formulation opérationnelle du seuil, et elle vaut pour tout fichier à tests en ligne.
2. **On comble l'écart du côté le moins coûteux**, mesuré et non supposé.
   Pour `course.rs` : 7 régions côté tests unitaires contre 162 côté intégration — d'où le choix inverse de celui retenu pour le parseur de catalogue.
3. Les quatre comportements que seules les fixtures exerçaient reçoivent un test unitaire en ligne : préalables dans la grammaire repliés en `Prerequisites::Parsed`, dédoublonnage de deux sessions d'une même saison, saison `Été`, mode `À distance`.
   Les six fixtures gelées restent la vérification de bout en bout ; elles ne sont pas une source de couverture.
4. **Diagnostic avant écriture** : une couverture inférieure à 100 % sur un fichier à tests en ligne se ventile d'abord par compilation, à partir de `cargo +nightly llvm-cov report --json`, en regroupant les régions par nom de fonction démanglé et en comparant les compteurs des deux hachages de crate.
   Écrire un test sans cette ventilation revient à deviner de quel côté se trouve le trou.

## Alternatives rejetées

- **Combler du côté intégration**, comme pour le parseur de catalogue : 162 régions à atteindre à travers l'API publique, dont toutes les gardes de la grammaire des préalables, alors que le même contrat est déjà vérifié par les tests en ligne.
- **Accepter 99,45 %** : défendable puisque aucune ligne n'est réellement non testée, mais le seuil perd alors sa valeur de signal — un vrai trou et un artefact de repliage deviennent indiscernables.
- **Exclure les régions concernées de la mesure** : 100 % par non-mesure, déjà rejeté dans `2026-07-couverture-100-et-frontiere-io`.
- **Renoncer aux tests unitaires en ligne** pour n'avoir qu'une compilation : supprimerait l'artefact, mais reprendrait l'accès aux fonctions privées que l'ADR `2026-07-tests-unitaires-en-ligne-et-couverture` avait justement acquis.
