# CLI : logique dans la lib, erreurs style clap, sortie par répertoire

Date : 2026-07-17

## Contexte

La sous-commande `catalogue` (jalon 1) devait être câblée sans sacrifier deux exigences : couverture mesurée à 100 % (le makefile exclut `main.rs`) et messages d'erreur courts — pas le mur d'aide complet à chaque faute de frappe.

## Décision

- **Toute la logique CLI vit dans `cli.rs` (lib, mesurée)** : `pub async fn run(args: Vec<String>)`. `main.rs` reste un shim : `env::args()` → `cli::run` → `ExitCode`.
- **Erreurs style clap/uv** (mesuré sur uv) : une erreur imprime 3 blocs courts (`error: …`, une ligne `Usage:`, renvoi à `--help`) sur stderr et sort avec le code 2 ; l'aide complète n'apparaît que sur demande (`--help`, `-h`, aucun argument — code 0). `main.rs` intercepte l'erreur (`eprintln!("{error:#}")`) pour supprimer le préfixe `Error:` et la backtrace du `Termination` par défaut.
- **Sortie par répertoire** : `ulaval-scraper catalogue <output_dir> <url>` écrit `<output_dir>/catalogue.json` (remplacement atomique : `.tmp` voisin puis `rename`) et `<output_dir>/catalogue_errors.log` (anomalies brutes, une par ligne). Un log périmé est supprimé lors d'un scrape propre — sinon il alerterait le cron pour toujours. Le nom anglais `catalogue_errors.log` remplace le `catalogue.erreurs.log` de l'ADR `2026-07-catalogue-artefact-commite` (cohérence code-en-anglais).
- **Trois niveaux de tests** : unitaires (parsing d'arguments, `format_usage`, `write_atomic`, chemins d'erreur), intégration (`cli::run` contre wiremock, matrice complète d'échecs d'écriture par obstruction de chemins), e2e (le binaire compilé via `CARGO_BIN_EXE_ulaval-scraper` : codes de sortie 0/2, contenu de stderr).
- **`print::TEST_STATE_LOCK`** : les tests qui pilotent l'état global d'affichage (dans `print.rs` ou ailleurs) le sérialisent via ce verrou ; ça remplace la règle « un seul test global ». Nécessaire pour exécuter le chemin heureux aussi dans le binaire de tests unitaires — sinon les instanciations mortes comptent comme régions manquées fantômes (même artefact que dans `2026-07-couverture-100-et-frontiere-io`).

## Alternatives rejetées

- **`clap`** : une dépendance lourde pour une sous-commande à deux arguments positionnels ; les slice patterns portent la grammaire.
- **`bail!(usage)` avec l'aide complète** : le message d'erreur se noie dans le mur de texte ; c'est exactement l'anti-patron que clap évite.
- **`scrape_all(urls)` multi-URL** : spéculatif — les facettes matières se combinent dans une seule URL de requête ; retiré.
- **Chemin de sortie passé à `scrape`** : couplait produire et persister ; la testabilité voulue vient de `run(args)` injectable.
- **`pub mod print`** : l'API d'affichage reste privée à la crate (choix utilisateur) ; les fonctions de progression pas encore appelées portent un `#[allow(dead_code)]` ciblé jusqu'au scrape des pages cours.
