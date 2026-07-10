# Générateur d'horaire

Le Générateur d'horaire est un outil de génération d'horaire et de planification de cheminement.
Il aide à bâtir un horaire hebdomadaire sans conflit pour une session donnée et à planifier un cheminement complet (organigramme A1→H8) sous les règles d'un programme.
Il est développé à l'Université Laval, Québec, Canada, pour Daniel Nadeau, directeur du baccalauréat en génie des eaux (GEX).

> **Statut :** conception terminée, implémentation en cours.

📋 **[Plan du projet](docs/project_plan.md)** · 📐 **[Documents de conception](docs/conception/)**

[**Utilisation**](#utilisation)
| [**Développement**](#développement)
| [**Références**](#références)

## Utilisation

L'application est entièrement statique, sans serveur ni base de données.
Les données proviennent de snapshots JSON par session, produits par un scraper lancé sur un cron d'intégration continue — jamais de scraping depuis l'application.
Le solveur d'horaire tourne dans le navigateur ; l'état de l'utilisateur vit dans le `localStorage` et un horaire se partage par URL.

### Prérequis

- [Rust](https://www.rust-lang.org/tools/install) (édition 2021).
- La CLI Dioxus (`dx`) : `cargo install dioxus-cli`.

### Lancer l'application

Lancer le serveur de développement pour la plateforme web (par défaut) :

```sh
dx serve
```

L'interface est disponible à http://localhost:8080.

Pour une autre plateforme, utiliser le drapeau `--platform` (features Cargo `web`, `desktop`, `mobile`) :

```sh
dx serve --platform desktop
```

## Développement

L'application est bâtie avec [Dioxus 0.7](https://dioxuslabs.com/learn/0.7) (Rust, WebAssembly, rendu côté client).

### Structure

```
assets/        # ressources statiques (favicon, styles, images)
src/           # code de l'application (point d'entrée : main.rs)
Cargo.toml     # dépendances et features de plateforme
Dioxus.toml    # configuration Dioxus (titre, ressources web)
```

### Qualité du code

```sh
cargo fmt
cargo clippy
```

## Références

- [Plan du projet](docs/project_plan.md) — portée, contraintes et jalons.
- [Documents de conception](docs/conception/) — historique de conception et décisions (ADR).
- [Documentation Dioxus 0.7](https://dioxuslabs.com/learn/0.7)
