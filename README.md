# Générateur d'horaire

Le Générateur d'horaire est un outil de génération d'horaire et de planification de cheminement.
Il aide à bâtir un horaire hebdomadaire sans conflit pour une session donnée et à planifier un cheminement complet (organigramme A1→H8) sous les règles d'un programme.
Il est développé à l'Université Laval, Québec, Canada, pour Daniel Nadeau, directeur du baccalauréat en génie des eaux (GEX).

> **Statut :** scraper (étape 1) et solveurs (étape 2) livrés ; l'interface (jalons 3–9) est en cours.

📖 **[Documentation](https://antoinelb.github.io/ulaval-generateur-horaire/docs/)** · 📋 **[Plan du projet](docs/project_plan.md)** · 📐 **[Documents de conception](docs/conception/)**

[**Utilisation**](#utilisation)
| [**Développement**](#développement)
| [**Références**](#références)

## Utilisation

L'application est entièrement statique, sans serveur ni base de données.
Les données proviennent de snapshots JSON produits par le scraper sur un cron d'intégration continue — jamais de scraping depuis l'application.
Les solveurs tournent dans le navigateur ; l'état de l'utilisateur vit dans le `localStorage` et un horaire se partage par URL.

Quatre morceaux sont publiés sur GitHub Pages, à la même origine :

| Emplacement | Contenu |
|---|---|
| [`/`](https://antoinelb.github.io/ulaval-generateur-horaire/) | l'application : le générateur d'horaire et de cheminement |
| [`/pkg`](https://antoinelb.github.io/ulaval-generateur-horaire/pkg/ulaval_scheduler_wasm.js) | le module WASM (paquet ES + `.d.ts` typé), importable par URL depuis n'importe quel HTML |
| [`/data`](https://antoinelb.github.io/ulaval-generateur-horaire/data/cours.json) | les snapshots (cours et programmes) |
| [`/docs`](https://antoinelb.github.io/ulaval-generateur-horaire/docs/) | la documentation (livre mdBook en français) |

Consommer les solveurs depuis JavaScript :

```js
import init, {generate_schedule} from "https://antoinelb.github.io/ulaval-generateur-horaire/pkg/ulaval_scheduler_wasm.js";

await init();
```

Le [guide du consommateur JavaScript](https://antoinelb.github.io/ulaval-generateur-horaire/docs/guide/chargement.html) couvre les quatre fonctions, les schémas et les erreurs.

## Développement

Un workspace Cargo, tout en Rust :

```
crates/core/     # logique du domaine, zéro IO ; compile en natif et en WASM
crates/scraper/  # binaire natif async : pages ULaval → snapshots JSON
crates/wasm/     # cdylib : les solveurs exposés au JavaScript nu
crates/ui/       # interface Dioxus 0.7 (rendu client, web seulement)
data/            # snapshots commis, servis tels quels sur Pages
docs/livre/      # source du livre mdBook publié à /docs
```

### Cibles make

```sh
make static  # fmt + clippy (natif --all-features, puis wasm32), avertissement = erreur
make test    # cargo +nightly llvm-cov — 100 % de couverture exigé
make wasm    # wasm-pack build crates/wasm --target web (requiert wasm-pack)
make docs    # mdbook build docs/livre (requiert mdbook)
```

L'interface se lance avec la CLI Dioxus (`cargo install dioxus-cli`) : `dx serve` depuis `crates/ui`, disponible à http://localhost:8080.

### Intégration continue

- `ci.yml` : `static` et `test` sur chaque push et pull request, puis (hors PR) déploiement de `pkg` + `data` + `docs` sur GitHub Pages.
- `scrape.yml` : cron quotidien gardé par `data/dates_scraping.txt` ; scrape complet, commit atomique des snapshots, redéclenchement du déploiement.

Chaque décision est consignée dans un ADR sous [`docs/conception/adr/`](docs/conception/adr/).

### Assistants de programmation

Le dépôt prend en charge Codex et Claude Code en parallèle.
Codex charge [`AGENTS.md`](AGENTS.md), les rôles sous `.codex/` et les compétences sous `.agents/skills/`.
Claude Code continue de charger [`CLAUDE.md`](CLAUDE.md) et `.claude/`.
Les adaptateurs Codex réutilisent les consignes Claude existantes afin de conserver une seule source de vérité pendant la transition.

## Références

- [Documentation](https://antoinelb.github.io/ulaval-generateur-horaire/docs/) — guide JavaScript, architecture et domaine.
- [Plan du projet](docs/project_plan.md) — portée, contraintes et jalons.
- [Documents de conception](docs/conception/) — historique de conception et décisions (ADR).
