# Le parseur déménage dans `core`, derrière la feature `parser`

## Contexte

L'import d'un programme par URL tourne dans le navigateur : coller une adresse ulaval.ca, récupérer le HTML via un proxy, et en tirer un `core::Program` sans jamais repasser par un serveur.
Le parseur (`parser/{mod, catalogue, course, program}.rs` et `ParseError`) vivait dans `crates/scraper`, un binaire natif async qui traîne `reqwest`, `tokio` et `clap` — aucun de ces trois ne compile en `wasm32-unknown-unknown`.
Le faire vivre là où l'UI peut l'appeler exigeait donc de le déplacer, pas de le dupliquer : `scraper = "0.27.0"` est pur Rust et compile en wasm32, seule sa dépendance en amont (`ulaval-scheduler-scraper` au complet) posait problème.

## Décision

- Le parseur et `ParseError` déménagent dans `core`, derrière la feature `parser`, **activée par défaut** — `crates/core/Cargo.toml` déclare `default = ["parser"]` et `scraper = { version = "0.27.0", optional = true }` ; oublier le flag reste donc impossible à faire taire silencieusement.
- `crates/scraper` (le crate) perd sa propre dépendance `scraper = "0.27"` et ré-exporte celle de `core` : `pub use ulaval_scheduler_core::parser;` dans `crates/scraper/src/lib.rs`. Le binaire natif ne change ni de comportement ni d'API publique.
- `crates/wasm` prend `core` en `default-features = false` (`ulaval-scheduler-core = { path = "../core", default-features = false }`) : le paquet npm et le worker `calc.wasm` publiés n'embarquent ni `html5ever` ni le reste de l'arbre de `scraper` — seule l'UI Dioxus, qui a réellement besoin du parseur pour l'import, l'active.
- `semester_after` et son `civil_from_days` suivent le même chemin, de `crates/scraper/src/cli.rs` vers `core::program` : paramétrés par des secondes écoulées depuis l'epoch fournies par l'appelant, jamais lues en interne — `std::time::SystemTime::now()` panique sur `wasm32-unknown-unknown`. Le scraper natif calcule ces secondes lui-même (`cli.rs::now_secs`, plancher à 0 sur une horloge antérieure à 1970) ; l'UI les tire de l'horloge du navigateur (`browser::now_secs`).
- Les fixtures HTML→JSON du parseur (`tests/fixtures/test_cases/{courses,programs}/*.{html,json}`) sont désormais exercées depuis `crates/core/tests/integration/parser_{catalogue,course,program}.rs`, à la place des tests équivalents de `crates/scraper`.
- La compilation `wasm32` du parseur est vérifiée par le clippy wasm32 de `make lint` — pas seulement par la compilation de `crates/ui`, qui pourrait masquer une régression du feature-gating si `ui` cessait un jour de dépendre de `core::parser` directement.

## Alternatives rejetées

- **Faire dépendre `ui`/`wasm` du crate `ulaval-scheduler-scraper`** — il traîne `reqwest`, `tokio` et `clap`, tous absents de la cible wasm32 ; le crate entier ne compilerait pas pour le navigateur.
- **Laisser le parseur dans `scraper` et coller le HTML à la main dans l'UI** — hors périmètre du plan (item 1), et cela dupliquerait la grammaire des programmes dans deux endroits qui devraient rester synchronisés à chaque évolution du répertoire.
