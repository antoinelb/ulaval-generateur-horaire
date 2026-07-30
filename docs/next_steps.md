# Plan — le cœur solveur

~~**Avant toute chose, faire en sorte que rouler `programs` sans url rafraîchit l'ensemble des programmes déjà dans `data`.**~~ — fait (ADR `2026-07-programs-sans-url-rafraichit-par-slug`, `2026-07-annee-de-programme-selon-la-date-de-scrape` : les snapshots sont désormais `{code}-{year}.json`).

Étape 2 (« Cœur ») de l'ordre de construction de `docs/project_plan.md` — l'étape 1 (scraper) est livrée, son plan test-first vit dans l'historique git de ce fichier.
Fondements, mathématiques et justifications complètes dans `docs/conception/solveur-conception.md` — **le lire avant d'écrire du code**.
Ce plan-ci porte le *quoi faire*, dans quel ordre ; le doc de conception porte le *pourquoi*.

Deux solveurs distincts qui partagent un squelette : **A** (horaire hebdomadaire, jalon 2) et **B** (organigramme, jalons 7–9).
Les deux sont faits main (ADR `2026-07-b-placement-par-satisfaction-fait-main` : placement seul, pas d'optimisation ; l'embranchement Pumpkin est fermé, Pumpkin reste le repli documenté en conception §5.2–§6) et B retourne **toutes** les solutions faisables, bornées par le budget de nœuds (ADR `2026-07-b-enumere-toutes-les-solutions`).
**B place une liste de cours donnée — il ne choisit jamais de cours** : l'étudiant (ou le directeur pour une base générale) fournit la liste, possiblement partielle ; la couverture des règles et la validation de la sélection sont une fonction pure séparée de `core` consommée par l'UI.

**Conventions (rappel `CLAUDE.md`).** Test-first : écrire le test qui échoue, puis l'implémentation jusqu'au vert.
`make test` (couverture `cargo +nightly llvm-cov`, cible 100 % hors `lib.rs`/`mod.rs`/`main.rs`) et `make static` (fmt + clippy `-D warnings`) verts à chaque tâche.
**Ni boucle `while` ni récursion** (itérateurs, `fold`).
**Éviter `expect` en production.**
Le code est en anglais, le domaine en français dans la prose.
Toute décision prise en cours de route = un ADR individuel sous `docs/conception/adr/`, jamais laissée dans la conversation seule.
L'absence de perte silencieuse s'applique partout : une règle, un préalable ou une opérande hors grammaire est remonté, jamais ignoré.

**Dépendances nouvelles attendues.** `core` : aucune ; `proptest` en `dev-dependencies` pour les tests de propriété (liberté de conflit).

---

## Phase 0 — Fondations partagées

- [x] `week.rs` : encodage du temps
    - [x] `WeekMask([u64; 32])` — semaine à seaux de 5 min (7 × 288 = 2016 bits ; 5 min nécessaires et suffisants, mesuré sur les 24 042 bornes de tout `data/cours/` — ADR `2026-07-encodage-semaine-en-seaux-de-5-minutes`) ; `overlaps` (ET mot à mot), `merge` (OU mot à mot), `is_empty`
    - [x] `slots_to_mask(&[Slot]) -> WeekMask` : `Day` + `Time` → index de seau ; une `Section` sans plage (à distance) donne le masque vide
    - Opérations de préférence sur bits (`before_noon_free`, `has_midday_gap`, `day_transitions`) : **reportées au jalon 10** — rien d'écrit, la sémantique se calibre avec le classement de A (ADR `2026-07-preferences-de-a-reportees-au-jalon-10`)
    - Verify : ✅ tests unitaires sur des plages réelles (GCI-1007) ; propriétés `proptest` : `overlaps` symétrique, `merge` associatif/commutatif, plages sans horaire = masque vide

- [x] Construction du domaine de A (dans `weekly.rs`, fonction pure)
    - [x] `build_domain(offering) -> Vec<Opt>` où `Opt = { nrc_set, mask }` : une entrée par `options[i]`, masque = union de ses sections ; prend l'offre déjà résolue
    - [x] Équivalences : `resolve_offering` retient l'offre au millésime de session le plus récent, année fournie par l'appelant, égalité → le cours (ADR `2026-07-equivalences-par-millesime-de-session`)
    - [x] Section forcée : `force_nrc(domain, nrc)` restreint aux options dont l'ensemble de sections **contient** le NRC (jamais « l'option k » — un NRC peut être dans plusieurs options ; cf. test `one_nrc_may_appear_in_several_options`)
    - [x] Crédits en intervalle : `Credits::resolve(chosen)` — la pondération choisie par l'étudiant entre en paramètre, séparée du domaine (ADR `2026-07-resolution-des-credits-choisis`)
    - Verify : ✅ tests sur GCI-1007 (multi-options réel) ; forcer le NRC 13449 partagé de CSO-6702 en garde deux ; forcer un NRC absent vide le domaine

---

## Phase 1 — Solveur A (jalon 2, arrêté, fait main)

- [x] `weekly.rs` : la recherche
    - [x] `enumerate` : produit incrémental élagué par `fold` sur les cours (ni `while` ni récursion), élagage par `overlaps`, collecte **toutes** les feuilles valides (le classement en a besoin) — feuilles en indices d'option, l'ordre du snapshot que `Opt.nrc_set` perd
    - [x] `is_feasible(&[Vec<Opt>]) -> bool` (le veto pour B) — **court-circuite** (`try_fold`, arrêt dès que les préfixes se vident), sans payer la collecte complète — et `best_schedule(&[Vec<Opt>]) -> Option<Schedule>` (le « premier horaire faisable » du contrat ; `Score` naît au jalon 10 — ADR `2026-07-score-de-a-reporte-au-jalon-10`)
    - [x] `Schedule` (ensemble de NRC choisis, partageable en URL plus tard)
    - Verify : ✅ propriétés `proptest` — toute feuille d'`enumerate` est sans conflit ; `is_feasible` ⇔ `best_schedule.is_some()` ⇔ énumération non vide ; ajouter un cours ne peut jamais rendre faisable un ensemble infaisable

- [x] Rapport de conflit (cas infaisable)
    - [x] `schedule_report(courses, season, chosen)` — la fonction pure du contrat UI gelé : sélection « premier faisable, épinglés gardés », marquage `valid: false` par cours et par alternative (sémantique swap) ; le raffinement Max-CSP « ensemble minimal » reste ouvert (doc §1.1 et §7)
    - Verify : ✅ les 18 fixtures `tests/fixtures/test_cases/schedules/` reproduites à l'identique (`crates/core/tests/integration/schedule.rs`), **cas piège** `triple-infeasible-pairwise-ok` inclus ; les entrées hors contrat (cours non offert, zéro option, épinglage inconnu ou sans option) sont des `ScheduleError` typées, jamais inventées

- [x] Harnais CLI (livrable du jalon 2)
    - [x] `crates/cli` (binaire `ulaval-scheduler`, ADR `2026-07-harnais-cli-en-crate-dedie`) : imprime l'horaire d'une liste de codes d'une session (`anyhow` à la frontière binaire), équivalences résolues par `resolve_offering`, total de crédits, sortie 2 + cours fautifs si conflit
    - Verify : ✅ `make test` vert (100 %) ; `ulaval-scheduler schedule a2026 GEX-1002 GEX-2003 GCI-1007` imprime un horaire sans conflit (GCI-1007 bascule visiblement sur son option B du vendredi) ; liberté de conflit testée par propriétés

---

## Phase 2 — Substrat de B

- [ ] Modèle du domaine de B (`organigramme.rs`)
    - [ ] Entrée = **liste de cours fournie** (possiblement partielle — tronc seul, ou base du directeur), sessions `{1,…,8}`, saison de chaque session, crédits, plafond ; B ne dérive jamais de candidats depuis les règles
    - [ ] Contraintes de l'utilisateur → **réductions de domaine à des singletons** : cours réussis (retirés, crédits précomptés), cours voulus (forcés — p. ex. avec un ami), sessions remplies à la main, session à l'étranger (puits de crédits). Un seul mécanisme pour « à partir de zéro » *et* « avec cours fixés »
    - Verify : un cours épinglé a un domaine singleton ; un cours réussi ne figure plus dans les candidats mais compte dans les crédits et les préalables ; une liste partielle se place sans erreur

- [ ] Vérificateur de règles (`rules.rs` — l'API produit, cœur pur, consommée par l'UI ; jalon 8)
    - [ ] Couverture d'une sélection : par règle, **satisfait / à combler / candidats** ; `Constraint::Count{n}` exige `n` choisis, `Constraint::Credits{min,max}` une somme dans l'intervalle ; obligatoires manquants signalés
    - [ ] Candidats pour combler une règle = sa liste de cours, filtrée par `weekly::is_feasible` contre l'horaire ouvert de la session visée (A suffit — aucun cheminement à vérifier)
    - [ ] `RuleCourses::{Reference, Keyword, Raw}` (`Keyword::{Any, Negotiated}`) et toute règle `constraint: None` **remontées à l'étudiant, jamais inventées** (`2026-07-contrainte-de-regle-optionnelle`, `2026-07-regles-negociees-reconnues`) ; la règle 5 GEX est `Keyword::Any` (domaine = tout le catalogue) → choix de l'étudiant, jamais énumérée
    - Verify : sur les règles réelles du bac GEX (1 parmi 3 ; 3–9 cr parmi 4 ; 3–9 cr parmi 9 ; 3 cr parmi 19 ; règle 5 `Keyword::Any` remontée), le rapport accepte des sélections conformes fabriquées et signale les non conformes avec la règle fautive ; une règle `constraint: None` (fixture synthétique — aucune dans GEX) produit une note, pas un verdict

- [ ] Filtres structurels (chacun en O(cours), à appliquer dans cet ordre)
    - [ ] Offre par saison ; précédence par parcours du `PrereqTree` (`All` = chaque enfant strictement avant ; `Any` = au moins un avant ; `ProgramCredits` = crédits accumulés avant ≥ seuil ; opérandes `Raw`/non vérifiables **remontées, jamais imposées** — `2026-07-operande-non-verifiable-gardee-en-texte`) ; capacité de crédits par session
    - [ ] Option « concomitants » : relâche « strictement avant » en « avant ou identique »
    - Verify : un cours placé avant son préalable est rejeté ; un arbre OU satisfait par un seul enfant passe ; une saison non offerte est rejetée ; un dépassement de plafond est rejeté

- [ ] Oracle de faisabilité mémoïsé (`feasibility.rs`)
    - [ ] `term_feasible(cache, (Season, BTreeSet<code>), snapshot) -> bool` appelant `weekly::is_feasible` sur le domaine construit, clé canonique indépendante de l'ordre
    - Verify : deux candidats partageant l'ensemble de cours d'une session ne calculent la faisabilité qu'une fois (compteur d'appels sous-jacents)

---

## Phase 3 — Solveur B (fait main, placement seul, toutes les solutions)

- [ ] `organigramme.rs` : la recherche de placement
    - [ ] Affectation systématique **complète** cours par cours (frontière par `fold`, ni `while` ni récursion), élaguée par les filtres structurels à chaque extension
    - [ ] Ordre de valeurs = session du `cheminement_type` de référence d'abord, puis sessions voisines ; sans seed (autre programme, base du directeur) : plus tôt offerte d'abord — la première solution de la liste ressemble au cheminement de référence
    - [ ] A-veto mémoïsé (`term_feasible`) dès qu'une session est complète, pas seulement aux feuilles
    - [ ] Budget de nœuds explicite, qui borne aussi la taille de l'ensemble retourné (mémoire WASM) : recherche épuisée sans solution = « aucun cheminement faisable » **prouvé** ; budget atteint = « budget épuisé », jamais confondus
    - [ ] **Rejet, jamais réparation** : un préfixe qui échoue est abandonné, jamais corrigé sur place
    - [ ] Sortie : **toutes les solutions faisables trouvées**, dans l'ordre de la recherche (ADR `2026-07-b-enumere-toutes-les-solutions`) — recherche épuisée = ensemble complet prouvé (vide = infaisable) ; budget atteint = ensemble partiel, signalé ; l'utilisateur édite, on replace
    - Verify : sur le bac GEX complet (34 obligatoires + électifs choisis), l'énumération complète sort en bien moins d'une seconde ; **mesure consignée** : le nombre de solutions du bac complet *et* d'une liste partielle (tronc seul) — tout raffinement (dédoublonnage d'électifs interchangeables) attend cette donnée ; propriétés — chaque solution de l'ensemble respecte précédence, offre, capacité, singletons épinglés, et chaque session est horaire-faisable via A ; sur entrée sur-contrainte fabriquée sans solution, sortie « infaisable » prouvée (pas de plantage, pas de faux positif) ; un budget minuscule forcé rapporte « budget épuisé » avec ensemble partiel, jamais « infaisable »

---

## Phase 4 — Intégration et harnais

- [ ] Harnais CLI/test de B
    - [ ] Fournir des contraintes (réussis, voulus, sessions manuelles, étranger) et une liste de cours → le premier organigramme imprimé en entier, le compte total de solutions affiché ; avec une liste partielle, le rapport de couverture des règles imprimé à côté
    - Verify : `make test` vert à couverture cible ; un organigramme signalant un cours placé avant son préalable et ce qui manque pour diplômer (jalon 8, via le vérificateur)

- [ ] Tests de propriété transverses de B
    - Verify : sur des contraintes et listes générées, toute solution de l'ensemble retourné respecte précédence, offre, capacité, et chaque session est horaire-faisable via A ; le vérificateur de règles est cohérent (une sélection déclarée conforme ne perd aucune règle)

- [ ] Câblage `ui` (jalon 7+ ; hors cœur) : `core` piloté depuis Dioxus, snapshot chargé au navigateur — **aucune règle métier dans la vue** (invariant) : couverture des règles, candidats et validation viennent tous de `core`. Lire `.claude/dioxus.md` avant tout code Dioxus 0.7

---

## Encore à planifier (à faire remonter, pas à inventer)

Ces points sont des **décisions ou des données manquantes**, pas des tâches d'implémentation ; les trancher avec l'utilisateur et les consigner en ADR avant de coder ce qui en dépend.

- **Plafond de crédits par session** : dur (17 ?) ou cible molle — le chiffre n'a aucune source documentée, à confirmer avec le directeur.
- **Pondération des crédits en intervalle** : requise d'emblée, ou défaut à la borne basse.
- **Sémantique exacte des préférences de A** (journées compactes, matins libres, pause dîner) — pour le classement du jalon 10, à calibrer contre des données réelles (Phase 0 laisse les signatures ouvertes) ; B n'a plus d'objectif.
- **Interaction règles × profils** (jalon 6) — désormais côté vérificateur/affichage, plus côté solveur.
- **Format JSON de l'organigramme** échangé entre « Cours pour le programme » et l'horaire hebdomadaire — question ouverte du plan, provisoire.
- **Forme minimale du rapport de conflit** de A — tranchée au niveau cours : marquage optionnel `valid: false` par cours et par alternative (sémantique swap), contrat et cas de test figés dans `tests/fixtures/test_cases/schedules/` (ADR `2026-07-contrat-horaire-hebdomadaire-vers-ui`) ; reste ouvert le raffinement « ensemble minimal » Max-CSP pour le cas infaisable (le cas « paires compatibles, ensemble infaisable » est couvert par `triple-infeasible-pairwise-ok.json`).
- **Présentation des solutions multiples de B dans l'UI** : la première est proposée ; comment (et si) offrir les autres — et si la mesure révèle une explosion de variantes interchangeables, la forme du dédoublonnage.
