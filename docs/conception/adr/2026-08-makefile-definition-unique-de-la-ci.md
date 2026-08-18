# Le makefile est la définition unique de ce que vérifie la CI

Date : 2026-08-17

## Contexte

`ci.yml` recopiait les commandes de `make static` et `make test` au lieu de les appeler.
Les deux avaient dérivé :

- la CI n'exécutait **pas** `cargo clippy -p ulaval-scheduler-ui --target wasm32-unknown-unknown`, que le makefile exécute — or c'est le seul lint qui compile `browser.rs`, donc tout ce fichier était un angle mort. Les deux défauts corrigés en même temps que cet ADR (manifeste figé à 8 programmes, service worker de portée `/assets/`) y vivaient ;
- le makefile n'avait **pas** `--fail-under-lines 100`, que la CI a : `make test` passait localement là où le push échouait ;
- les deux `--ignore-filename-regex` différaient d'un motif.

La dérive n'est pas cosmétique : elle produit exactement la classe de bogue que la CI existe pour attraper.

## Décision

Le makefile porte les définitions, la CI les appelle :

- une cible **`lint`** contient le jeu de clippy ; `static` formate (`cargo fmt --all`) puis appelle `$(MAKE) lint`, la CI vérifie (`cargo fmt --all --check`) puis appelle `make lint`. Le formatage reste le seul écart délibéré : l'un écrit, l'autre vérifie ;
- `make test` reçoit `--fail-under-lines 100` : un échec local et un échec de CI arrivent au même endroit ;
- le job `deploy` appelle `make wasm ui-build docs` et recopie `docs/livre/book` dans `_site/docs`, au lieu d'un second appel à `mdbook` avec un `--dest-dir` différent.

Chaque écart a été réglé au plus strict : la CI gagne un lint, le makefile gagne un seuil.
`build.rs` s'ajoute aux exclusions de couverture — un build script est de la plomberie de compilation, pas du code testable.

Les jobs `static` et `test` installent désormais `wasm-pack` : leurs cibles dépendent de `ui-calc`, qui construit le module du worker.

## Alternatives rejetées

- **Aligner les deux fichiers à la main** : c'est ce qui existait ; ils avaient dérivé en une révision.
- **Faire appeler `make static` par la CI** : la cible formate en place, ce qu'un job de vérification ne doit pas faire.
- **Une cible `ci-test` distincte pour le seuil** : deux comportements pour une même commande, et la surprise au push revient. Le seuil est la règle du projet (« `make test` doit donner 100 % une fois la feature finie »), il appartient à la cible.
