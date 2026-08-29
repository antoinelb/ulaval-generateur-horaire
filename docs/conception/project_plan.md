# PLAN — Générateur d'horaire / planificateur de cheminement

**Date :** juillet 2026.
**Statut : archivé (2026-08-19).**
Ce document a guidé la construction de la première version livrée (scraper, solveurs A et B, UI des jalons 3–9) ; il n'est plus tenu à jour.
La source de vérité est désormais `docs/conception/adr/` — une décision par fichier ; en cas de contradiction, l'ADR a préséance sur ce plan comme sur le reste de `docs/conception/`.

---

## Fonctionnalités

### Acquisition des données (scraper)

- Extraire des pages publiques de l'ULaval : le catalogue (~10 000 cours), les cours offerts par session (sections, NRC, plages horaires, sections liées, préalables, programmes contributoires, équivalences) et les règles des programmes (crédits exigés, cours obligatoires, « Règle N – X crédits parmi »).
- Produire un snapshot JSON unique `data/cours.json` — chaque cours entier, chaque saison datée de sa dernière année d'offre (`last_offered`) — plus un fichier des programmes (ADR `2026-07-snapshot-unique-des-cours-millesime-par-saison`).
- Une session future sans horaire publié réutilise l'offre la plus récente de la même saison, par cours via `last_offered` (hypothèse de Daniel) ; un cours nouveau sans aucune section de session est gardé, offert automne+hiver, horaire inconnu (`options: null` — ADR `2026-07-cours-sans-section-de-session-offert-automne-hiver`).
- Le scrape du catalogue est toujours complet — l'union des facettes matières, aucun mode scopé (ADR `2026-07-scraper-plein-catalogue-seulement`) ; seules les pages programmes se limitent aux programmes nécessaires.
- Reprendre un scrape interrompu sans tout refaire ; throttler à ~10 requêtes/seconde (~20 min pour le catalogue complet).
- Parser les préalables (ET/OU parenthésés, exigences de crédits) et les règles de programme en arbres structurés ; toute expression hors grammaire est conservée en brut et signalée, jamais perdue silencieusement.
- Tourner en CLI et sur un cron CI : les données sont à jour quand Daniel ouvre l'application, sans qu'il ait jamais à lancer quoi que ce soit ; un scrape qui échoue alerte un humain (le mode de défaillance est des données silencieusement périmées).

### Application (frontend)

#### Horaire hebdomadaire pour la session

- Chercher des cours parmi ceux disponibles pour la session visée (automne, hiver ou été).
- Filtrer les cours disponibles par matière, cycle, programme.
- Ajouter un cours à l'horaire actuel.
- Ajouter un cours directement par son code, sans passer par la recherche (le flux minimal de la v0).
- Enlever un cours de l'horaire actuel.
- Affichage du nombre total de crédits de l'horaire actuel.
- Ajouter manuellement un cours avec son horaire (ex. session à l'étranger, autre université).
- Proposer un cours ajouté à la main au catalogue partagé : un bouton ouvre une issue GitHub préremplie avec son JSON ; une fois commité dans `data/cours.manuel.json`, il est visible de tous (ADR `2026-07-contribution-de-cours-manuels`).
- Visualisation de tous les cours sélectionnés dans un horaire hebdomadaire.
- Choix automatique d'une combinaison de sections sans conflit (une section de chaque type par cours, sections liées obligatoires incluses).
- Quand un cours a un équivalent, utiliser l'horaire du plus récent des deux.
- Le choix n'est pas opaque : les autres sections d'un cours restent visibles, et cliquer une section la force ; le reste se recalcule autour.
- Mise en évidence des conflits d'horaire ; s'il n'existe aucune combinaison valide, les plages en conflit sont identifiées et surlignées.
- Classement des combinaisons valides selon des préférences (journées compactes, matins libres, pause dîner) — modèle exact à concevoir contre des données réelles.
- Présentation des cours du programme selon ses règles et profils, avec mise en évidence des règles pour chacun des cours.
- Choix de la concentration et du profil au panneau (deux menus, option neutre « Aucune »/« Aucun » ; défaut : la première concentration du millésime, jamais de profil imposé), changeable à tout moment sans toucher la grille placée ; le solveur, le bilan de couverture et les crédits « en sus » suivent la portée choisie (ADR `2026-08-selection-concentration-et-profil-au-panneau`).

##### Automatisation de la création d'horaire

- Ajout automatique des cours à partir d'un organigramme fourni en JSON (format provisoire ; sa forme et son intégration avec « Cours pour le programme » restent à définir).
- Mise en évidence des cours qui rentreraient dans l'horaire.
- Ajouter manuellement un cours avec son horaire.
- Filtre des cours dont les préalables ne sont pas remplis (option pour permettre ou non les préalables faits à la même session).

#### Cours pour le programme (création de l'organigramme)

- Construire l'organigramme du programme (quel cours à quelle session, A1→H8 mappé sur des sessions réelles) sous contraintes fournies par l'utilisateur : cours voulus, sessions remplies à la main, session à l'étranger, etc. Un cours déjà réussi s'exprime en le plaçant dans sa session passée — l'interface n'a pas de marquage « réussi » distinct ; seul le mécanisme `passed` de `core` subsiste, nourri par la case « scolarité préparatoire faite » (ADR `2026-08-retrait-de-la-notion-de-cours-reussi`).
- Respecter les règles du programme (obligatoires, « Règle N – X crédits parmi », sous-groupes) et l'ordre imposé par les préalables (équivalences comprises), selon les sessions d'offre.
- Partir du cheminement type GEX pré-chargé (encodé à la main — seule donnée sans source machine-lisible).
- Afficher la couverture des règles : ce qui est satisfait, ce qui reste à combler, et des cours candidats pour combler.
- Prendre un cours est un geste, le geler en est un autre : chaque rangée du panneau offre « automatique » (le cours est voulu, le solveur choisit sa session) et une puce par session de l'horizon où il est offert (le cours est voulu **et** gelé là). Le choix retenu se voit, un « ✕ » le retire, et un cours obligatoire n'en a pas — il est toujours voulu (ADR `2026-08-choix-automatique-ou-session-gelee`).
- Produire l'organigramme consommé par l'automatisation de l'horaire hebdomadaire (l'échange JSON ci-dessus).

#### Transversal

- Reprise côté client (`localStorage`) : filtres, session affichée, horaire et organigramme courants — l'utilisateur reprend exactement où il était.
- Partage par URL : le lien porte **l'organigramme entier** (programme et millésime, sessions, épinglages, sections forcées, cours manuels complets, ententes, cours crédités) dans le fragment `#…` — le destinataire colle le lien et voit tout, sans rien ajuster (ADR `2026-08-partage-de-lorganigramme-complet-en-fragment`).
- L'agencement des écrans n'est pas figé (pas nécessairement un écran unique) ; ce document identifie les fonctionnalités, pas leur agencement.

### Portée

Cœur (requis explicites de Daniel) : acquisition des données, horaire hebdomadaire d'une session avec combinaison automatique, application au catalogue complet.
Vision complète : + automatisation par organigramme, création de l'organigramme, préférences, partage.
Heures du mandat : cœur ≈ 24–34 h, vision complète ≈ 53–78 h ; le surcoût d'apprentissage Rust est absorbé par Antoine et n'y change rien.

---

## Contraintes

### Source de données

- Les pages cours, catalogue et programmes sont rendues côté serveur et accessibles par de simples GET ; parseur HTML simple, **pas de navigateur headless**.
- Un scrape complet ≈ 10 000 requêtes ; politesse obligatoire (~10 requêtes/seconde), d'où filtres et reprise sur erreur.
- Le mapping cours → programmes se construit depuis les pages cours (« Cette activité est contributoire dans : ») ; seuls les programmes dont on veut les règles nécessitent leur page (~400 pages programmes évitées).
- Les cheminements types (A1→H8) n'ont aucune source machine-lisible : encodés à la main sous `data/cheminements/{code}-{semester}[-{concentration}].json`, un fichier par cheminement, que le scraper n'écrit jamais — le document est la grille seule (`completed`, `sessions`), le nom du fichier portant programme, millésime et concentration en snake case (autorité `core::cheminement`, ADR `2026-08-un-cheminement-par-fichier`) ; 26 fichiers convertis depuis les CSV du dépôt JS et revérifiés entrée par entrée contre eux, B-GPH en attente d'une vraie source (son index amont est la copie de celui du B-GMC).
- Une page programme se lit en groupes → blocs → accordéons ; le `<h3>` qui nomme le rôle d'un groupe (« Concentrations », « Profils ») **manque parfois** (bac en génie mécanique), et un groupe non étiqueté à plusieurs blocs est alors lu comme des concentrations, avec anomalie (ADR `2026-07-blocs-de-la-page-programme`).
  Un bloc rend ses « Cours obligatoires » en `mandatory` (y compris dans une concentration — ADR `2026-07-cours-obligatoires-de-concentration`) ; une contrainte de règle illisible reste absente plutôt qu'inventée (ADR `2026-07-contrainte-de-regle-optionnelle`) ; la prose qu'aucune grammaire ne couvre — étiquettes de sous-groupes notamment — est conservée en `notes` et affichée, jamais interprétée (ADR `2026-07-notes-en-prose-conservees`, `2026-07-texte-brut-de-regle-paragraphe-complet`).
  Une contrainte de règle est étiquetée `{type: course|credits, min, max}` (« Un cours parmi » = min 1, max 1 ; l'affichage — nombre exact ou intervalle — est un choix de l'UI) ; dépasser le max est une erreur typée, jamais un verdict inventé (ADR `2026-08-contrainte-etiquetee-min-max`, `2026-07-somme-au-dessus-du-max-en-erreur-typee`).
  L'**exigence linguistique** fait exception : ANL-2020/VEPT pour la personne francophone, FLS-2093/TCF-TP pour la non-francophone, c'est une porte de diplômation où le score au test dispense du cours, donc elle est interprétée dans un champ dédié `language_requirement` du programme (cours + seuils comparables + `raw`) plutôt que laissée en note (ADR `2026-07-exigence-linguistique-champ-dedie`).
  Le **stage obligatoire en prose** des bacs de génie fait aussi exception : promu en règle « Stages » ajoutée à la suite des règles scrapées (`{type: course, min: 1, max: 8}`, tous les sigles de la note dans l'ordre, note entière conservée sur la règle), avec `credits_in_addition: true` puisque « les crédits de ces stages sont en sus des crédits exigés » (ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`).
  Une règle valide sans liste énumérable mais reconnue — cours « convenus avec la direction », « requis par sa concentration », passage intégré au deuxième cycle — devient `courses: "negotiated"` (texte gardé en `raw`) et n'est plus signalée comme anomalie (ADR `2026-07-regles-negociees-reconnues`).
  La **scolarité préparatoire** est une règle calculée, pas scrapée : appendée en dernier par `ulaval-scraper program`, titrée « Scolarité préparatoire », sans contrainte, elle liste tous les cours 0xxx atteignables transitivement depuis les `mandatory` du programme via les arbres de préalables de `data/cours.json` (branches OU aplaties ; omise si vide) — la commande exige donc `data/cours.json` et échoue immédiatement s'il manque (ADR `2026-08-regle-scolarite-preparatoire`).
- La dérive du markup ULaval est une certitude, pas un risque : on ne la prévient pas, on la rend bruyante (tests sur fixtures, alertes CI) et peu coûteuse à réparer.
  Une sortie attendue est **produite par le parseur** depuis le HTML gelé puis relue, jamais écrite à la main : les trois premières fixtures programmes, saisies à la main quatre jours avant le gel du HTML, avaient silencieusement perdu deux cours et fabriqué une contrainte (ADR `2026-07-fixtures-programmes-regenerees`).
  Quand la sortie attendue doit exister **avant** que le parseur sache la produire — un cas de test écrit d'abord —, elle est dérivée du HTML gelé par une implémentation de référence indépendante, validée sur les fixtures déjà figées, puis confrontée au parseur corrigé (ADR `2026-07-fixture-attendue-derivee-avant-le-parseur`).
- Une saison d'un cours porte des **combinaisons d'inscription complètes** (`options`) et non des groupes de choix : on retient une option en entier et on unit les plages de ses sections, ce qui rend inconstruisible l'appariement d'une section avec un laboratoire qui n'est pas le sien (ADR `2026-07-sections-en-combinaisons-valides`).
  Les crédits d'un cours sont un nombre ou un intervalle `{min, max}` pour les stages que l'étudiant pondère (ADR `2026-07-credits-variables-en-enum`).
  Le périmètre se décide deux fois : le filtre `8xxx` du catalogue épargne une requête mais n'est pas exhaustif, le cycle lu sur la page fait autorité (ADR `2026-07-cycles-hors-perimetre-sans-erreur`).
  Les cours `0xxx` sont réintégrés, leur cycle « Préuniversitaire » porté par un type `CourseCycle` propre au cours (ADR `2026-07-cours-dappoint-reintegres`, `2026-07-cycle-preuniversitaire-cours-seulement`).

### Produit

- Utilisable par un non-technicien (Daniel) : un lien à ouvrir, aucun rituel de mise à jour, aucun binaire à installer.
- Le domaine est français ; le vocabulaire du domaine reste français partout (`cours`, `cheminement`, `préalables`, `matière`, `session`, `jalon`).
- Fraîcheur requise : au changement de session seulement ; aucun suivi de changements en cours de session (décision explicite).
- Chaque jalon est démontrable ; ~10 h/semaine ≈ un jalon par semaine.

### Architecture (les invariantes)

- **Toute la logique métier vit en un seul endroit pur et testable** ; la couche de vue est mécanique et sans règle métier (reformulation de la « ligne rouge » : la frontière n'est plus un processus backend mais un module pur).
- **Aucune base de données, aucune persistance serveur** ; le catalogue est en lecture seule, l'état de l'utilisateur vit côté client.
- **Ne jamais rien perdre silencieusement** : préalable hors grammaire → `{"brut": "…"}`, règle non reconnue → texte brut ; affichés et comptés, jamais ignorés.
- **Remplacement atomique des snapshots** : fichier temporaire puis `rename` ; les snapshots existants restent servis pendant un scrape.
- Récolte (réseau) et parsing strictement séparés : le parseur se teste sur des HTML gelés en fixtures ; un changement de markup = un test qui échoue, pas des données corrompues en silence.

### Apprentissage

- Le projet sert aussi à apprendre Rust en profondeur ; les fondations transférables (workspace Cargo, cœur pur + coquilles minces, pipeline WASM) préparent un futur jumeau numérique côtier, dont les décisions propres (3D) sont explicitement hors portée.

---

## Implémentation

### Architecture : site statique, sans serveur

Application cliente WASM servie en fichiers statiques — GitHub Pages, où l'interface occupe la **racine** du site de projet (`make ui-build` la construit avec `--base-path`, le service worker déposé à côté de l'index pour que sa portée couvre l'application — ADR `2026-08-interface-publiee-a-la-racine-de-pages`), qui sert aussi le module wasm (`pkg/`), les snapshots (`data/`) et la documentation (`docs/`, un livre mdBook en français sous `docs/livre/`, construit par `make docs` et par la CI — ADR `2026-08-documentation-mdbook-en-francais`) avec `Access-Control-Allow-Origin: *` (ADR `2026-08-ci-et-publication-sur-github-pages`) ; les données sont des snapshots JSON pré-générés, servis depuis la même origine.
Le site publié est toujours le code du dernier tag `v*` avec le `data/` courant de `main` — un push sur `main` ne déploie plus rien, seul un tag (ou le redéploiement de données) publie (ADR `2026-08-publication-du-site-sur-tag`).
**Il n'y a aucun backend** : le solveur tourne dans le navigateur, et le partage passe par l'URL, donc rien n'exige de serveur.
Un crate serveur (Axum) est réservé mais non construit ; déclencheurs qui justifieraient de le bâtir : rafraîchissement à la demande plus rapide que le cron (ex. suivi des places en période d'inscription), ou état partagé mutable (comptes).

### Stack : Rust de bout en bout (décidé)

Un dépôt, un workspace Cargo :

- **`core`** (bibliothèque) — types du domaine (`Course`, `Section`, `TimeSlot`, `Schedule`, arbres de préalables, règles), détection de conflits, combinaison/classement de sections, génération d'organigramme sous contraintes, validation des préalables.
  Zéro IO, zéro async ; compile identiquement en natif (scraper, tests) et en WASM (UI).
- **`scraper`** (binaire natif, async) — récolte + parsing → snapshots JSON ; dépend de `core` pour les types de sortie ; `tokio` + `reqwest` + `scraper` + `serde` ; `thiserror` dans la bibliothèque, `anyhow` à la frontière du binaire.
- **`ui`** (binaire WASM) — frontend Dioxus 0.7, rendu client ; charge le snapshot JSON, pilote `core`, affiche.
  Il consomme aussi les modules purs de `wasm` (`credits`, `merge`) en rlib, et son Web Worker charge le module wasm de ce même crate — mais il le lie en `default-features = false`, la colle JavaScript qu'il n'appelle jamais coûtant ~800 Ko de son propre wasm (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`).
- **`wasm`** (bibliothèque `cdylib` + rlib) — le crate de frontière : le même `core` exposé aux deux consommateurs navigateur, plus les fonctions pures que `ui` appelle nativement. Une seule orchestration, deux surfaces (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`) :
  - **la surface JavaScript** — huit fonctions : `generate_schedule`/`verify_schedule`, `generate_organigramme`/`verify_organigramme`, plus les questions statiques `admissible_sessions`, `prerequisites_met`, `coverage_report` (bilan seul, sur grille partielle) et `horizon_sessions` (codes de millésime) — entrées et sorties en objets JS ordinaires via `serde-wasm-bindgen`. Vérifier un organigramme = `place` avec tout épinglé (preuve du placement) **et** `coverage_report` (comptage des règles) (ADR `2026-08-module-wasm-quatre-fonctions-js`, `2026-08-surface-wasm-etendue-a-huit-fonctions`).
  - **la surface du worker Dioxus** — `init_snapshot` puis `handle_message` : des chaînes JSON dans les deux sens, une requête sous son `id`, toujours une réponse (ADR `2026-08-crate-ui-calculations-et-worker`).
  - **le catalogue est chargé une fois** : `courses` est optionnel dans les entrées, `init_snapshot` remplissant un cache partagé par les deux surfaces. Le transport coûtait ~66 ms par appel — la totalité du temps d'une fonction courte comme `generate_schedule` (ADR `2026-08-snapshot-en-cache-dans-le-module-wasm`).
  - `courses` est un **paramètre** des fonctions pures, jamais lu depuis l'entrée : c'est ce qui permet aux deux surfaces de partager la même orchestration.
  Les fonctions Rust sont pures et testées nativement ; la colle `#[wasm_bindgen]` n'existe que sous `cfg(all(target_arch = "wasm32", feature = "boundary"))`, la feature étant active par défaut. `make wasm` produit le paquet npm, `make ui-calc` le même crate dans les assets du `ui`. Le `.d.ts` du paquet porte de vrais types dérivés des structs serde (`tsify` en mode déclaratif, feature `tsify` de `core` activée par le build navigateur seul) et le JSDoc français des exports vient de leurs commentaires rustdoc (ADR `2026-08-types-typescript-tsify-declaratif`).
- `server` (Axum) et un wrapper desktop sont des noms réservés, construits seulement si leurs déclencheurs se matérialisent.

Les répertoires gardent les noms `core`/`scraper`/`ui` ; les paquets Cargo sont préfixés `ulaval-scheduler-` (ADR `2026-07-nommage-des-crates-prefixe-ulaval-scheduler`).

Convention de langue : le domaine s'exprime en français dans la prose, la documentation et l'interface, mais **le code est en anglais** — identifiants (variables, fonctions, types), messages d'erreur et clés de données JSON (ADR `2026-07-code-en-anglais-domaine-en-francais`).

Alternatives rejetées (raisonnement complet dans `docs/conception/`) : Python + JS vanilla, Rust au scraper seulement, Leptos (second choix), Yew, iced, hybride Elm + WASM.

### Flux de données de bout en bout

Cron GitHub Actions — déclenché quotidiennement, mais ne lançant un scrape complet qu'aux mois-jours listés dans `data/dates_scraping.txt` (première ligne `mm-jj` pour le format, puis une date par ligne, sans année pour valoir chaque année — ADR `2026-08-scraping-pilote-par-fichier-de-dates`) — → binaire `scraper` (GET throttlés à ~10 req/s par un throttle partagé honorant `Retry-After`, pagination du catalogue calculée depuis la page 0 (borne supérieure, pages « Aucun résultat » excédentaires tolérées) puis vérifiée par réconciliation arithmétique, catalogue complet = union des facettes matières partitionnées (l'index du site plafonne toute requête à 10 000 résultats ; la bannière est ignorée, le widget troué étant un bug du site assumé) — ADR `2026-07-conception-du-fetcher`, `2026-07-pagination-du-catalogue-par-comptage`, `2026-07-tolerance-des-pages-aucun-resultat-du-fan-out`, `2026-07-partition-du-catalogue-par-matiere`, `2026-07-le-catalogue-est-lunion-des-facettes` — parsing via les types de `core`) → `data/catalogue.json` (catalogue complet trié/dédupliqué par code, écrit seulement si ≥ 90 % du compte précédent) + `data/catalogue_errors.log` (anomalies brutes, une par ligne ; le cron alerte si non vide), puis, à partir de ce catalogue comme file de travail, `data/cours.json` — un seul fichier, chaque `Course` multi-saisons entier, chaque saison datée `last_offered` (ADR `2026-07-snapshot-unique-des-cours-millesime-par-saison`) — + `data/cours_errors.log` (une page en échec est une anomalie, jamais un arrêt) + `data/programmes/{code}-{semestre}.json` — un fichier par programme et par semestre, nommé par le code officiel du répertoire (`B-GEX-A26.json`), extrait des ids d'accordéon de la page ; le slug d'URL est un champ dédié (ADR `2026-08-code-officiel-de-programme-et-slug`) — (`A26`/`H27`, la session qui suit le scrape — un programme n'est défini que pour l'automne ou l'hiver, jamais l'été : septembre–décembre → `H` de l'année suivante, tous les autres mois → `A` de l'année courante ; les étudiants gardent la version de leur inscription — ADR `2026-08-millesime-de-programme-en-semestre`, `2026-08-millesime-automne-ou-hiver-jamais-ete` ; le champ `possible_semester_start` porte les sessions d'admission de la page, en lettres `["A", "H"]`), écrit par `ulaval-scraper program [<url>...]` — sans URL, le run rafraîchit tous les programmes déjà présents, slug lu dans le champ `slug` de chaque snapshot (ADR `2026-08-code-officiel-de-programme-et-slug`) ; avec des URL, il ne touche que les programmes nommés — + `data/programmes_errors.log` (ADR `2026-07-un-fichier-par-programme`, `2026-07-echec-de-page-programme-non-bloquant`) → commit du snapshot → redéploiement du site statique (dispatch explicite de `ci.yml`, un push du `GITHUB_TOKEN` ne déclenchant aucun workflow — ADR `2026-08-ci-et-publication-sur-github-pages` ; le dispatch republie le code du dernier tag `v*` avec les données fraîches de `main` — ADR `2026-08-publication-du-site-sur-tag`) → `ui` charge le JSON dans le navigateur, tout le calcul tourne localement via `core` → un horaire choisi se partage en URL.
Aucun serveur nulle part dans le chemin.
En parallèle, `data/cours.manuel.json` (jamais touché par le scraper) est copié en asset et fusionné au chargement avec le snapshot, scrapé prioritaire en cas de collision de code — ADR `2026-07-contribution-de-cours-manuels` ; il porte deux choses : `courses`, les cours sans source machine-lisible, et `vintages`, la surcouche `{"A24": {"prerequisites": {"GCI-2000": "GCI-1000 ET MAT-1902"}}}` qui rétablit, par millésime d'admission, les préalables qu'un cours avait sous une version antérieure du programme (ADR `2026-08-correction-des-prealables-par-millesime`). L'étudiant corrige lui-même ce que le fichier ne couvre pas, dans `Plan.prereq_overrides` — sa correction l'emporte, voyage dans le lien et est appliquée aux `Course` en amont du solveur, jamais passée en paramètre. Une expression illisible, un sigle absent ou un préalable officiel qui a bougé depuis sont remontés, jamais tus.
Le fichier existe depuis 2026-08-15 : les cours hors catalogue de l'interface JS (gabarits `OPT-*`, échanges `EHE-*`, `AUC-HOIX`, etc.), chacun offert en toute saison avec horaire non publié (`last_offered`/`options` nuls — ADR `2026-08-cours-manuels-offerts-en-toute-saison`).

Entre les deux phases, `data/cache/cours/{code}.json` (gitignoré) garde les cours déjà parsés sans anomalie, pour qu'une relance ne refasse que les pages qui en ont besoin — ADR `2026-07-cache-de-cours-parses`.
Un changement de format du `Course` sérialisé périme d'un coup tout le cache : chaque fichier redevient un défaut, silencieusement, et la relance est froide sans le dire.
La ligne de clôture du scrape annonce donc la répartition (`Scraped 8826 courses (8518 cached, 308 fetched).`) — sans elle, un cache périmé est indiscernable d'un throttle mal placé.
Le cache porte aussi le verdict « hors périmètre » (les ~20 pages `MDD-5xxx`/`PSY-785x` qui ne donnent aucun cours), stampé de l'empreinte de la règle de périmètre et retesté à la lecture, si bien qu'une relance en cache fait 0 requête sans jamais rester périmée si la règle change — ADR `2026-07-cache-du-verdict-hors-perimetre`.

Un run restreint (`--subjects gex`) **fusionne** dans le snapshot existant au lieu de le remplacer : il réécrit exactement les cours de ses matières et laisse les autres intacts, en triant par code comme le ferait un run complet — ADR `2026-07-run-par-matiere-fusionne-dans-le-snapshot`.

Le spike du 2026-07-02 a confirmé que les pages observées sont accessibles par de simples GET (ni session, ni POST de formulaire) ; le cookie store de `reqwest` reste un repli si certaines pages l'exigent (à vérifier à la semaine 1).

### Ordre de construction

1. **Scraper d'abord** — tue le plus gros risque externe (la forme réelle des données) avant que du code n'en dépende ; démarche test-first : fixtures e2e des pages catalogue/cours/programme → parseur validé → tests unitaires.
   Les sorties attendues vivent dans `tests/fixtures/test_cases/` (`catalogue/`, `classes/`, `programs/`) ; pour le catalogue, la vérité terrain est le catalogue fusionné de la facette GEX (`catalogue/gex.json`), comparé au parsing de pages HTML gelées, les comportements par page (page vide, `total_results` optionnel) étant épinglés par des tests unitaires (ADR `2026-07-catalogue-artefact-commite`, révisé par `2026-07-catalogue-teste-sur-html-gele`).
   Livrable : `data/cours.json` + fixtures HTML + tests du parseur.
2. **Cœur ensuite** — Rust pur contre les vraies données de l'étape 1 : combinaison de sections, préférences, préalables, génération d'organigramme (démarche détaillée : `docs/next_steps.md`).
   Livrable : un harnais CLI/test qui imprime des horaires valides pour des codes de cours donnés, absence de conflit testée par propriétés.
3. **UI en dernier** — à ce stade c'est un problème de rendu, pas de conception.
4. **Cron CI** — ~30 lignes de YAML autour du binaire existant + notifications d'échec ; le YAML lit `data/dates_scraping.txt` pour décider si le jour courant est un jour de scrape.

Chaque étape consomme la sortie réelle de la précédente ; l'inconnue la plus risquée meurt en premier.

### Risques

| Risque | Mitigation |
|---|---|
| Dérive du markup ULaval (certitude) | Séparation récolte/parsing, fixtures, alertes CI ; la métrique est le temps de réparation |
| Variété des cas particuliers du catalogue (stages, multi-sessions, distance, préalables inédits) | Grammaires extensibles + « jamais de perte silencieuse » ; principal risque résiduel d'estimation |
| Churn des frameworks pré-1.0 (Dioxus) | Cœur pur : une migration ne touche que la vue ; versions épinglées |
| Accès au portail depuis le CI | User agent honnête, throttling, vérification des conditions d'utilisation ; repli : machine personnelle planifiée |
| Taille du bundle WASM | Non préoccupant à cette échelle ; code splitting + `wasm-opt` si besoin |

---

## Versions et jalons hebdomadaires

Chaque jalon est démontrable en fin de semaine ; à ~10 h/semaine, une semaine ≈ un jalon.
Les heures facturables sont celles du mandat (voir « Portée ») ; le temps d'apprentissage Rust est absorbé par Antoine — il peut étirer le calendrier, jamais les heures.
L'ordre suit l'ordre de construction (scraper → cœur → UI → cron) : chaque semaine consomme la sortie réelle de la précédente.
Les jalons se regroupent en trois versions livrables, chacune utilisable de bout en bout (ADR `2026-07-decoupage-en-versions-v0-v1-v2`).

### v0 (MVP) — semaines 1 à 3

Entrer des codes de cours pour une session : l'horaire se crée automatiquement et s'affiche avec les conflits d'horaire évidents ; on peut ajouter et enlever des cours, et le nombre de crédits est affiché.

| Semaine | Jalon | Démonstration |
|---|---|---|
| 1 | **Scraper d'une session** (test-first) : workspace Cargo, types du domaine dans `core`, fixtures e2e des pages catalogue et cours, parseur validé, snapshot des cours pour les matières GEX | Le JSON de GCI-1007 (cours + laboratoires + sections liées) est correct |
| 2 | **Cœur solveur** : détection de conflits, combinaison automatique de sections (backtracking borné, une section de chaque type, sections liées incluses), harnais CLI | Le harnais imprime un horaire valide pour une liste de codes de cours ; absence de conflit testée par propriétés |
| 3 | **UI minimale de l'horaire** : app Dioxus servie en statique, ajout/retrait de cours par code, grille hebdomadaire, combinaison automatique affichée, plages en conflit surlignées quand aucune combinaison n'existe, nombre total de crédits affiché | Le requis central de Daniel de bout en bout : entrer des codes de cours d'une session → l'horaire se monte tout de suite, crédits et conflits visibles |

### v1 — semaines 4 à 6

Choisir les cours d'une liste plutôt que par code, le programme présentant ses cours selon ses règles et profils.

| Semaine | Jalon | Démonstration |
|---|---|---|
| 4 | **Horaire complet** : recherche et filtres (matière, cycle, programme), sections visibles et cliquables (le reste se recalcule autour), ajout manuel d'un cours avec son horaire, reprise `localStorage` | Choisir des cours dans la liste, forcer un NRC et voir l'horaire se recalculer ; fermer puis rouvrir le navigateur sans rien perdre |
| 5 | **Catalogue complet + cron CI** : reprise sur erreur, throttling, écriture atomique ; workflow planifié selon `data/dates_scraping.txt`, notifications d'échec, déploiement statique automatique | Le site public se met à jour sans intervention ; un scrape interrompu reprend où il était |
| 6 | **Programmes et préalables** : scraper des pages programmes (obligatoires + règles + profils, validé sur GEX), grammaire des préalables (ET/OU, crédits exigés) ; dans l'UI, cours du programme présentés selon ses règles et profils, filtre des cours aux préalables non remplis (option concomitants) | Les règles du bac GEX en JSON fidèle ; la liste de cours s'organise selon les règles et profils et se filtre selon les préalables |

### v2 — semaines 7 à 10

Le bac complet : les sessions se remplissent automatiquement et restent modifiables.

| Semaine | Jalon | Démonstration |
|---|---|---|
| 7 | **Automatisation par organigramme** : format JSON provisoire de l'organigramme, ajout automatique des cours de la session visée, mise en évidence des cours qui rentreraient dans l'horaire | Charger un organigramme → l'horaire de la session se remplit tout seul |
| 8 | **Couverture des règles** : satisfait / à combler / cours candidats pour un organigramme donné ; une règle-liste sans contrainte (« Scolarité préparatoire ») est comptée (`counted`/`candidates`) mais reste `reported` (`2026-08-regle-sans-contrainte-comptee-mais-reportee`) ; validation de l'ordre des cours selon les préalables (équivalences comprises) | L'organigramme signale un cours placé avant son préalable et ce qui manque pour diplômer |
| 9 | **Génération de l'organigramme sous contraintes** : cours réussis, cours voulus, sessions remplies à la main, session à l'étranger ; préalables, offre et plafond respectés ; **placement seul, fait main, toutes les solutions retournées** (bornées par le budget de nœuds) — le solveur ne choisit jamais de cours, l'étudiant fournit la liste et le vérificateur de règles (`core`) valide la sélection et affiche la couverture (ADR `2026-07-b-placement-par-satisfaction-fait-main`, sortie révisée par `2026-07-b-enumere-toutes-les-solutions`) ; un cours implaçable (aucune session, préalables insatisfiables) est détecté avant la recherche et nommé dans `Placement.blocked` au lieu d'épuiser le budget (ADR `2026-07-implacabilite-prouvee-avant-la-recherche`) ; l'horizon insère un été après chaque hiver (dernier inclus), fermé aux cours réguliers sauf ouverture explicite (`open_summers`, défaut : aucun cours l'été), les stages s'y placent sauf épinglage (`2026-08-stage-place-en-ete-sauf-epinglage`, `2026-08-horizon-avec-ete-apres-chaque-hiver`) ; la scolarité préparatoire non réussie et le stage obligatoire entrent dans la liste à placer via l'intake (`2026-08-stage-obligatoire-et-scolarite-preparatoire-dans-lintake`) ; un électif **forcé** par les préalables d'un obligatoire (GLO-1901 au B-GMC) est injecté par l'intake, remonté dans `injected` et adopté par l'UI avec alerte — pas une entorse au « jamais de choix » : quand un seul électif peut satisfaire le préalable, il n'y a rien à choisir, et un vrai choix (`any` entre électifs) reste bloqué (ADR `2026-08-injection-des-electifs-forces-par-les-prealables`) ; quand aucun agencement complet n'existe, la proposition — automatique à chaque édition, sans bouton (ADR `2026-08-organigramme-en-continu-sans-bouton`) — ne se tait plus : une seconde passe **remplit au mieux**, chaque cours placé respectant toujours toutes les contraintes et ceux qui ne rentrent nulle part étant nommés dans `Solution.left_out` — des trous, jamais une faute (ADR `2026-08-placement-au-mieux-en-repli`) | Fournir ses contraintes et sa liste de cours → tous les organigrammes complets et valides sont énumérés, le premier (le plus proche du cheminement de référence) proposé ; sans agencement complet, une grille partielle qui dit ce qui manque |
| 10 | **Préférences et partage** : classement des combinaisons (journées compactes, matins libres, pause dîner), partage d'un horaire par URL, contribution d'un cours manuel (fusion de `{session}.manuel.json` + bouton d'issue préremplie) ; polissage | Changer une préférence reclasse l'horaire ; l'URL copiée rouvre le même horaire ailleurs ; un cours proposé par Daniel apparaît pour tous après commit |

Le cœur (requis explicites de Daniel, voir « Portée ») = v0 + jalons 4–5 ; la vision complète s'achève avec la v2.

---

## Décisions révisées par rapport à la conception initiale

Historique complet dans `docs/conception/` ; les décisions futures s'ajoutent en fichiers individuels dans `docs/conception/adr/`.

| Sujet | Conception initiale | Décision retenue |
|---|---|---|
| Backend | Backend sans état, tous les calculs serveur, REST + SSE | Aucun backend ; calculs dans le navigateur via `core` (WASM) ; crate `server` réservé avec déclencheurs explicites |
| Scraping in-app | Lancé depuis l'interface, progression SSE | Cron CI + CLI ; la section scraper de l'UI disparaît ; le vrai requis de Daniel (catalogue complet, filtres, reprise) est conservé |
| Organigramme | Éditeur drag-and-drop | Génération sous contraintes (« Cours pour le programme ») + échange JSON provisoire vers l'horaire ; un éditeur interactif reste possible plus tard |
| Ligne rouge | Zéro règle métier côté client (frontière = processus) | Zéro règle métier dans la vue (frontière = module pur `core`) ; l'esprit est inchangé |
| Stack | Question ouverte (A / B / mitoyenne) | Rust de bout en bout : workspace `core`/`scraper`/`ui`, frontend Dioxus |
| Partage | Aucun pour le moment | Partage d'horaire par URL en portée (quasi gratuit en statique) ; reprise `localStorage` inchangée |
| Hébergement | Question ouverte (serveur externe ou local) | Site statique (ex. GitHub Pages) ; « local » = ouvrir les fichiers statiques |

Inchangé et toujours contraignant : hypothèse fondatrice (réutilisation de la saison, désormais par cours via `last_offered`), formats de données, grammaires (règles et préalables), conventions défensives du scraper, `cheminement_type` à la main, JSON plutôt que SQLite, pas de navigateur headless.

---

## Questions ouvertes

- Format de l'organigramme JSON et son intégration entre « Cours pour le programme » et l'horaire hebdomadaire (produit par l'un, consommé par l'autre? édité à la main au début?).
  Forme provisoire concrète depuis les fixtures du solveur B : liste ordonnée de saisons de sessions + solution en carte cours → numéro de session (ADR `2026-07-schema-des-fixtures-de-placement`) ; l'intégration UI reste ouverte.
- Agencement des écrans (pas nécessairement un écran unique) — à explorer une fois les fonctionnalités gelées.
- Le catalogue est-il joignable sans identifiants depuis le CI? (Détermine la gestion de secrets — à résoudre au jalon 1.)
- Modèle exact de préférences/scoring — structure arrêtée dans `docs/conception/solveur-conception.md` (opérations sur bits + somme pondérée) ; poids et sémantique fine à calibrer contre des données réelles.
- Plafond de crédits par session : dur (17 ?) ou cible molle — aucun chiffre documenté, à confirmer avec le directeur ; la mécanique est en place (entrée explicite du solveur B et du harnais, jamais une constante).
- Dédoublonnage des solutions de B : mesuré le 2026-07-30, l'ensemble complet du bac GEX dépasse 500 000 placements (électifs et cours lâches interchangeables) alors que la première solution vient en moins de 50 ms — forme du dédoublonnage ou plafond UI à trancher avant le jalon 9 (`docs/next_steps.md`, Phase 3).
- GCI-1011 (obligatoire du bac GEX 2026) n'a de page de cours dans aucun snapshot — trou de scrape ou cours jamais offert ? À vérifier à la source ; le harnais l'écarte bruyamment en attendant (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`).
- Canal de notification d'échec du cron — résolu : l'échec du job (après le commit du snapshot valide) suffit, GitHub notifie par courriel (ADR `2026-08-ci-et-publication-sur-github-pages`).
- Cheminements types d'autres programmes que GEX : qui les fournit, le cas échéant?
- Niveau de couverture des cas particuliers exigé avant livraison (stages, cours multi-sessions, formation à distance, formes de préalables non observées) — principal risque résiduel d'estimation.
