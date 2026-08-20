# Le site ne se publie que sur tag

Date : 2026-08-19

## Contexte

Depuis l'ADR `2026-08-ci-et-publication-sur-github-pages`, le job `deploy` de `ci.yml` publiait le site sur GitHub Pages à chaque push sur `main`.
Chaque commit devenait donc immédiatement public, sans moment de release choisi.
Or `scrape.yml` dépend de ce déploiement : après un rafraîchissement des données, il redéclenche `ci.yml` par `gh workflow run --ref main` pour que le site serve les snapshots frais — bloquer le déploiement sur tag seul aurait figé les données publiées entre deux releases.

## Décision

Le job `deploy` ne s'exécute que sur un push de tag `v*` (le motif de `release.yml` : un même tag publie le binaire du scraper et le site) ou sur `workflow_dispatch`, et le site publié respecte toujours l'invariant **code du dernier tag `v*` + `data/` de main** :

- sur push de tag, le checkout est déjà au tag ; `data/` de `origin/main` est superposé (no-op sauf scrape intercalé) ;
- sur dispatch (le redéploiement de `scrape.yml`, ou manuel), le job checkout le dernier tag `v*` par tri de version puis superpose `data/` de `origin/main` — les données restent fraîches sans jamais publier de code non tagué ;
- aucun tag `v*` dans le dépôt = échec explicite du job, jamais un déploiement silencieux de `main`.

Les jobs `static` et `test` continuent de tourner sur chaque push sur `main` et chaque pull request ; seul `deploy` est conditionnel. `scrape.yml` est inchangé.

## Alternatives rejetées

- **Strictement sur tag** (retirer le dispatch de `scrape.yml`) : les données du site stagneraient entre deux releases, contredisant la raison d'être du cron de scraping.
- **Le dispatch déploie `main` tel quel** : chaque scrape aurait publié le code non tagué du moment, vidant la garde de son sens.
- **Taguer depuis le bot de scrape** : polluerait le semver de `release.yml` avec des tags de données et déclencherait des releases du binaire sans changement de code.
