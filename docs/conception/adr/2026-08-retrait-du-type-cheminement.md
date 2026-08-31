# Retrait du type `Cheminement` et de son test de répertoire

Date : 2026-08-30

## Contexte

Le commit `248c8df` a supprimé les 26 fichiers de `data/cheminements/`, dernier reste d'un format que plus rien ne lit : l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` avait déjà retiré de l'interface l'aller-retour `.json` du cheminement, et avec lui `crates/ui/src/cheminement.rs`.

Le test `every_cheminement_file_parses_with_the_core_type` (`crates/scraper/tests/integration/manual.rs`) a survécu à ses données :

```rust
let entries = fs::read_dir(&dir)
    .expect("data/cheminements is readable")
    .flatten();
```

`read_dir` sur un répertoire absent rend `Err`, donc l'`expect` panique : `make test` échouait sur `dev`, sans rapport avec le travail en cours.

Le type lui-même, `core::Cheminement`, n'avait plus **aucun** consommateur en production — vérifié sur les quatre crates. Ses 79 régions n'étaient couvertes que par ses propres tests inline, sur des littéraux : du code dont le seul usage était de se tester lui-même.

## Décision

- **Le test disparaît avec les fichiers qu'il gardait.** Un test qui lit un répertoire supprimé n'est pas une garde, c'est une panne.
- **`crates/core/src/cheminement.rs` disparaît aussi**, avec son `pub mod` et son `pub use` dans `lib.rs`. Un type public que rien n'instancie hors de ses propres tests n'est pas une API : c'est de la couverture achetée à vide, et le seuil de 100 % perd son sens s'il peut être tenu par du code sans usage.
- **Le garde `.manuel.json` de `crates/scraper/src/cli.rs` reste**, malgré son commentaire qui évoque les cheminements : il protège autre chose — un fichier tenu à la main dans `data/programmes/` ne doit pas être lu comme un instantané.

## Alternatives rejetées

- **Ne retirer que le test** : `make test` repassait au vert, mais laissait un type public sans consommateur, dont les tests inline masquent l'inutilité derrière un 100 % flatteur.
- **Restaurer `data/cheminements/`** : les grilles écrites à la main ont servi à vérifier le solveur entrée par entrée pendant la conversion depuis le dépôt JS ; cette vérification est faite, et la reconduire n'a de valeur que si quelque chose les relit un jour. Elles restent dans l'historique git, récupérables par `git checkout 248c8df~1 -- data/cheminements/`.
- **Garder le type « au cas où » l'interface relirait des cheminements** : l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` a tranché l'inverse — « Partager » est le round trip, le tiroir Capsule la voie d'entrée. Un type gardé pour un besoin qu'une décision a écarté est du poids mort.

## Conséquences

Dépasse `2026-08-un-cheminement-par-fichier` sur ce qu'il en restait : le format un-cheminement-par-fichier n'a plus ni fichiers, ni type, ni test.
Le `CLAUDE.md` est mis à jour en conséquence — il décrivait le répertoire et le type comme existants.
