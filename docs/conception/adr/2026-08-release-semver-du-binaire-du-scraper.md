# Le binaire du scraper publié en release semver

Date : 2026-08-09

## Contexte

Le dépôt frère `grille-de-cheminement-interactive` (le frontend livré) veut rafraîchir ses propres snapshots par un cron, plutôt que consommer ceux servis sur Pages ici.
Il n'a aucune toolchain Rust : c'est un site HTML/CSS/JS vanilla, sans étape de build.
Compiler le scraper dans son cron voudrait dire installer Rust et compiler le workspace à chaque run, pour un binaire qui ne change que quelques fois par année.

## Décision

Un workflow `release.yml`, déclenché par un tag git `v*` (plus `workflow_dispatch` avec le tag en input pour republier), compile `ulaval-scheduler-scraper` en release et attache le binaire `ulaval-scraper` à une release GitHub du même nom que le tag.
Les versions suivent le semver du workspace (`Cargo.toml`, `version = "0.1.0"`) : un tag est posé à la main quand le comportement du scraper change d'une manière qui intéresse un consommateur.
Un consommateur télécharge la dernière release sans épingler : `gh release download -R antoinelb/ulaval-generateur-horaire -p ulaval-scraper`.

Le binaire est construit pour la cible par défaut du runner (glibc, `x86_64-unknown-linux-gnu`), pas en musl statique : le seul consommateur est un runner `ubuntu-latest`, la même image que le constructeur.
Une cible `x86_64-unknown-linux-musl` reste le chemin de mise à niveau si un consommateur tourne ailleurs un jour ; elle coûte aujourd'hui `musl-tools` et la cross-compilation du C de `aws-lc-sys` (la pile TLS de `reqwest`), pour un gain nul.

## Alternatives rejetées

- **Release roulante écrasée** (un tag fixe `binaire`, `--clobber` à chaque push sur `main`) : pas d'historique, aucun retour arrière possible si un scrape casse, et le consommateur ne peut pas dire quel binaire a produit un snapshot.
- **Artefact de workflow** (`actions/upload-artifact`) : expire après 90 jours et exige un token cross-repo pour être téléchargé ; une release est publique et permanente.
- **Compiler dans le cron du consommateur** : installer la toolchain et compiler le workspace à chaque run, pour un binaire quasi immuable.
- **Publication sur crates.io** : `cargo install` impose quand même une toolchain chez le consommateur, et le crate n'a aucun public de bibliothèque.
