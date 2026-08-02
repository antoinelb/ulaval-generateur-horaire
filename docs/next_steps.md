# Plan — le cœur solveur

~~**Avant toute chose, faire en sorte que rouler `programs` sans url rafraîchit l'ensemble des programmes déjà dans `data`.**~~ — fait (ADR `2026-07-programs-sans-url-rafraichit-par-slug`, `2026-07-annee-de-programme-selon-la-date-de-scrape` : les snapshots sont désormais `{code}-{year}.json` ; depuis, le millésime est un semestre `{code}-A26.json` — ADR `2026-08-millesime-de-programme-en-semestre`).

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

- [x] Modèle du domaine de B (`organigramme.rs`)
    - [x] Entrée = **liste de cours fournie** (`PlacementRequest` : sessions en saisons ordonnées, plafond, concomitance, réussis, épinglés, seed d'ordre) ; B ne dérive jamais de candidats depuis les règles
    - [x] Contraintes de l'utilisateur → **réductions de domaine à des singletons** : réussis retirés et précomptés, épinglés en singleton, entrées contradictoires ou orphelines en `PlacementError` typées
    - Verify : ✅ tests en ligne (épinglé = singleton ; réussi absent du placement, compté aux crédits et aux préalables ; liste partielle sans erreur ; tout-réussi = une solution vide)

- [x] Vérificateur de règles (`rules.rs` — l'API produit, cœur pur, consommée par l'UI ; jalon 8)
    - [x] Couverture d'une sélection : par portée (programme + concentration/profil choisis), obligatoires satisfaits/manquants, règles `satisfied`/`incomplete`/`reported` avec `missing` miroir de `Constraint` ; somme > `max` = erreur typée (ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee`) ; crédits `Range` à la borne basse (ADR `2026-07-credits-range-borne-basse-en-planification`)
    - [x] `RuleCourses::{Reference, Keyword, Raw}` et toute règle `constraint: None` **remontées, jamais inventées** ; `Reference` résolue avant le verdict (une référence brisée reste une erreur) ; candidats **non filtrés** par faisabilité (couche comptable — la composition avec A viendra avec sa forme d'entrée)
    - Verify : ✅ les 14 fixtures gelées de `tests/fixtures/test_cases/rules/` reproduites à l'identique (`crates/core/tests/integration/rules.rs`), tous les chemins d'erreur épinglés en tests unitaires

- [x] Filtres structurels (appliqués à chaque extension, le moins cher d'abord)
    - [x] Offre par saison (au domaine) ; capacité ; précédence par évaluation **trois-valuée** de l'arbre aplati (sans récursion) — `False` prouvé permanent donc élagage sûr ; opérandes `Raw` **et codes inconnus préuniversitaires (0xxx)** présumés satisfaits et remontés par solution (`assumed`) ; tout autre code inconnu bloque (ADR `2026-07-prealable-inconnu-non-bloquant-remonte`, restreint par `2026-07-presomption-limitee-au-preuniversitaire`)
    - [x] Option « concomitants » : relâche « strictement avant » en « avant ou identique » (jamais pour soi-même ni pour `program_credits`)
    - Verify : ✅ chaque filtre épinglé par les fixtures `organigrammes/` et par tests en ligne (chaîne de préalables, OU à une branche, saison absente, dépassement de plafond)

- [x] Oracle de faisabilité mémoïsé (`feasibility.rs`)
    - [x] `FeasibilityCache::term_feasible(season, codes, by_code)` appelant `weekly::is_feasible`, clé `(Season, BTreeSet<code>)` canonique — appliqué au contenu **partiel** de la session à chaque ajout (`is_feasible` est monotone, le veto anticipé est déjà final)
    - Verify : ✅ le même ensemble ne calcule qu'une fois (`computed()`), saisons distinctes = verdicts distincts, code sans offre = infaisable bruyant

---

## Phase 3 — Solveur B (fait main, placement seul, toutes les solutions)

- [x] `organigramme.rs` : la recherche de placement
    - [x] Affectation systématique **complète** cours par cours — parcours en profondeur sur pile explicite dans un `try_fold` borné (ni `while` ni récursion ; la profondeur préserve les solutions déjà trouvées quand une borne arrête la recherche, ce que la frontière de A perdrait — ADR `2026-07-budget-de-b-en-double-borne`), élaguée par les filtres structurels à chaque extension
    - [x] Ordre de valeurs = session du seed d'abord, puis voisines par distance (plus tôt sur égalité) ; sans seed : plus tôt offerte d'abord — le paramètre `seed` attend le `cheminement_type` manuel (fichier encore inexistant)
    - [x] A-veto mémoïsé (`term_feasible`) sur le contenu partiel de la session à **chaque ajout** (monotonie de `is_feasible`), pas seulement aux feuilles
    - [x] **Double borne** explicite (`max_nodes` = affectations partielles développées, `max_solutions` = taille de l'ensemble) ; `Completion::{Complete, NodeBudget, SolutionCap}` jamais confondus — recherche épuisée à ensemble vide = infaisabilité **prouvée**
    - [x] **Rejet, jamais réparation** ; sortie = toutes les solutions trouvées en ordre de recherche, `assumed` par solution pour les opérandes présumées
    - Verify : ✅ les 13 fixtures gelées de `tests/fixtures/test_cases/organigrammes/` reproduites à l'identique, sortie canonisée avant comparaison — dont l'inversion des projets intégrateurs en admission hiver ; propriétés `proptest` : l'ensemble retourné **égale** une énumération brute indépendante (justesse *et* complétude), recherche déterministe, budget réduit = préfixe de l'ensemble complet ; budget minuscule = `NodeBudget` jamais « infaisable » ; sur-contraint = infaisable prouvé
    - **Mesure consignée (2026-07-30, release, plafond 17, admission a2026)** : bac GEX complet (33 obligatoires plaçables + 4 électifs, GCI-1011 sans page de cours — voir « Encore à planifier ») → première solution < 50 ms, 1 000 solutions en 36 ms ; tronc seul (33 cours) → 100 000 solutions en 1,4 s, 500 000 en 7,9 s, **plafond toujours atteint** : l'ensemble complet explose combinatoirement (queue de cours lâches), « bien moins d'une seconde » vaut pour tout ensemble borné raisonnable mais **pas** pour l'énumération totale — la donnée que le dédoublonnage d'électifs interchangeables attendait ; la double borne fait exactement son travail

---

## Phase 4 — Intégration et harnais

- [x] Harnais CLI/test de B
    - [x] `ulaval-scheduler organigramme <session-départ> [codes…] --credit-cap N [--program STEM] [--passed …] [--pinned CODE=N] [--concomitant] [--sessions N] [--max-nodes N] [--max-solutions N]` : saisons alternées automne/hiver depuis la session de départ, `data/cours.json` unique (offre la plus récente par saison portée par `last_offered` — hypothèse fondatrice par cours), premier organigramme imprimé en entier avec charges par session, compte et statut de l'ensemble, préalables présumés remontés ; avec `--program`, les obligatoires entrent dans la liste et le rapport de couverture est imprimé à côté ; un obligatoire sans page de cours est écarté bruyamment (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`), l'entrée tapée reste strictement validée
    - Verify : ✅ `make test` vert à 100 % ; le bac GEX réel imprime son organigramme et sa couverture (règles 1–4 satisfaites par les électifs choisis, règle 5 et exigence linguistique remontées), infaisable prouvé = sortie 2

- [x] Tests de propriété transverses de B
    - Verify : ✅ sur instances générées (saisons, offres, plages, préalables `Needs`/`Either`/`CreditsBefore`, réussis, épinglés, plafond, concomitance), l'ensemble retourné **égale** l'énumération brute naïve — donc chaque solution respecte précédence, offre, capacité, épinglages, et chaque session est horaire-faisable via A ; le vérificateur est verrouillé par les 14 fixtures et ses tests unitaires (contenus de `counted`/`candidates`, arithmétique des manques)

- [ ] Câblage `ui` (jalon 7+ ; hors cœur) : `core` piloté depuis Dioxus, snapshot chargé au navigateur — **aucune règle métier dans la vue** (invariant) : couverture des règles, candidats et validation viennent tous de `core`. Lire `.claude/dioxus.md` avant tout code Dioxus 0.7

---

## Encore à planifier (à faire remonter, pas à inventer)

Ces points sont des **décisions ou des données manquantes**, pas des tâches d'implémentation ; les trancher avec l'utilisateur et les consigner en ADR avant de coder ce qui en dépend.
Tranchés le 2026-07-30 (avec Antoine, ADR individuels) : le budget de B (double borne — `2026-07-budget-de-b-en-double-borne`), le préalable hors liste (non bloquant, remonté — `2026-07-prealable-inconnu-non-bloquant-remonte`, restreint le 2026-07-31 aux codes 0xxx — `2026-07-presomption-limitee-au-preuniversitaire`), la pondération des crédits en intervalle (borne basse en planification — `2026-07-credits-range-borne-basse-en-planification`) et la somme au-dessus du `max` (erreur typée — `2026-07-somme-au-dessus-du-max-en-erreur-typee`).

- **Plafond de crédits par session** : dur (17 ?) ou cible molle — le chiffre n'a aucune source documentée, à confirmer avec le directeur (la mécanique est en place : le plafond est une entrée, `--credit-cap` au harnais).
- **Dédoublonnage des solutions de B** : la mesure du 2026-07-30 (Phase 3) montre que l'ensemble complet du bac GEX dépasse 500 000 solutions — variantes d'électifs et de cours lâches interchangeables ; forme du dédoublonnage (classes d'équivalence ?) ou plafond UI bas à trancher avant le jalon 9.
- **GCI-1011 sans page de cours** : obligatoire du bac GEX 2026, absent de tous les snapshots 2009–2026 — trou de scrape ou cours jamais offert ? À vérifier à la source ; en attendant, le harnais l'écarte bruyamment (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`).
- **Sémantique exacte des préférences de A** (journées compactes, matins libres, pause dîner) — pour le classement du jalon 10, à calibrer contre des données réelles (Phase 0 laisse les signatures ouvertes) ; B n'a plus d'objectif.
- **Interaction règles × profils** (jalon 6) — désormais côté vérificateur/affichage, plus côté solveur ; le vérificateur accepte déjà une portée profil.
- **Format JSON de l'organigramme** échangé entre « Cours pour le programme » et l'horaire hebdomadaire — forme provisoire concrète fixée par les fixtures (saisons ordonnées des sessions + solution en carte cours → numéro de session, ADR `2026-07-schema-des-fixtures-de-placement`) ; l'intégration UI reste ouverte.
- **Double comptage au vérificateur** : un cours candidat aux listes de deux règles compte-t-il pour les deux ? Sans effet sur les rapports par règle, décisif pour « ce qui manque pour diplômer » — à trancher avec le directeur.
- **Concomitance par arête** (jetons ombrés des organigrammes PDF) : `PrereqTree` ne la porte pas — l'astérisque du site ne survit que dans `prerequisites.raw` ; seule l'option globale est fixturée, l'encoder est une décision scraper/modèle de données.
- **Forme minimale du rapport de conflit** de A — tranchée au niveau cours : marquage optionnel `valid: false` par cours et par alternative (sémantique swap), contrat et cas de test figés dans `tests/fixtures/test_cases/schedules/` (ADR `2026-07-contrat-horaire-hebdomadaire-vers-ui`) ; reste ouvert le raffinement « ensemble minimal » Max-CSP pour le cas infaisable (le cas « paires compatibles, ensemble infaisable » est couvert par `triple-infeasible-pairwise-ok.json`).
- **Arbitrage de la référence B** : les fixtures sont reproduites à l'identique par l'implémentation Rust — les scripts `tests/reference/solveur_b/` peuvent être supprimés après confirmation par Antoine (ADR `2026-07-reference-b-versionnee-jusqua-larbitrage` : « quand l'implémentation Rust reproduit les fixtures »).
