# Tests unitaires en ligne et exclusion de la couverture

Date : 2026-07-13

## Contexte

L'ADR `2026-07-structure-des-tests-et-fixtures` plaçait les tests unitaires dans une cible externe `tests/unit/`, au motif qu'un test externe vérifie aussi l'utilisabilité de l'API publique.
Deux constats ont remis ce choix en question :

- Une cible externe compile comme une crate séparée et ne voit que les items `pub` ; or les grammaires à venir (préalables, règles) auront des fonctions privées à tester directement.
- Les tests dans `src/` sont instrumentés par `cargo llvm-cov` : `make test` affichait 99,49 % de régions sur `course.rs` à cause des bras `_ => false` jamais pris des `matches!` dans les assertions — des régions impossibles à couvrir sans faire échouer le test.
  Le total (97,55 %) était de plus gonflé par le code de test compté comme code couvert.

## Décision

- **Les tests unitaires vivent en ligne**, dans un module `#[cfg(test)] mod tests` au bas du fichier qu'ils épinglent (`common.rs`, `course.rs`, `program.rs`) ; la cible externe `tests/unit/` est supprimée.
  Seule la cible `integration` (round-trip sur fixtures réelles) subsiste, inchangée.
- **Ces modules sont exclus de la couverture** par l'attribut `#[cfg_attr(coverage_nightly, coverage(off))]`, avec la porte de fonctionnalité `#![cfg_attr(coverage_nightly, feature(coverage_attribute))]` à la racine de la crate.
  `cargo-llvm-cov` définit `cfg(coverage_nightly)` pendant `make test` (toolchain nightly) ; les builds stables ne voient aucun des deux attributs.
  Le cfg est déclaré dans `[workspace.lints.rust] unexpected_cfgs` (`check-cfg`) pour rester net sous `-D warnings`.
- Toute crate qui adoptera des tests en ligne (p. ex. `scraper` pour son parseur) reprend le même patron : porte de fonctionnalité à sa racine + attribut sur chaque module de tests.

La couverture mesure ainsi uniquement le code du domaine exercé par les tests : `core` affiche 100 % de régions, et les binaires vides (`scraper`, `ui`) partent d'un zéro honnête au lieu d'être dilués dans un total déjà vert.

## Alternatives rejetées

- **Garder la cible externe `tests/unit/`** : couverture propre par défaut, mais aucun accès aux items privés — bloquant pour tester les fonctions internes des futurs parseurs.
- **Fichiers de tests greffés par `#[path]` (`src/tests/*.rs`)** : accès privé et exclusion par regex, mais l'indirection `#[path]` casse la convention module ↔ fichier ; un lecteur ne retrouve pas le parent du module sans chercher la déclaration.
- **Modules enfants `src/<module>/tests.rs`** : équivalent fonctionnel, au prix d'un sous-dossier par fichier testé ; la localité du module en ligne a été préférée.
- **Exclusion par `--ignore-filename-regex` seule** : impossible pour des tests en ligne (même fichier que le code) ; l'attribut porte l'exclusion au point de déclaration au lieu d'une convention de chemin dans le makefile.
