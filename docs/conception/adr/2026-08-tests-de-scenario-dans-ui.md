# Des tests de scénario dans `crates/ui`

Date : 2026-08-30

**Statut :** accepté. **Complète** `2026-07-structure-des-tests-et-fixtures` (une cible de test de plus, et où elle s'arrête).

## Contexte

Les modules purs de `crates/ui` — `state`, `solve`, `persist`, `present`, `capsule`, `panel`, `alerts`, `data`, `import`, `export` — sont à 100 % de couverture, lignes et régions.
Chaque brique est donc prouvée **seule**.
Aucun test n'en enchaînait deux.

L'enchaînement, lui, vit dans `crates/ui/src/components/mod.rs` : `restore_state`, `import_organigramme`, `apply_proposal`, `edit_plan`, `save_on_change`, `track_plan_change`, `auto_propose`, `auto_verify`.
Ce fichier est `#[cfg(target_arch = "wasm32")]` et le `makefile` l'exclut explicitement du calcul de couverture (`--ignore-filename-regex … crates/ui/src/components/`).
Il n'est ni exécuté ni mesuré par `make test`.

C'est précisément là que se cachaient les défauts rapportés par les personas le 2026-08-30, et ceux d'avant :

- l'épingle acceptée sans un mot (`2026-08-une-epingle-est-verifiee-comme-le-reste`, points 1 et 5) ;
- le verdict `verify` périmé accepté comme frais, qui figeait « Placement vérifié ✓ » sur une grille que personne n'avait jugée ;
- le siège hors horizon d'une vieille sauvegarde, qui faisait refuser tout le plan à chaque rechargement (2026-08-26) ;
- la proposition en vol adoptée par-dessus un import, qui délogeait les épingles et faisait mourir la requête suivante sur « pinned but has no Course ».

Aucun de ces défauts n'est visible en regardant une fonction. Tous le sont en regardant une **suite** de gestes.

Rien n'empêchait pourtant de rejouer cette suite nativement : toutes les portes prennent leur monde en argument plutôt que de l'aller chercher.
`persist::restore_plan` reçoit un `Option<&str>` — la frontière `localStorage` est en dehors.
`data::parse_data` reçoit des `String`, pas des chemins.
`solve::place_request` produit la requête JSON exacte, `ulaval_scheduler_wasm::protocol::handle` (gardé par aucun `cfg`) la traite, `solve::parse_worker_answer` la relit : la boucle complète du solveur tourne en natif, sans navigateur ni Web Worker.
`state::apply`/`undo` sont l'unique porte de mutation. `capsule::load` lit la fixture partagée `tests/fixtures/test_cases/transcripts/exemple.html`.

## Décision

**Une cible de test `scenarios` dans `crates/ui`**, déclarée dans `Cargo.toml` parce que ses fichiers vivent dans un sous-répertoire :

```toml
[[test]]
name = "scenarios"
path = "tests/scenarios/main.rs"
```

`main.rs` ne fait que déclarer ses modules. Aucune `[dev-dependencies]` : une cible de test lie déjà les `[dependencies]` de la caisse.

`tests/scenarios/harness.rs` rejoue la chaîne de `components/mod.rs` dans son ordre propre — une `BTreeMap` là où le navigateur a `localStorage`, un appel direct à `protocol::handle` là où il a un Web Worker.
**Le harnais ne décide rien** : chaque verdict vient de `state`, `solve`, `persist`, `capsule`, `panel` ou du solveur lui-même.
Chaque scénario **part d'un état existant** — une sauvegarde, un lien de partage, un relevé, une étagère — jamais de `Plan::default()`.

Quinze scénarios, nommés à l'indicatif comme le reste du dépôt :

| module | ce qu'il enchaîne |
| --- | --- |
| `reload` | sauvegarde → restauration → épinglage → solveur → adoption → sauvegarde → restauration ; guérison d'un siège hors horizon ; chaque siège restauré porte son `Course` dans la requête suivante |
| `share` | lien ouvert par-dessus un document existant : étagère, gel intégral, une « Annuler », et un organigramme modifié qui cesse d'encoder le lien reçu |
| `transcript` | relevé Capsule : « Début » ancré dans le passé, sessions notées gelées, horizon grandi, solve subséquent qui respecte les acquis ; une seule « Annuler » pour tout l'import |
| `documents` | l'étagère par (programme, millésime), aller-retour, et le verdict qui ne survit pas à la bascule |
| `start` | « Début » sous l'horloge : épingler dans une session vécue et voir le solveur en assumer les préalables ; l'aller-retour de « Début » **documenté** comme délibéré |
| `verdict` | la course entre l'envoi et la réponse : verdict périmé, proposition en vol adoptée après un import |

**Le coût en couverture est nul.** Le `makefile` ne passe pas `--include-tests` à `cargo llvm-cov` : les fichiers sous `tests/` ne sont pas instrumentés. Vérifié : `make test` reste à 100,00 % lignes et régions, sur le même total de régions qu'avant (17 632).

### La frontière avec les tests navigateur

Ces scénarios s'arrêtent **au bord de la vue**. Ils ne prouvent rien de ce que `components/` fait du résultat : pas un rendu, pas un clic, pas un glisser-déposer, pas un `use_effect`, pas la temporisation de 500 ms, pas le vrai `localStorage`, pas le vrai Web Worker.
C'est le domaine des tests Playwright (`tests/e2e/`), montés en parallèle, et cette cible ne les remplace pas.

**Ce que ces scénarios n'attrapent pas, et qu'il faut dire** : le harnais est un *miroir* de `components/mod.rs`, pas `components/mod.rs`. Une règle qui quitterait un module pur pour la vue — ou un appel que la vue cesserait de faire — laisserait ces tests verts. Ils gèlent la composition des décisions, pas leur branchement. Le branchement reste la charge des tests navigateur et de la relecture.

## Alternatives rejetées

- **Rendre `components/mod.rs` testable nativement** (extraire l'orchestration dans un module pur, ou compiler la vue hors wasm). C'est la vraie correction, et elle est bien plus grosse que ce mandat : elle touche l'architecture du fichier le plus dense de l'interface, sans qu'aucun test n'existe encore pour la garder. On écrit d'abord le filet.
- **Mettre ces scénarios en tests inline dans les modules purs.** Un scénario traverse `persist`, `state`, `solve` et `capsule` : le loger dans l'un d'eux mentirait sur son sujet, et le ferait compter dans la couverture d'un fichier dont il ne prouve pas les lignes (`2026-07-couverture-par-instanciation-le-plus-petit-ecart`).
- **Tout confier à Playwright.** Un aller-retour de solveur y coûte des secondes et une infrastructure ; ici il coûte 2 s pour les quinze. Et un test navigateur qui échoue ne dit pas *quelle* décision a bougé.
- **Une caisse `tests/` séparée dans l'atelier.** Elle n'aurait pas accès aux modules de `ui` autrement qu'en les rendant publics deux fois ; la cible de test de la caisse elle-même les lie déjà.
- **Charger les 24 instantanés de `data/programmes/`** dans le harnais. Deux suffisent (B-GEX pour le cas courant, B-GCI pour le rapport du directeur) ; le reste n'ajoute que du bruit au sélecteur et du temps d'analyse.
