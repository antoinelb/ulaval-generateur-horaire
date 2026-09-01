# Générateur d'horaire

Le Générateur d'horaire est un outil de génération d'horaire et de planification de cheminement.
Il aide à bâtir un horaire hebdomadaire sans conflit pour une session donnée et à planifier un cheminement complet (organigramme A1→H8) sous les règles d'un programme.
Il est développé à l'Université Laval, Québec, Canada, pour Daniel Nadeau, directeur du baccalauréat en génie des eaux (GEX).

📋 **[Plan du projet](docs/project_plan.md)** · 📐 **[Documents de conception](docs/conception/)**

[**Ajouter un programme ou un millésime**](#ajouter-un-programme-ou-un-millésime)
| [**Utilisation**](#utilisation)
| [**Développement**](#développement)
| [**Références**](#références)

## Ajouter un programme ou un millésime

L'application ne couvre que les programmes dont un snapshot a été publié ou qui peut être directement ajouté du site web de l'Université Laval.
Un *millésime* est la version d'un programme telle qu'offerte à une session d'admission donnée (par exemple automne 2025) : les exigences évoluent d'une admission à l'autre, et chaque étudiant est tenu à celles de son admission.
Pour ajouter un programme manquant — ou un millésime manquant d'un programme déjà couvert — il suffit d'[ouvrir un issue](https://github.com/antoinelb/ulaval-generateur-horaire/issues/new) avec soit le fichier json du programme (voir les exemples dans `https://github.com/antoinelb/ulaval-generateur-horaire/tree/main/data/programmes` ou l'URL de la page du programme sur ulaval.ca et les modifications à y apporter au besoin.

## Utilisation

L'application est entièrement statique, sans serveur ni base de données.
Les données proviennent de snapshots JSON produits par le scraper sur un cron d'intégration continue.
Les programmes ne sont donc mis à jour qu'à certains moments durant l'année.
Les solveurs tournent dans le navigateur ; l'état de l'utilisateur vit dans le `localStorage` et un cheminement se partage par URL.

L'application est publiée sur GitHub Pages : <https://antoinelb.github.io/ulaval-generateur-horaire/>.

## Développement

Un workspace Cargo, tout en Rust :

```
crates/core/     # logique du domaine, zéro IO ; compile en natif et en WASM
crates/scraper/  # binaire natif async : pages ULaval → snapshots JSON
crates/wasm/     # les solveurs compilés en WASM pour l'interface
crates/ui/       # interface Dioxus 0.7 (rendu client, web seulement)
data/            # snapshots commis (cours et programmes)
```

### Cibles make

```sh
make static    # fmt + clippy (natif --all-features, puis wasm32), avertissement = erreur
make test      # cargo +nightly llvm-cov — 100 % de couverture exigé
make e2e       # suite navigateur Playwright sur le site construit
make ui        # dx serve, disponible à http://localhost:8080 (requiert dioxus-cli)
make ui-build  # site déployable dans _ui/public
```

### Intégration continue

- `ci.yml` : lint, tests et suite navigateur sur chaque push et pull request, puis (hors PR) déploiement sur GitHub Pages.
- `scrape.yml` : cron quotidien gardé par `data/dates_scraping.txt` ; scrape complet, commit atomique des snapshots, redéclenchement du déploiement.

Chaque décision est consignée dans un ADR sous [`docs/conception/adr/`](docs/conception/adr/).

## Références

- [Plan du projet](docs/project_plan.md) — portée, contraintes et jalons.
- [Documents de conception](docs/conception/) — historique de conception et décisions (ADR).
