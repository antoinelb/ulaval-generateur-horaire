# Catalogue : artefact commité et politique d'échec du scrape du catalogue

Date : 2026-07-13

> **Partiellement remplacé** (2026-07-17) : le e2e du catalogue ne scrape plus la facette GEX en direct — il parse des pages HTML gelées, comparées au même `gex.json`.
> Voir `2026-07-catalogue-teste-sur-html-gele.md`. Tout le reste (artefact commité, schéma, phases, politique d'échec) reste en vigueur.

## Contexte

Le scrape du catalogue (https://www.ulaval.ca/etudes/cours, ~10 000 cours, ~205 pages) produit les entrées `{code, title, url}` qui alimentent ensuite le scrape des pages cours.
Le plan ne nommait que `data/cours/{session}.json` et `data/programmes.json` ; le sort du catalogue (transitoire ? persistant ?) n'était pas décidé, et les fixtures par page de l'ADR `2026-07-cas-de-test-catalogue-facette-gex` étaient remises en question.

## Décision

- **Le catalogue est un artefact commité** : `data/catalogue.json`, un seul fichier global, remplacé atomiquement comme les autres snapshots.
  Il sert de file de travail au scrape des pages cours (la reprise = diff du catalogue contre les cours déjà scrapés) et de donnée produit potentielle (recherche v1).
- **Schéma** : `{"courses": [{code, title, url}]}` — entrées **triées et dédupliquées par code** à l'écriture (diffs git stables), `url` relative (jointe à `https://www.ulaval.ca` par le consommateur).
  `total_results` (le « N résultats » de la page) est un contrôle d'intégrité au moment du scrape, pas une donnée stockée (il égalerait la longueur du tableau).
- **Types dans `core`** (`catalogue.rs` : `Catalogue`, `CatalogueEntry`) — l'artefact est une donnée produit, donc compilable en WASM ; le parsing HTML reste dans `scraper`.
- **Fixture fusionnée** : `tests/fixtures/test_cases/catalogue/gex.json` (les 52 cours GEX) remplace les trois fichiers par page.
  Le e2e scrape la facette GEX en direct (pages jusqu'à la page vide → fusion → tri → dédup) et compare au fichier ; il est insensible à une repagination (50 → 60 par page) et n'échoue que sur un vrai changement de contenu ou de markup.
  Les comportements par page (page vide = signal de terminaison, extraction de `total_results`, entrée malformée) seront épinglés par des tests unitaires du parseur avec extraits HTML en ligne.
- **Phases séquentielles** : le catalogue est écrit (atomiquement) avant que le scrape des pages cours ne commence ; à ~10 req/s le catalogue prend ~20 s, le parallélisme n'apporterait rien et coûterait le point de reprise net.
- **Politique d'échec — écrire quand même, avec garde-fous** :
  - toute anomalie (compte ≠ `total_results`, codes dupliqués contradictoires, entrée ou page non reconnaissable) est ajoutée **brute, une par ligne**, à `data/catalogue.erreurs.log`, commité à côté de l'artefact — la dérive du markup devient diffable dans l'historique ; le cron CI alertera quand le fichier est non vide ;
  - **plancher de rétrécissement à 90 %** : `data/catalogue.json` n'est écrit que si le nouveau compte ≥ 90 % du compte commité ; sous le plancher, l'ancien fichier est conservé et le processus sort en erreur (cas visé : une dérive de markup qui viderait des pages entières). Pas de fichier précédent → écriture inconditionnelle ;
  - doublons identiques entre pages (décalage de pagination) dédupliqués silencieusement ; doublons contradictoires : première occurrence conservée, conflit journalisé ;
  - terminaison = page à 0 entrée **avec** l'élément `total-resultats` présent ; 0 entrée **sans** lui = dérive de markup = erreur, pas fin des résultats.
- **Un scrape scopé (par matière) n'écrit jamais `data/`** : sortie vers `--output` ou stdout ; seul le scrape complet touche `data/catalogue.json`. Le e2e appelle la fonction de bibliothèque directement, sans écriture.

## Alternatives rejetées

- **Catalogue transitoire (en mémoire)** : un scrape de ~20 min interrompu repartirait de zéro ; le fichier commité est le point de reprise naturel.
- **Fixtures par page conservées** : cassent faussement à la moindre repagination du site alors que les tests roulent sur HTML frais ; l'étape fusion/tri/dédup n'aurait pas de sortie attendue.
- **Échec dur sans écriture à la moindre anomalie** : jette des données partielles utiles ; l'utilisateur préfère écrire avec erreurs journalisées, le plancher couvrant le cas catastrophique.
- **Avertissements purement informatifs (commit systématique)** : personne ne lit les logs d'un cron ; un catalogue vide remplacerait 10 000 cours avec un pipeline vert.
- **`total_results` stocké dans l'artefact** : redondant avec la longueur du tableau.
- **Fusion d'un scrape scopé dans le catalogue** : sémantique de fusion à concevoir (cours disparu de la facette ?) sans consommateur actuel.
