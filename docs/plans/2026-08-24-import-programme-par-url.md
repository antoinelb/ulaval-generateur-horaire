# Import d'un programme depuis son URL ulaval.ca

## Goal
L'étudiant colle l'URL d'une page programme de www.ulaval.ca dans un tiroir repliable du `ProgramPicker` ; l'app récupère le HTML via le proxy CORS corsproxy.io, le parse dans le navigateur, sauvegarde le programme en localStorage et l'ajoute à la liste avec un badge « Ajouté localement ».

## Out of scope
Pas de fallback collage de HTML ni de chaîne de proxys : corsproxy.io seul, erreur claire en échec.
Pas de rafraîchissement d'un programme local : on supprime et on ré-importe.
Pas de `cheminement_type` pour un programme local (les programmes livrés sans `manuel.json` fonctionnent déjà ainsi).
Pas de changement à la surface JS du paquet wasm publié ni au dépôt JS sœur.
Pas d'import de cours : `data/cours.json` (8 834 cours) couvre déjà le catalogue entier.

## Constraints
Design retenu : D2 de l'artefact « Trois portes vers ulaval.ca » — tiroir « Votre programme n'est pas là ? » replié au repos, verrouillé pendant l'import, jamais refermé sur erreur — enrichi des phases nommées et cochées de D3 (téléchargement → analyse → enregistrement, secondes écoulées, bouton Annuler, jamais de spinner nu — LAT-4/5).
Erreurs en 5 parties ERR-1 (`UiError` existant), validation du champ au commit seulement (INP-7), jamais de vidage de champ (ERR-6).
Suppression destructive → toast « Programme supprimé — Annuler » (pattern ACT existant), jamais de dialogue de confirmation.
Provenance affichée sur la carte du programme local : URL source, date d'import absolue, mention du proxy (TRU) ; anomalies de parsing signalées sur la carte, jamais tues (« never drop unrecognized input silently »).
L'import est un chemin non critique (BLD-1) : l'indisponibilité du proxy ne casse rien, un programme déjà importé vit en localStorage et survit hors-ligne (DEG-3).
Toute la logique dans les modules purs (`state`, `data`, `persist`, `present`, `panel`), zéro logique dans `rsx!` (AP-5) ; IO navigateur confiné à `browser.rs` (AP-7) ; lire `.claude/dioxus.md` avant tout code Dioxus.
Le parseur migre entièrement dans `core` (cours et catalogue inclus, pas seulement programme) pour rester cohérent ; `scraper` 0.27 est pur Rust et compile en wasm32.
En cas de doublon `(code, semestre)` entre un programme livré et un local, le livré gagne et le local est signalé comme remplacé, jamais ignoré en silence.
Chaque décision de cette conversation reçoit son ADR : proxy CORS corsproxy.io, parseur dans core, programmes locaux en localStorage.

## Items
1. Migration du parseur : `crates/scraper/src/parser/{mod,catalogue,course,program}.rs` et `ParseError` déménagent dans `core` (dépendance `scraper = "0.27"` ajoutée à core), le crate scraper les ré-exporte ou les importe depuis core, les fixtures de `tests/fixtures/test_cases/` continuent de passer, et `make lint` (qui clippy le wasm32) confirme la compilation navigateur.
2. Millésime côté client : `semester_after` et son `civil_from_days` migrent de `crates/scraper/src/cli.rs` vers core, paramétrés par une date fournie par l'appelant, le scraper natif comme l'UI les appellent depuis là.
3. Fetch via proxy : `browser.rs` gagne un fetch d'URL externe via `https://corsproxy.io/?url=…` avec `gloo-net` (déjà en dépendance), erreurs typées distinguant panne du proxy, page introuvable et réponse non HTML, annulable.
4. Règle « Scolarité préparatoire » côté client : après le parsing, `core::preparatory_rule` (déjà pure) est appliquée avec le catalogue déjà chargé du `Snapshot`, comme `cli.rs::add_preparatory_rules` le fait au scrape.
5. Persistance : nouvelle clé `gh.v1.programmes-locaux` dans `persist.rs`, `Envelope` versionnée portant chaque programme avec ses métadonnées de provenance (URL source, date d'import) et ses anomalies, restauration bruyante-et-tolérante sur le modèle de `restore_manual`.
6. Fusion dans le snapshot : `data.rs::parse_data` accepte les programmes locaux (paramètre sur le modèle de `manual: Vec<Course>`), dédup par `(code, semestre)` où le livré gagne avec signalement, tri inchangé.
7. UI du tiroir : dans `ProgramPicker`, section repliable « Votre programme n'est pas là ? » avec champ URL (validation au commit : URL de page programme ulaval.ca), phases nommées et cochées avec secondes écoulées et bouton Annuler, erreur ERR-1 en 5 parties sous le formulaire, tiroir verrouillé pendant l'import et jamais refermé sur erreur.
8. Carte du programme local : badge textuel « Ajouté localement », provenance (URL, date absolue, mention du proxy), anomalies signalées, bouton Supprimer.
9. Suppression avec undo : retrait du programme local (localStorage + snapshot en mémoire), toast « Programme supprimé — Annuler » qui restaure tout, et retour au picker si le programme supprimé était le programme actif.
10. ADRs : `2026-08-import-de-programme-via-proxy-cors.md`, `2026-08-parseur-dans-core.md`, `2026-08-programmes-locaux-en-localstorage.md` sous `docs/conception/adr/`.
11. Tests et couverture : tests purs des nouveaux chemins (`persist`, `data`, `present`, `panel`, protocole d'import), fixtures HTML→JSON du parseur exercées depuis core, `make test` à 100 %.

## Acceptance
Dans l'app servie par `make ui`, coller l'URL d'un vrai programme absent du snapshot (p. ex. B-GLO) l'importe, il apparaît avec son badge, se planifie, et survit au rechargement de la page.
`make lint` sans warning et `make test` à 100 % de couverture.
Les trois ADR sont écrits.

## Check
`make lint && make test`
