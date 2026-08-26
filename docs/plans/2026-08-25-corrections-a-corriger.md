# Corrections de a_corriger.md : sessions par défaut, import JSON de programme, import capsule

## Goal
Les trois corrections de `a_corriger.md` sont livrées : le nombre de sessions par défaut suit les crédits du programme, un programme millésimé se charge depuis un fichier JSON, et un relevé Capsule collé remplit l'organigramme (cours passés épinglés, acquis crédités, session de début ajustée).

## Out of scope
- Le jalon 10 (préférences).
- Toute modification du scraper ou des snapshots `data/`.
- Le dépôt JS `../grille-de-cheminement-interactive` (il évolue en parallèle).
- L'export d'un programme en JSON (seul le chargement est demandé).

## Constraints
- Code, identifiants et clés JSON en anglais ; prose et textes affichés en français (sans espace avant ?, ! ou ;).
- Le parsing du relevé Capsule est de la logique métier : il vit dans `core` (module `parser`, comme le parseur de pages ULaval), jamais dans la vue.
- Ne jamais avaler silencieusement une entrée non reconnue : toute ligne, note ou section inattendue du relevé ou du JSON est rapportée à l'utilisateur.
- Avant tout code d'interface, lire et appliquer le skill `air` ; avant tout code Dioxus, lire `.claude/dioxus.md`.
- Pas de boucles `while`, pas de `expect` en production, pas de récursion non bornée.
- Chaque décision prise reçoit son ADR dans `docs/conception/adr/`.
- `make test` doit finir à 100 % de couverture.
- L'échantillon de relevé est `exemple_capsule.html` à la racine du dépôt (non versionné) : en tirer une fixture anonymisée et committable, ne jamais committer le fichier brut tel quel sans vérifier qu'il est anonymisé.

## Items
1. Sessions par défaut selon les crédits : à l'ouverture d'un programme, `study_sessions` vaut `credits_required / 15` arrondi vers le haut (90 → 6, 120 → 8) au lieu de la constante `DEFAULT_STUDY_SESSIONS = 8` de `crates/ui/src/state.rs` ; trouver l'endroit où les faits du programme ouvrent le plan (ADR `2026-08-reglages-transversaux-dans-linstantane`) et y brancher la formule, qui reste un défaut modifiable par l'étudiant.
2. Parseur de relevé Capsule dans `core::parser` : depuis le HTML collé, extraire les trois sections — « CRÉDITS DE L'UNIVERSITÉ LAVAL » (blocs par session, en-tête `span.fieldOrangetextbold` du type « Automne 2024 », lignes sigle `XX(X{0,2})-#### `/cycle/titre/note/crédits), « RECONNAISSANCE DES ACQUIS » (cours validés « V » d'un autre établissement), « CRÉDITS EN COURS » (sessions courantes et futures, sans colonne note) — en une structure typée par session, avec les lignes non reconnues rapportées, jamais ignorées.
3. Fixture de relevé : une copie anonymisée et réduite d'`exemple_capsule.html` sous `tests/fixtures/test_cases/` (nouvelle famille, par exemple `transcripts/`), avec son JSON attendu, pour tester le parseur contre du HTML gelé comme le font les autres familles.
4. Application du relevé au plan : notes de passage (D et mieux, P) et cours en cours ou inscrits → `pinned_sessions` à leur session réelle ; échecs ignorés ; « RECONNAISSANCE DES ACQUIS » → `Plan.credited` sans influencer la session de début ; `Plan.start` devient la plus ancienne session ULaval du relevé ; un été présent au relevé ouvre `summers_open` ; l'horizon (`study_sessions`) s'étend si les sessions du relevé le débordent.
5. Bouton « Charger depuis Capsule » dans le plan ouvert : un modal avec zone de collage et le mode d'emploi (ctrl-u pour la source de la page du relevé, ctrl-a puis ctrl-c pour tout copier), qui parse, applique au plan comme un geste annulable, et affiche le bilan — cours placés, crédités, ignorés et lignes non reconnues.
6. Bouton « Charger depuis un fichier JSON » dans l'écran de choix de programme, à côté de l'import par URL : sélection d'un fichier `core::Program` (l'instantané `{code}-{semestre}.json`) et, optionnellement, de son `.manuel.json` (le `cheminement_type`) ; le programme rejoint `gh.v1.programmes-locaux` comme un import URL, avec une provenance « fichier local » à la place de l'URL source.
7. Validation du JSON chargé : un fichier qui ne désérialise pas en `Program` (ou un manuel sans `cheminement_type`) est refusé avec un message clair, jamais un plan à moitié chargé.
8. ADRs des trois décisions : sessions par défaut dérivées des crédits, import de programme par fichier JSON, import de relevé Capsule (format reconnu, sémantique passé/en cours/acquis, ajustement de la session de début).

## Acceptance
- Ouvrir le B-GEX (120 cr) propose 8 sessions ; un programme de 90 cr en propose 6 ; le réglage reste modifiable.
- Coller `exemple_capsule.html` dans le modal Capsule épingle les cours réussis et en cours à leurs sessions réelles, met MAT-1910 dans les crédités, règle le début à A24 et affiche le bilan.
- Charger un `{code}-{semestre}.json` (avec ou sans son `.manuel.json`) rend le programme disponible comme un programme importé par URL.
- Un JSON invalide ou un HTML sans relevé est refusé avec un message explicite.
- `make lint && make test` passe, couverture à 100 %.

## Check
`make lint && make test`
