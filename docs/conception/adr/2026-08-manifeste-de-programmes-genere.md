# Le manifeste des snapshots de programmes est généré par `build.rs`

Date : 2026-08-17

## Contexte

`asset!()` est un macro sur **chemin littéral** : il ne peut pas lire un répertoire à la compilation.
`crates/ui/src/browser.rs` portait donc une liste écrite à la main, avec son propre avertissement : « `asset!()` is compile-time: the manifest is hardcoded, and a new program snapshot must be added here ».

Elle comptait 8 fichiers alors que `data/programmes/` en contient 24 (hors `*.manuel.json`) : tous les millésimes de B-GIN et de B-GMC produits par le scraper n'étaient jamais servis.
Un snapshot ajouté par le cron n'apparaissait dans l'application qu'après une modification de code — un oubli silencieux par construction.

## Décision

`crates/ui/build.rs` énumère `assets/data/programmes/*.json`, trie les noms et écrit `OUT_DIR/programmes.rs` :

```rust
const PROGRAMS: &[(&str, Asset)] = &[
    ("B-ANT-A26.json", asset!("/assets/data/programmes/B-ANT-A26.json")),
    // …
];
```

que `browser.rs` reprend par `include!(concat!(env!("OUT_DIR"), "/programmes.rs"))`.
`cargo:rerun-if-changed=assets/data/programmes` fait de `make ui-data` le seul décideur de ce qui embarque.

Trois précisions :

- **Les `*.manuel.json` sont écartés** : ils portent un `cheminement_type` hand-encodé, pas un `Program`. Les charger lèverait une erreur de parsing dure au démarrage.
- **Noms triés** : l'ordre est reproductible, et le départage premier-arrivé de `parse_data` (qui préfère le fichier dont le nom s'accorde avec le contenu) reste déterministe — le cas connu `B-GEX-A24.json` contenant un A26 est traité comme avant.
- **Échec bruyant, mais seulement sous wasm32** : un répertoire vide fait échouer le build script si `CARGO_CFG_TARGET_ARCH` vaut `wasm32`. Une compilation native n'a jamais besoin des snapshots (`browser.rs` et `components/` sont `#[cfg(target_arch = "wasm32")]`), donc les tests et `cargo llvm-cov` continuent de tourner sans `make ui-data` ; une compilation navigateur sans aucun programme, elle, livrerait une application vide sans rien dire.

Conséquence assumée : le sélecteur passe de 8 à 24 entrées. C'est le comportement que `parse_data` annonçait déjà — « several vintages of one program are all offered — the picker lists them ». 228 Ko au total, mis en cache par le service worker.

## Alternatives rejetées

- **Un blob fusionné** (`make ui-data` concatène les snapshots en un seul `programmes.json` via `jq`) : une seule requête au lieu de 24, mais il faut re-sérialiser chaque entrée pour préserver `RawData.programs`, ou toucher `parse_data` et ses tests. Diff plus large pour un gain de latence négligeable sur 228 Ko.
- **Lire `data/programmes/index.json` à l'exécution** (le job `deploy` le génère déjà) : casse `dx serve`, qui ne sert que les fichiers passés par `asset!()`.
- **Générer un fichier source versionné** (`make ui-data` écrirait `crates/ui/src/programmes.rs`) : `cargo build` seul ne suffirait plus, et un fichier généré traînerait dans l'arbre. `build.rs` est le mécanisme prévu pour ça.
