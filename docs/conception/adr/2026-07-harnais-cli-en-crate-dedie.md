# Le harnais CLI du solveur dans un crate dédié `crates/cli`

Date : 2026-07-29

## Contexte

Le livrable du jalon 2 est un harnais CLI qui imprime un horaire valide pour des codes de cours d'une session.
Toute la logique du solveur vit dans `core` (pur, zéro IO, compilé en WASM pour l'UI) ; le harnais n'est qu'une coquille : parser les arguments, lire `data/cours/{session}.json`, appeler `core::schedule_report`, imprimer.
Cette coquille fait de l'IO — elle ne peut pas vivre dans `core` sans tirer `serde_json`, `anyhow` et `clap` dans l'arbre de dépendances compilé pour la cible WASM.

## Décision

Un nouveau crate binaire **`crates/cli`** (`ulaval-scheduler-cli`, binaire `ulaval-scheduler`), sur le patron établi par le scraper (ADR `2026-07-cli-dans-la-lib-et-style-derreurs`, `2026-07-adoption-de-clap`) : toute la logique dans `cli.rs` (lib, mesurée), `main.rs` en shim qui imprime l'erreur `anyhow` et sort avec le code 2.
L'horaire s'imprime par la sous-commande **`schedule`** (`ulaval-scheduler schedule a2026 GEX-1002 …`) plutôt que par des positionnels nus : les commandes du solveur B (organigramme) siégeront à côté dans le même binaire.
Synchrone — IO fichier seulement, pas de tokio.
Il redéclare localement la forme du snapshot (`{"courses": [...]}`) plutôt que de dépendre du crate scraper : deux consommateurs du même JSON, pas un couplage de code.
C'est lui, l'appelant qui sait de quel fichier vient chaque offre, qui fournit l'année aux équivalences (`resolve_offering`) — un seul snapshot aujourd'hui, le repli multi-saisons plus tard.
Sortie en français (texte affiché) ; un cours à crédits variables sans pondération choisie fait remonter son erreur, jamais un défaut inventé.

## Alternatives rejetées

- **Binaire dans `crates/core`** : les dépendances d'un target contaminent tout le crate — `serde_json`/`clap`/`anyhow` seraient compilées aussi pour la lib WASM, contre l'invariant « zéro IO » du plan.
- **Sous-commande du scraper** : réutiliserait `read_snapshot` et `print.rs`, mais mélange deux préoccupations (récolte réseau asynchrone vs consultation locale) dans un binaire dont le nom dit « scraper ».
- **Dépendre du crate scraper pour `SessionSnapshot`** : tire reqwest/tokio/scraper dans l'arbre du harnais pour une struct d'une ligne.
