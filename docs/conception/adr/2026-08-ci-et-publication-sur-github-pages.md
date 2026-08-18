# CI, module wasm et données publiés sur GitHub Pages

Date : 2026-08-09

## Contexte

Le plan prévoyait depuis le début une CI (statique + tests), un déploiement statique automatique et le cron de scraping piloté par `data/dates_scraping.txt` (ADR `2026-08-scraping-pilote-par-fichier-de-dates`), mais aucun workflow n'existait.
Le module wasm (`wasm-pack --target web`, ADR `2026-08-module-wasm-quatre-fonctions-js`) doit être importable depuis n'importe quel fichier HTML sur le web, et les snapshots de données doivent être servis d'un endroit gratuit.

## Décision

Deux workflows GitHub Actions :

- **`ci.yml`** (push sur `main`, pull request, `workflow_dispatch`) — trois jobs :
  - `static` : `cargo fmt --all --check` (le makefile formate en place, la CI vérifie seulement), clippy workspace `-D warnings`, clippy du crate wasm sur la cible `wasm32-unknown-unknown` — les mêmes commandes que `make static`.
  - `test` : `cargo llvm-cov` sur nightly avec les exclusions de `make test`, plus `--fail-under-lines 100` — la règle « 100 % une fois la feature finie » devient vérifiée mécaniquement.
  - `deploy` (après `static` + `test`, jamais sur pull request) : `wasm-pack build crates/wasm --target web`, puis publication sur **GitHub Pages** d'un site contenant `pkg/` (le module ES) et `data/` (`catalogue.json`, `cours.json`, `programmes/`). Pas de page d'accueil : la vraie UI prendra la racine plus tard (fait — ADR `2026-08-interface-publiee-a-la-racine-de-pages` ; la duplication des commandes du makefile dans le workflow a été supprimée par l'ADR `2026-08-makefile-definition-unique-de-la-ci`).
- **`scrape.yml`** (cron quotidien 09:30 UTC + `workflow_dispatch` avec input `force`) : garde de date par `grep` du `mm-jj` du jour dans `data/dates_scraping.txt` ; scrape complet `catalogue` → `courses` → `program` ; commit et push des snapshots par le bot ; redéclenchement de `ci.yml` par `gh workflow run` (un push fait avec le `GITHUB_TOKEN` ne déclenche jamais d'autre workflow — `workflow_dispatch` est l'exception documentée) ; enfin, **échec du job après le commit** si un `*_errors.log` existe — le snapshot valide est préservé (écritures atomiques, garde ≥ 90 % du catalogue) et l'échec sert de canal de notification gratuit : GitHub envoie un courriel au propriétaire du workflow.

GitHub Pages sert les sites publics avec `Access-Control-Allow-Origin: *` : n'importe quel HTML peut faire `import init, {generate_schedule} from "https://<user>.github.io/ulaval-generateur-horaire/pkg/ulaval_scheduler_wasm.js"`, et charger `…/data/cours.json` de la même origine.
Étape manuelle unique : activer Pages avec la source « GitHub Actions » dans les réglages du dépôt.

## Alternatives rejetées

- **Publication npm** : compte + secret `NPM_TOKEN` + gestion de versions, pour aucun consommateur à bundler connu ; l'import par URL couvre le besoin réel.
- **jsDelivr comme emplacement primaire** (`cdn.jsdelivr.net/gh/…`) : miroir gratuit possible plus tard sans rien changer, mais Pages donne une origine stable unique pour la lib et les données.
- **`raw.githubusercontent.com`** : CORS correct mais pas pensé comme CDN de production (pas de compression garantie, types MIME approximatifs).
- **Canal de notification dédié** (webhook, courriel custom) : l'échec de job + courriel GitHub par défaut suffit — zéro secret, zéro service externe.
