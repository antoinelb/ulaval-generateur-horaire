# PLAN — Générateur d'horaire / planificateur de cheminement

**Date :** juillet 2026.
**Statut :** scraper (étape 1) livré ; cœur solveur (étape 2) conçu, implémentation à venir — conception dans `docs/conception/solveur-conception.md`, plan d'implémentation dans `docs/next_steps.md`.
**Rôle de ce document :** point d'entrée du projet, autonome — tout ce qu'il faut pour implémenter est ici.
Les documents de conception d'origine sont archivés dans `docs/conception/` ; ils gardent le détail supplémentaire (grammaires, exemples de formats JSON, résultats du spike, alternatives rejetées), mais en cas de contradiction, ce document a préséance.
Toute nouvelle décision est documentée dans un fichier individuel sous `docs/conception/adr/`, et ce document est mis à jour en conséquence : le plan porte le *quoi*, l'ADR conserve le *pourquoi*.

---

## Fonctionnalités

### Acquisition des données (scraper)

- Extraire des pages publiques de l'ULaval : le catalogue (~10 000 cours), les cours offerts par session (sections, NRC, plages horaires, sections liées, préalables, programmes contributoires, équivalences) et les règles des programmes (crédits exigés, cours obligatoires, « Règle N – X crédits parmi »).
- Produire un snapshot JSON par session (`a2026`, `h2027`, …) plus un fichier des programmes.
- Conserver un snapshot par saison, jamais écrasé aveuglément : une session future sans horaire publié réutilise le plus récent de la même saison (hypothèse de Daniel).
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
- Proposer un cours ajouté à la main au catalogue partagé : un bouton ouvre une issue GitHub préremplie avec son JSON ; une fois commité dans `data/cours/{session}.manuel.json`, il est visible de tous (ADR `2026-07-contribution-de-cours-manuels`).
- Visualisation de tous les cours sélectionnés dans un horaire hebdomadaire.
- Choix automatique d'une combinaison de sections sans conflit (une section de chaque type par cours, sections liées obligatoires incluses).
- Quand un cours a un équivalent, utiliser l'horaire du plus récent des deux.
- Le choix n'est pas opaque : les autres sections d'un cours restent visibles, et cliquer une section la force ; le reste se recalcule autour.
- Mise en évidence des conflits d'horaire ; s'il n'existe aucune combinaison valide, les plages en conflit sont identifiées et surlignées.
- Classement des combinaisons valides selon des préférences (journées compactes, matins libres, pause dîner) — modèle exact à concevoir contre des données réelles.
- Présentation des cours du programme selon ses règles et profils, avec mise en évidence des règles pour chacun des cours.

##### Automatisation de la création d'horaire

- Ajout automatique des cours à partir d'un organigramme fourni en JSON (format provisoire ; sa forme et son intégration avec « Cours pour le programme » restent à définir).
- Mise en évidence des cours qui rentreraient dans l'horaire.
- Ajouter manuellement un cours avec son horaire.
- Filtre des cours dont les préalables ne sont pas remplis (option pour permettre ou non les préalables faits à la même session).

#### Cours pour le programme (création de l'organigramme)

- Construire l'organigramme du programme (quel cours à quelle session, A1→H8 mappé sur des sessions réelles) sous contraintes fournies par l'utilisateur : cours déjà réussis, cours voulus, sessions remplies à la main, session à l'étranger, etc.
- Respecter les règles du programme (obligatoires, « Règle N – X crédits parmi », sous-groupes) et l'ordre imposé par les préalables (équivalences comprises), selon les sessions d'offre.
- Partir du cheminement type GEX pré-chargé (encodé à la main — seule donnée sans source machine-lisible).
- Afficher la couverture des règles : ce qui est satisfait, ce qui reste à combler, et des cours candidats pour combler.
- Produire l'organigramme consommé par l'automatisation de l'horaire hebdomadaire (l'échange JSON ci-dessus).

#### Transversal

- Reprise côté client (`localStorage`) : filtres, session affichée, horaire et organigramme courants — l'utilisateur reprend exactement où il était.
- Partage d'un horaire par URL : un horaire choisi n'est qu'un ensemble de sections, encodable dans l'URL.
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
- Le `cheminement_type` (A1→H8) n'a aucune source machine-lisible : encodé à la main, pour GEX seulement, dans `data/programmes/{code}.manuel.json` que le scraper n'écrit jamais (ADR `2026-07-cheminement-type-en-fichier-manuel`).
- Une page programme se lit en groupes → blocs → accordéons ; le `<h3>` qui nomme le rôle d'un groupe (« Concentrations », « Profils ») **manque parfois** (bac en génie mécanique), et un groupe non étiqueté à plusieurs blocs est alors lu comme des concentrations, avec anomalie (ADR `2026-07-blocs-de-la-page-programme`).
  Un bloc rend ses « Cours obligatoires » en `mandatory` (y compris dans une concentration — ADR `2026-07-cours-obligatoires-de-concentration`) ; une contrainte de règle illisible reste absente plutôt qu'inventée (ADR `2026-07-contrainte-de-regle-optionnelle`) ; la prose qu'aucune grammaire ne couvre — étiquettes de sous-groupes, stages exigés pour diplômer — est conservée en `notes` et affichée, jamais interprétée (ADR `2026-07-notes-en-prose-conservees`, `2026-07-texte-brut-de-regle-paragraphe-complet`).
  L'**exigence linguistique** fait exception : ANL-2020/VEPT pour la personne francophone, FLS-2093/TCF-TP pour la non-francophone, c'est une porte de diplômation où le score au test dispense du cours, donc elle est interprétée dans un champ dédié `language_requirement` du programme (cours + seuils comparables + `raw`) plutôt que laissée en note (ADR `2026-07-exigence-linguistique-champ-dedie`).
  Une règle valide sans liste énumérable mais reconnue — cours « convenus avec la direction », « requis par sa concentration », passage intégré au deuxième cycle — devient `courses: "negotiated"` (texte gardé en `raw`) et n'est plus signalée comme anomalie (ADR `2026-07-regles-negociees-reconnues`).
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

Application cliente WASM servie en fichiers statiques (ex. GitHub Pages) ; les données sont des snapshots JSON pré-générés, servis depuis la même origine.
**Il n'y a aucun backend** : le solveur tourne dans le navigateur, et le partage passe par l'URL, donc rien n'exige de serveur.
Un crate serveur (Axum) est réservé mais non construit ; déclencheurs qui justifieraient de le bâtir : rafraîchissement à la demande plus rapide que le cron (ex. suivi des places en période d'inscription), ou état partagé mutable (comptes).

### Stack : Rust de bout en bout (décidé)

Un dépôt, un workspace Cargo :

- **`core`** (bibliothèque) — types du domaine (`Course`, `Section`, `TimeSlot`, `Schedule`, arbres de préalables, règles), détection de conflits, combinaison/classement de sections, génération d'organigramme sous contraintes, validation des préalables.
  Zéro IO, zéro async ; compile identiquement en natif (scraper, tests) et en WASM (UI).
- **`scraper`** (binaire natif, async) — récolte + parsing → snapshots JSON ; dépend de `core` pour les types de sortie ; `tokio` + `reqwest` + `scraper` + `serde` ; `thiserror` dans la bibliothèque, `anyhow` à la frontière du binaire.
- **`ui`** (binaire WASM) — frontend Dioxus 0.7, rendu client ; charge le snapshot JSON, pilote `core`, affiche.
- `server` (Axum) et un wrapper desktop sont des noms réservés, construits seulement si leurs déclencheurs se matérialisent.

Les répertoires gardent les noms `core`/`scraper`/`ui` ; les paquets Cargo sont préfixés `ulaval-scheduler-` (ADR `2026-07-nommage-des-crates-prefixe-ulaval-scheduler`).

Convention de langue : le domaine s'exprime en français dans la prose, la documentation et l'interface, mais **le code est en anglais** — identifiants (variables, fonctions, types), messages d'erreur et clés de données JSON (ADR `2026-07-code-en-anglais-domaine-en-francais`).

Alternatives rejetées (raisonnement complet dans `docs/conception/`) : Python + JS vanilla, Rust au scraper seulement, Leptos (second choix), Yew, iced, hybride Elm + WASM.

### Flux de données de bout en bout

Cron GitHub Actions → binaire `scraper` (GET throttlés à ~10 req/s par un throttle partagé honorant `Retry-After`, pagination du catalogue calculée depuis la page 0 (borne supérieure, pages « Aucun résultat » excédentaires tolérées) puis vérifiée par réconciliation arithmétique, catalogue complet = union des facettes matières partitionnées (l'index du site plafonne toute requête à 10 000 résultats ; la bannière est ignorée, le widget troué étant un bug du site assumé) — ADR `2026-07-conception-du-fetcher`, `2026-07-pagination-du-catalogue-par-comptage`, `2026-07-tolerance-des-pages-aucun-resultat-du-fan-out`, `2026-07-partition-du-catalogue-par-matiere`, `2026-07-le-catalogue-est-lunion-des-facettes` — parsing via les types de `core`) → `data/catalogue.json` (catalogue complet trié/dédupliqué par code, écrit seulement si ≥ 90 % du compte précédent) + `data/catalogue_errors.log` (anomalies brutes, une par ligne ; le cron alerte si non vide), puis, à partir de ce catalogue comme file de travail, `data/cours/{session}.json` — un fichier par couple (saison, année) rencontré, chaque `Course` projeté sur la seule saison concernée — + `data/cours_errors.log` (une page en échec est une anomalie, jamais un arrêt) + `data/programmes/{code}.json` — un fichier par programme, écrit par `ulaval-scraper program <url>...` dont les URL sont des arguments obligatoires (aucune file de travail n'existe pour les programmes), un run ne touchant que les programmes nommés — + `data/programmes_errors.log` (ADR `2026-07-un-fichier-par-programme`, `2026-07-echec-de-page-programme-non-bloquant`) → commit du snapshot → redéploiement du site statique → `ui` charge le JSON dans le navigateur, tout le calcul tourne localement via `core` → un horaire choisi se partage en URL.
Aucun serveur nulle part dans le chemin.
En parallèle, `data/cours/{session}.manuel.json` (cours sans source machine-lisible, jamais touché par le scraper) est fusionné au chargement avec le snapshot, entrées marquées `source: "manuel"` et scrapé prioritaire en cas de collision de code — ADR `2026-07-contribution-de-cours-manuels`.

Entre les deux phases, `data/cache/cours/{code}.json` (gitignoré) garde les cours déjà parsés sans anomalie, pour qu'une relance ne refasse que les pages qui en ont besoin — ADR `2026-07-cache-de-cours-parses`.
Un changement de format du `Course` sérialisé périme d'un coup tout le cache : chaque fichier redevient un défaut, silencieusement, et la relance est froide sans le dire.
La ligne de clôture du scrape annonce donc la répartition (`Scraped 8826 courses (8518 cached, 308 fetched).`) — sans elle, un cache périmé est indiscernable d'un throttle mal placé.
Le cache porte aussi le verdict « hors périmètre » (les ~20 pages `MDD-5xxx`/`PSY-785x` qui ne donnent aucun cours), stampé de l'empreinte de la règle de périmètre et retesté à la lecture, si bien qu'une relance en cache fait 0 requête sans jamais rester périmée si la règle change — ADR `2026-07-cache-du-verdict-hors-perimetre`.

Un run restreint (`--subjects gex`) **fusionne** dans les snapshots existants au lieu de les remplacer : il réécrit exactement les cours de ses matières et laisse les autres intacts, en triant par code comme le ferait un run complet — ADR `2026-07-run-par-matiere-fusionne-dans-le-snapshot`.

Le spike du 2026-07-02 a confirmé que les pages observées sont accessibles par de simples GET (ni session, ni POST de formulaire) ; le cookie store de `reqwest` reste un repli si certaines pages l'exigent (à vérifier à la semaine 1).

### Ordre de construction

1. **Scraper d'abord** — tue le plus gros risque externe (la forme réelle des données) avant que du code n'en dépende ; démarche test-first : fixtures e2e des pages catalogue/cours/programme → parseur validé → tests unitaires.
   Les sorties attendues vivent dans `tests/fixtures/test_cases/` (`catalogue/`, `classes/`, `programs/`) ; pour le catalogue, la vérité terrain est le catalogue fusionné de la facette GEX (`catalogue/gex.json`), comparé au parsing de pages HTML gelées, les comportements par page (page vide, `total_results` optionnel) étant épinglés par des tests unitaires (ADR `2026-07-catalogue-artefact-commite`, révisé par `2026-07-catalogue-teste-sur-html-gele`).
   Livrable : `data/{session}.json` + fixtures HTML + tests du parseur.
2. **Cœur ensuite** — Rust pur contre les vraies données de l'étape 1 : combinaison de sections, préférences, préalables, génération d'organigramme (démarche détaillée : `docs/next_steps.md`).
   Livrable : un harnais CLI/test qui imprime des horaires valides pour des codes de cours donnés, absence de conflit testée par propriétés.
3. **UI en dernier** — à ce stade c'est un problème de rendu, pas de conception.
4. **Cron CI** — ~30 lignes de YAML autour du binaire existant + notifications d'échec.

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
| 1 | **Scraper d'une session** (test-first) : workspace Cargo, types du domaine dans `core`, fixtures e2e des pages catalogue et cours, parseur validé, snapshot `data/cours/a2026.json` pour les matières GEX | Le JSON de GCI-1007 (cours + laboratoires + sections liées) est correct |
| 2 | **Cœur solveur** : détection de conflits, combinaison automatique de sections (backtracking borné, une section de chaque type, sections liées incluses), harnais CLI | Le harnais imprime un horaire valide pour une liste de codes de cours ; absence de conflit testée par propriétés |
| 3 | **UI minimale de l'horaire** : app Dioxus servie en statique, ajout/retrait de cours par code, grille hebdomadaire, combinaison automatique affichée, plages en conflit surlignées quand aucune combinaison n'existe, nombre total de crédits affiché | Le requis central de Daniel de bout en bout : entrer des codes de cours d'une session → l'horaire se monte tout de suite, crédits et conflits visibles |

### v1 — semaines 4 à 6

Choisir les cours d'une liste plutôt que par code, le programme présentant ses cours selon ses règles et profils.

| Semaine | Jalon | Démonstration |
|---|---|---|
| 4 | **Horaire complet** : recherche et filtres (matière, cycle, programme), sections visibles et cliquables (le reste se recalcule autour), ajout manuel d'un cours avec son horaire, reprise `localStorage` | Choisir des cours dans la liste, forcer un NRC et voir l'horaire se recalculer ; fermer puis rouvrir le navigateur sans rien perdre |
| 5 | **Catalogue complet + cron CI** : reprise sur erreur, throttling, écriture atomique ; workflow planifié, notifications d'échec, déploiement statique automatique | Le site public se met à jour sans intervention ; un scrape interrompu reprend où il était |
| 6 | **Programmes et préalables** : scraper des pages programmes (obligatoires + règles + profils, validé sur GEX), grammaire des préalables (ET/OU, crédits exigés) ; dans l'UI, cours du programme présentés selon ses règles et profils, filtre des cours aux préalables non remplis (option concomitants) | Les règles du bac GEX en JSON fidèle ; la liste de cours s'organise selon les règles et profils et se filtre selon les préalables |

### v2 — semaines 7 à 10

Le bac complet : les sessions se remplissent automatiquement et restent modifiables.

| Semaine | Jalon | Démonstration |
|---|---|---|
| 7 | **Automatisation par organigramme** : format JSON provisoire de l'organigramme, ajout automatique des cours de la session visée, mise en évidence des cours qui rentreraient dans l'horaire | Charger un organigramme → l'horaire de la session se remplit tout seul |
| 8 | **Couverture des règles** : satisfait / à combler / cours candidats pour un organigramme donné ; validation de l'ordre des cours selon les préalables (équivalences comprises) | L'organigramme signale un cours placé avant son préalable et ce qui manque pour diplômer |
| 9 | **Génération de l'organigramme sous contraintes** : cours réussis, cours voulus, sessions remplies à la main, session à l'étranger ; préalables, offre et plafond respectés ; **placement seul, fait main, toutes les solutions retournées** (bornées par le budget de nœuds) — le solveur ne choisit jamais de cours, l'étudiant fournit la liste et le vérificateur de règles (`core`) valide la sélection et affiche la couverture (ADR `2026-07-b-placement-par-satisfaction-fait-main`, sortie révisée par `2026-07-b-enumere-toutes-les-solutions`) | Fournir ses contraintes et sa liste de cours → tous les organigrammes complets et valides sont énumérés, le premier (le plus proche du cheminement de référence) proposé |
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

Inchangé et toujours contraignant : hypothèse des snapshots par saison, formats de données, grammaires (règles et préalables), conventions défensives du scraper, `cheminement_type` à la main, JSON plutôt que SQLite, pas de navigateur headless.

---

## Questions ouvertes

- Format de l'organigramme JSON et son intégration entre « Cours pour le programme » et l'horaire hebdomadaire (produit par l'un, consommé par l'autre? édité à la main au début?).
- Agencement des écrans (pas nécessairement un écran unique) — à explorer une fois les fonctionnalités gelées.
- Le catalogue est-il joignable sans identifiants depuis le CI? (Détermine la gestion de secrets — à résoudre au jalon 1.)
- Modèle exact de préférences/scoring — structure arrêtée dans `docs/conception/solveur-conception.md` (opérations sur bits + somme pondérée) ; poids et sémantique fine à calibrer contre des données réelles.
- Plafond de crédits par session : dur (17 ?) ou cible molle — aucun chiffre documenté, à confirmer avec le directeur.
- Cadence du cron (hebdomadaire vs quotidien) et canal de notification d'échec.
- Cheminements types d'autres programmes que GEX : qui les fournit, le cas échéant?
- Niveau de couverture des cas particuliers exigé avant livraison (stages, cours multi-sessions, formation à distance, formes de préalables non observées) — principal risque résiduel d'estimation.
