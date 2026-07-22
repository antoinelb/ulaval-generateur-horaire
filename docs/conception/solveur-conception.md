# Le solveur — conception et fondements théoriques

**Date :** 2026-07-21
**Statut :** conception ; répond au « modèle exact de préférences/scoring » et au sens de « génération sous contraintes » laissés ouverts dans `docs/project_plan.md`.
Révision du jour même : l'embranchement de la §5.2 est **fermé** — satisfaction, placement seul, fait main (ADR `2026-07-b-placement-par-satisfaction-fait-main`) ; voir la Résolution en §5.2.
**Portée :** ce document consolide toute l'analyse du solveur — classification théorique, expression mathématique des contraintes, taxonomie des algorithmes, conception des deux problèmes, et la décision d'outillage pour B.
Il étend et, pour le moteur de B, **révise** la décision D5 de la conception initiale (`docs/conception/initial/ADR.md`) : celle-ci fixait un moteur écrit à la main ; ce document ouvre explicitement le choix entre ce moteur à la main et un solveur PPC déclaratif, selon que l'objectif d'apprentissage de Rust s'applique ou non à B (révision actée dans l'ADR `2026-07-moteur-de-b-embranchement-pumpkin`).
**S'appuie sur :** `2026-07-sections-en-combinaisons-valides`, `2026-07-contrainte-de-regle-optionnelle`, `2026-07-operande-non-verifiable-gardee-en-texte`, `2026-07-prealables-hors-grammaire-en-enum`, `2026-07-credits-variables-en-enum`.

---

## 1. Les deux problèmes, formellement

Le cœur résout **deux** problèmes que le plan présentait comme un seul (« le solveur ») : l'**horaire hebdomadaire** d'une session (A) et l'**organigramme** A1→H8 d'un cheminement (B).
Ils partagent un squelette de résolution mais ne sont pas le même problème — la variable et la ressource en contention diffèrent toutes deux.

### 1.1 Problème A — l'horaire hebdomadaire

**Classe.** Problème de satisfaction de contraintes (CSP) à domaines finis.
Plus précisément, c'est le sous-problème que la littérature de *timetabling* appelle **student sectioning** : l'horaire institutionnel est déjà figé, on choisit une option (combinaison d'inscription) par cours de sorte qu'aucune plage ne chevauche.
C'est l'ombre facile du problème institutionnel (affecter des milliers d'activités à des salles et créneaux pour tous les étudiants et enseignants), qui, lui, occupe les chercheurs depuis les années 1970.

**Expression mathématique.** Cours `i ∈ {1,…,n}`, chacun une variable `x_i` sur son domaine `D_i` (ses options).
Un univers temporel `𝒯` discrétisé (seaux de 5 minutes) ; chaque option `o` porte un ensemble occupé `occ(o) ⊆ 𝒯`.
Seule contrainte dure, la disjonction temporelle deux à deux :

```
∀ i < j :  occ(x_i) ∩ occ(x_j) = ∅
```

Comme CSP binaire, les contraintes sont les relations de compatibilité `R_ij = { (a,b) ∈ D_i × D_j : occ(a) ∩ occ(b) = ∅ }`.

**Objet théorique.** Le graphe de **microstructure** : un sommet par couple `(i,o)`, une arête entre `(i,a)` et `(j,b)` quand `i ≠ j` et qu'ils sont compatibles.
Une solution est exactement une **clique de taille n** (un sommet par cours, tous compatibles deux à deux) — de façon équivalente, un transversal indépendant du graphe de conflit complémentaire.
Le test de recouvrement sur bitset ne fait que calculer ces arêtes à la volée.

**Complexité.** NP-complet dans le cas général : on réduit la `k`-coloration de graphe en posant un cours par sommet avec `k` options (les couleurs) et en rendant incompatibles les options de même couleur sur des sommets adjacents ; une coloration propre équivaut à une sélection valide.
Mais la complexité paramétrée par la taille de domaine `d = max_i |D_i|` est `O(d^n)`, et les données réelles vivent dans le coin trivial : en a2026 (4438 cours), la **moyenne** est de 1,21 option par cours, 90,3 % des cours n'ont qu'une seule option, le maximum observé est 20, et `n ≈ 5` cours par session.
**L'énumération n'est pas de la paresse : c'est la méthode indiquée quand la microstructure est presque une union disjointe de cliques.**

**Cas infaisable.** Quand aucune sélection valide n'existe, « surligner le moins de plages en conflit » est un **Max-CSP / CSP pondéré** : trouver l'affectation qui minimise le nombre (ou le poids) de `R_ij` violées.
C'est l'objet formel derrière le surlignage minimal des conflits.
Attention à la forme réduite « paires de cours incompatibles » : elle est incomplète, voir §7.

### 1.2 Problème B — l'organigramme

**Classe.** NP-difficile, et trois problèmes classiques empilés.
Sous la surface c'est un **Resource-Constrained Project Scheduling Problem (RCPSP) avec activités optionnelles et précédence ET/OU** — ou, dans la littérature propre à l'enseignement, de la **planification de cheminement** (*curriculum / degree planning*).
Les trois couches :

- **Sélection** (quels cours à option existent) — couverture d'ensemble / sac à dos issu des règles « X crédits parmi ».
- **Précédence ET/OU** — les arbres de préalables ne sont pas un ordre partiel simple : l'admissibilité de chaque cours est une **formule booléenne monotone** sur des littéraux « placé plus tôt ».
  Le RCPSP standard n'a qu'une précédence ET ; c'est là le twist non standard.
- **Affectation aux périodes sous sac à dos par période** — la capacité en crédits par session est la ressource renouvelable ; l'offre par saison est une restriction unaire de domaine.

**Révision (2026-07-21).** La couche de sélection est **retirée de B** par décision produit : B place une liste de cours *donnée* (possiblement partielle — tronc seul, ou base du directeur) ; le choix des cours à option revient à l'étudiant, validé par le vérificateur de règles (§5.1).
B se réduit à l'affectation sous précédence ET/OU et sac à dos par période — la classe reste NP-difficile, l'instance devient encore plus petite (ADR `2026-07-b-placement-par-satisfaction-fait-main`).
La formulation ci-dessous garde `y_c` pour montrer la structure complète ; dans le modèle retenu, `y_c` est une constante d'entrée, pas une variable.

**Expression mathématique (comme PLNE, pour *voir* la structure).** Avec `y_c ∈ {0,1}` = cours `c` sélectionné, `x_{c,t} ∈ {0,1}` = placé en session `t ∈ {1,…,8}`, et `period(c) := Σ_t t · x_{c,t}` :

```
Σ_t x_{c,t} = y_c                                  (placé ssi sélectionné)
x_{c,t} = 0   si season(t) ∉ offered(c)             (offre par saison)
y_c = 1       ∀ c ∈ M                               (obligatoires)
ℓ_r ≤ Σ_{c ∈ L_r} cr(c) · y_c ≤ u_r                (règle de crédits r)
Σ_{c ∈ L_r} y_c = n_r                               (règle de comptage r)
Σ_c cr(c) · x_{c,t} ≤ cap    ∀ t                    (sac à dos par session)
Σ_c cr(c) · y_c ≥ 120                               (crédits exigés)
period(c) − period(p) ≥ 1   quand y_c = 1           (préalable simple p → c)
```

Les préalables ET/OU sont là où la forme linéaire s'alourdit : une variable de satisfaction par nœud de l'arbre, un nœud OU exige `Σ (enfants satisfaits) ≥ 1`, chaque feuille réifiant « `q` sélectionné ET `period(q) < period(c)` ».
Objectif mou : minimiser le déséquilibre de charge entre sessions, ou le makespan, ou un coût de préférences.

**Complexité.** NP-difficile par le sac à dos (les lignes de capacité), par l'ordonnancement sous précédence, ou par la sélection de sous-ensemble à elle seule.
Contrairement à A, l'instance de B n'échappe pas trivialement à sa classe — elle est seulement **petite**.
Pour le bac GEX : 34 cours obligatoires, ~35 cours listés par les règles comptables (la règle wildcard « tout cours de premier cycle » étant résolue par l'étudiant, voir §5.1), 8 sessions, 120 crédits exigés — une sélection finale d'environ 40 cours, tronc largement fixé — donc les méthodes exactes terminent instantanément.

### 1.3 La relation entre A et B : composition, pas identité

Décomposés en primitives, A et B diffèrent sur les trois axes :

| | Horaire (A) | Organigramme (B) |
|---|---|---|
| Variable | quelle `options[i]` par cours | quelle session par cours |
| Domaine | la liste d'options du cours | les sessions dont la saison correspond à l'offre |
| Ressource en contention | le temps à la minute, dans une semaine | les crédits par session, et l'ordre |
| Contraintes | recouvrement d'intervalles | précédence ET/OU, remplissage, offre, sélection |

On ne passe pas de l'un à l'autre en activant une contrainte : la variable elle-même change (section vs session) et la ressource change (temps de la semaine vs crédits + ordre).
Ce qu'ils partagent est le **squelette** — affecter, élaguer, classer — pas le problème.

La relation exacte est la **composition** : B décide quels cours dans quelle session ; pour chaque session, A décide s'il existe un horaire hebdomadaire sans conflit et à quel point le meilleur est bon.
Le temps appartient à A seul.
L'hypothèse de réutilisation des snapshots par saison a une conséquence load-bearing : **toute** session du cheminement, proche ou à deux ans, dispose d'un horaire exploitable (une session future réutilise le plus récent de la même saison), donc A s'applique aux huit sessions, pas seulement à la prochaine.

A entre dans B à **deux sévérités** : un booléen « existe-t-il une combinaison valide » qui **veto** la faisabilité, et, pour les seuls survivants, une qualité du meilleur horaire qui **contribue au classement**.
Seule la seconde entre dans le score additif ; la première est un veto — sans quoi un plan élégant surclasserait une session où l'étudiant ne peut physiquement pas s'inscrire.

---

## 2. La composition B→A porte un nom : décomposition de Benders logique

La structure « un problème externe affecte des items à des segments de temps, un problème interne vérifie la faisabilité par segment » est la **décomposition de Benders logique (LBBD)**, introduite par Hooker et Ottosson (2003), qui généralise Benders classique pour admettre un sous-problème combinatoire dont l'inférence spécifique fournit les coupes.
Son application canonique en ordonnancement est presque mot pour mot la nôtre : le problème maître affecte des tâches à des segments de l'horizon, le sous-problème les ordonnance ; les frontières de segment y représentent des arrêts (fins de semaine).
Chez nous : B (maître) affecte les cours aux sessions, A (sous-problème) vérifie l'horaire hebdomadaire de chaque session.
L'intérêt du procédé est que combiner PLNE et PPC ainsi résout des problèmes de planification et d'ordonnancement bien plus vite que l'une ou l'autre méthode seule.

Le pipeline par rejet est la variante **branch-and-check** (le maître n'est résolu qu'une fois, chaque solution maître faisable déclenchant l'évaluation du sous-problème) dans sa forme **dégénérée** : le LBBD complet, à sous-problème infaisable, émet une **coupe de faisabilité** (un nogood « ne plus jamais mettre cet ensemble ensemble ») qui élague plusieurs solutions maîtres à la fois ; on ne réinjecte pas de coupe pour piloter l'énumération de B, on énumère les maîtres structurellement valides et on rejette les échecs.
La mémoïsation sur `(ensemble de cours du terme, saison)` est une version *faible* du contenu de la coupe : on met en cache le fait « cet ensemble est infaisable dans cette saison » sans en faire une coupe qui élague la recherche.

**Positionnement honnête :** branch-and-check LBBD, sans coupe pilotant le maître, plus mémoïsation du sous-problème.
Chemin de montée en charge, si l'espace maître de B dépassait un jour l'énumérable : ajouter de vraies coupes de faisabilité — pas une réécriture.

---

## 3. Taxonomie des algorithmes utilisés pour ces problèmes

Le fil historique compte pour ce projet : un directeur de recherche a résolu un problème de ce genre en **Prolog** dans les années 1990.
C'est du **CLP(FD)** — *constraint logic programming over finite domains* — dont le timetabling fut une application phare : le système CHIP (Dincbas et al., 1988), les travaux de Van Hentenryck (1989), plus tard ECLiPSe et SICStus.
L'ancêtre probable de ce travail est Guéret, Jussien, Boizumault et Prins, « Building university timetables using constraint logic programming » (PATAT 1996).
CLP(FD) = Prolog + moteur de propagation-et-retour-arrière à domaines finis ; « Prolog pour l'ordonnancement dans les années 90 » **est** l'approche PPC, et c'est la lignée directe de l'option PPC moderne (Pumpkin, cspsolver).
Elle n'est pas obsolète : elle est l'ancêtre.

**Méthodes exactes / complètes**

- **PPC / CLP(FD)** — propagation par cohérence d'arc/bornes, contraintes globales : `cumulative` *est* la contrainte de ressource du RCPSP, `alldifferent`/`disjunctive` gèrent le non-recouvrement.
  Home naturel de la précédence ET/OU via contraintes réifiées.
  La lignée Prolog.
- **PLNE (programmation linéaire en nombres entiers)** — la formulation de la §1.2, résolue par *branch-and-bound* / *branch-and-cut*, ou *branch-and-price* pour le timetabling à grande échelle.
- **Coloration de graphe** — le plus vieux modèle théorique (de Werra, années 1980) : créneaux = couleurs, conflits = arêtes ; heuristiques DSATUR et plus-grand-degré pour la construction.
  La lentille classique sur A.
- **SAT / MaxSAT / SMT**, et — l'héritier moderne du fil Prolog — **ASP (Answer Set Programming, clingo)**, dont les règles de choix et contraintes de cardinalité/poids expriment « N crédits parmi » presque littéralement.
- **Flots / couplage** pour les sous-structures polynomiales (affecter des étudiants à des sections d'un même cours est un problème de transport), et **programmation dynamique** pour les DAG de précédence de faible largeur et les sous-problèmes de sac à dos.

**Méthodes incomplètes / (méta)heuristiques** — dominantes sur les grandes instances institutionnelles et les compétitions ITC

- **Métaheuristiques** — recuit simulé (historiquement la bête de somme du timetabling d'examens), recherche tabou, algorithmes génétiques/mémétiques, GRASP, recherche à voisinage large (LNS), colonies de fourmis, et **hyper-heuristiques** (tout un programme de recherche né des compétitions).
- **Construction + réparation** — heuristique de coloration puis réparation locale.

**Hybrides / matheuristiques** — **PPC+PLNE**, **LBBD / branch-and-check** (§2), **LNS avec sous-solveur PPC ou PLNE**.

**Application aux deux problèmes.** Pour A, presque tout ceci est surdimensionné — la microstructure est triviale, tout algorithme complet dégénère en énumération et les métaheuristiques sont inutiles quand la réponse exacte est gratuite.
Pour B, les fits exacts naturels sont PPC/CLP(FD), PLNE, ASP ou LBBD ; les métaheuristiques ne se justifient qu'à l'échelle de centaines de cours, ce qu'un baccalauréat n'atteint jamais.

---

## 4. Problème A — la conception (arrêtée)

**Construction du domaine, une fois, avant toute recherche.** Elle absorbe trois subtilités du modèle pour que la recherche n'ait plus à y penser :

- **Équivalences** — un cours à `equivalents` : retenir l'offre de la plus récente des deux saisons.
- **Section forcée (un NRC)** — un NRC peut figurer dans plusieurs options (alternatives partageant un séminaire commun ; test `one_nrc_may_appear_in_several_options`).
  Forcer un NRC n'est *pas* « choisir l'option k » : c'est restreindre le domaine du cours aux options dont l'ensemble de sections contient ce NRC.
- **Crédits en intervalle** — un stage `Credits::Range` (`2026-07-credits-variables-en-enum`) exige la pondération choisie par l'étudiant en entrée ; aucun total n'a de sens sans elle.

**Le temps comme bitset fixe.** Les minutes des données réelles sont toutes multiples de 5 (vérifié : 0 contre-exemple sur les 3975 plages d'a2026), donc des seaux de 5 minutes sont exacts.
Une semaine complète = 7 × 288 = 2016 seaux → `[u64; 32]` (256 octets).
On précalcule un bitset par option (union des plages de ses sections) ; une option entièrement à distance est le bitset vide.
Recouvrement = ET mot à mot, sans branchement :

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
struct WeekMask([u64; 32]);

impl WeekMask {
    fn overlaps(&self, other: &WeekMask) -> bool {
        self.0.iter().zip(&other.0).any(|(a, b)| a & b != 0)
    }
    fn merge(&self, other: &WeekMask) -> WeekMask { /* OU mot à mot */ }
}
```

Chaque préférence conservée devient une opération sur bits de la semaine fusionnée : matins libres = masque avant midi vide ; pause dîner = un seau de la fenêtre médiane libre ; journées compactes = peu de transitions occupé→libre→occupé par jour.
(La distance entre pavillons sort de la portée — aucun champ de localisation requis sur `Slot`.)

**La recherche** est un produit incrémental élagué : on part de l'affectation vide et, cours par cours, on étend chaque préfixe valide par chacune de ses options compatibles ; l'espace étant minuscule, on collecte **toutes** les feuilles valides (le classement en a besoin).
Un `fold` sur les cours satisfait à la fois « pas de boucle `while` » (règle du projet) et « pas de récursion » (règle NASA du CLAUDE.md global) — le DFS récursif esquissé initialement violait la seconde.

```rust
fn enumerate(courses: &[Vec<Opt>]) -> Vec<Schedule> {
    let seed = vec![(WeekMask::EMPTY, Vec::new())];
    courses
        .iter()
        .fold(seed, |partials, options| {
            partials
                .iter()
                .flat_map(|(acc, chosen)| {
                    options.iter().enumerate().filter_map(|(k, opt)| {
                        (!acc.overlaps(&opt.mask)).then(|| {
                            let mut next = chosen.clone();
                            next.push(k);
                            (acc.merge(&opt.mask), next)
                        })
                    })
                })
                .collect()
        })
        .into_iter()
        .map(|(_, chosen)| Schedule::from(chosen))
        .collect()
}
```

A expose deux points d'entrée — B en a besoin aux deux sévérités :

```rust
fn is_feasible(courses: &[Vec<Opt>]) -> bool;                        // le veto
fn best_schedule(courses: &[Vec<Opt>]) -> Option<(Schedule, Score)>; // le score
```

`is_feasible` court-circuite (`try_fold` : arrêt dès que l'ensemble des préfixes se vide) plutôt que de payer la collecte complète — c'est le chemin chaud du veto de B, même mémoïsé.
Quand `is_feasible` est faux, A rapporte les plages en conflit pour le surlignage « aucune combinaison valide » du plan (forme complète en §1.1 et §7).

---

## 5. Problème B — la conception et la décision d'outillage

### 5.1 Le substrat commun (indépendant du moteur choisi)

Quel que soit le moteur retenu en §5.2, ces pièces sont les mêmes et se construisent d'abord :

- **Modèle du domaine** — cours candidats (obligatoires ∪ options des règles), sessions `{1,…,8}`, saison de chaque session, crédits (fixes ou intervalle), plafond de crédits.
- **Vérificateur de règles** — depuis la révision du 2026-07-21, c'est l'**API produit** du jalon 8, pas un auxiliaire du solveur : étant donné une sélection, il rend un rapport **satisfait / à combler / candidats** par règle, consommé par l'UI (cœur pur — l'invariant interdit cette logique dans la vue).
  `Constraint::Count{n}` et `Constraint::Credits{min,max}` se vérifient directement ; obligatoires manquants signalés ; les candidats pour combler une règle sont sa liste de cours filtrée par `weekly::is_feasible` contre l'horaire ouvert de la session visée (A suffit — aucun cheminement à vérifier).
  `RuleCourses::{Reference, Keyword, Raw}` (`Keyword::{Any, Negotiated}`) et toute règle à `constraint: None` **remontées à l'étudiant, jamais inventées** (`2026-07-contrainte-de-regle-optionnelle`, `2026-07-regles-negociees-reconnues`).
  Concrètement, la règle 5 du bac GEX est `Keyword::Any` (« tous les cours de premier cycle, à l'exception des cours correctifs de français ») : son domaine est le catalogue entier (~4438 cours en a2026), donc elle est résolue par un choix de l'étudiant, jamais énumérée.
  Aucune règle GEX n'a `constraint: None` — ce chemin se teste sur fixture synthétique.
  L'énumération des ensembles satisfaisants n'existe plus nulle part : B ne sélectionne pas.
- **Contraintes structurelles** — offre par saison ; précédence en parcourant le `PrereqTree` (`All` = chaque enfant strictement avant ; `Any` = au moins un avant ; `ProgramCredits` = crédits accumulés avant ≥ seuil ; opérandes `Raw` remontées, jamais imposées — `2026-07-operande-non-verifiable-gardee-en-texte`) ; capacité de crédits par session ; couverture des règles.
- **Contraintes de l'utilisateur** — cours réussis (forcés, crédits précomptés), cours voulus (forcés, p. ex. suivre un cours avec un ami), sessions remplies à la main (singletons), session à l'étranger (puits de crédits au domaine lâche).
  Ce sont des **réductions de domaine à des singletons**, d'où « synthèse à partir de zéro *ou* avec des cours fixés » par un seul mécanisme.
- **A comme oracle mémoïsé** — la faisabilité d'une session est une fonction pure de `(ensemble de cours, saison)`, mise en cache ; le tronc obligatoire bougeant à peine, chaque session distincte se calcule une fois.
- **L'objectif** — sans objet depuis la révision du 2026-07-21 : B est de la satisfaction, et l'ordre de recherche (session du seed d'abord) tient lieu de « qualité » du premier placement.
  Le classement par préférences reste l'affaire de A (jalon 10, opérations sur bits de §4).

### 5.2 La décision d'outillage — un embranchement selon un objectif

Le choix du moteur de B dépend d'une question de produit et d'une question d'objectif : **B est-il un problème de *satisfaction* (trouver un bon cheminement faisable) ou d'*optimisation* (prouver le meilleur sous un objectif) ?** et **l'objectif « apprendre Rust en profondeur » s'applique-t-il à B ?**

**Résolution (2026-07-21).** Satisfaction, tranché en produit ; et la sélection est retirée de B (§1.2).
Par les critères mêmes de cette section, l'embranchement se ferme sur la **Piste 1** : le coût documenté de la Piste 1 (« n'optimise que sur ce qu'il énumère ») cesse d'exister quand on ne demande plus d'optimiser, et sans `selected[c]` le modèle PPC n'a plus d'avantage structurel.
Le spike est annulé ; l'analyse Pumpkin ci-dessous est conservée comme **repli documenté** (les faits WASM vérifiés le 2026-07-21 restent valables si la recherche main venait à thrasher).
Une exigence remonte avec la satisfaction : la recherche de la Piste 1 doit être **complète**, car « aucun cheminement faisable » devient une affirmation forte qu'une génération par perturbations seule ne pourrait pas prouver.
ADR : `2026-07-b-placement-par-satisfaction-fait-main`.
**Révision (2026-07-21, même jour)** : la recherche complète étant payée de toute façon (le cas infaisable visite tout l'espace élagué), B retourne **toutes** les solutions trouvées — bornées par le budget de nœuds, ensemble complet/partiel/vide jamais confondus — plutôt que la première ; ADR `2026-07-b-enumere-toutes-les-solutions`.

**Piste 1 — moteur à la main (si l'apprentissage de Rust s'applique à B).**
Générer-et-tester par étapes en `core` pur : entonnoir de rejets bon marché d'abord (offre, précédence, capacité, couverture), A en veto mémoïsé en dernier, score additif, classement des N meilleurs.
**Rejet plutôt que réparation** : un candidat qui échoue est écarté, jamais corrigé sur place.
Justification du rejet — sa latence est plate et prévisible sur tout l'espace d'entrée, y compris la queue sur-contrainte où la réparation *thrashe* (décale, A refuse, décale encore, retour arrière) ; une exigence « au plus quelques secondes » est une exigence sur le pire cas.
Révisé avec la sélection retirée (2026-07-21) : la recherche est une affectation systématique **complète** cours par cours, ordre de valeurs = session du `cheminement_type` de référence d'abord puis sessions voisines (sans seed : plus tôt offerte d'abord), élaguée par les filtres structurels, budget de nœuds explicite.
Le premier placement trouvé ressemble ainsi au cheminement de référence ; une recherche épuisée **prouve** « aucun cheminement faisable », un budget atteint rapporte « budget épuisé » — jamais confondus.
- *Force* : zéro dépendance, préserve l'invariant cœur pur, la plus riche surface d'apprentissage Rust (itérateurs, traits, filtrage sur les enums).
- *Coût* : plus de code que déléguer à un solveur ; pas d'optimisation vraie — sans objet depuis que la cible est la satisfaction.

**Piste 2 — solveur PPC déclaratif (si la qualité prime et l'apprentissage ne s'applique pas à B) — non retenue (voir Résolution), conservée comme repli documenté.**
Modéliser B comme un **problème d'optimisation sous contraintes (COP)** et le résoudre avec un solveur PPC à *lazy clause generation*, en mode **anytime** (budget → meilleur incumbent = « qualité sans optimalité »).
PPC plutôt que PLNE parce que la structure de B se mappe sur les globales PPC directement : `cumulative` pour la capacité en crédits par session, `element`/table pour l'offre, cardinalité/linéaire pour les règles, booléens réifiés pour la précédence ET/OU — là où la PLNE force des encodages *big-M* à régler à la main.

Outil sous les contraintes réelles (navigateur/WASM, sans backend, invariant « toute la logique en `core` pur ») : **Pumpkin**, par la crate **`pumpkin-core`** (0.4.0, MIT/Apache-2.0, active) — et **non** l'ombrelle `pumpkin-solver`, qui tire `signal-hook` et une build-dep `cc` pour son CLI et n'est pas propre pour le WASM.
Rust pur ; fait de l'optimisation, pas seulement de la satisfaction : `Solver::optimise(brancher, termination, resolver, procedure)`, `OptimisationDirection::{Minimise,Maximise}`, procédures `LinearSatUnsat`/`LinearUnsatSat` ; embarque exactement les globales voulues — `cumulative`, `element`, (in)égalités linéaires, `maximum`, `all_different`, `table`, plus `.reify(lit)`/`.implied_by(lit)` génériques sur toute contrainte (la précédence ET/OU s'encode directement) ; solveur *lazy clause generation* (la variante PPC qui emprunte l'apprentissage de clauses au SAT, la plus forte précisément en ordonnancement) ; backend MiniZinc/FlatZinc (on prototype le modèle en MiniZinc avant d'écrire une ligne de Rust).
Raison décisive : c'est une *crate* Rust, donc elle vit **dans** `core` et compile encore natif + WASM — un solveur est du calcul, pas de l'IO, l'invariant tient.

Le mode *anytime* est natif : une `TerminationCondition` (`TimeBudget::starting_now`) borne la recherche, un callback reçoit chaque incumbent, et `OptimisationResult::Satisfiable` porte la meilleure solution connue quand le budget expire avant la preuve d'optimalité.
L'incrémental aussi : après un solve, le solveur revient à la racine et accepte de nouvelles contraintes/clauses sur la même instance (les clauses ajoutées persistent) — la boucle branch-and-check à nogoods n'exige pas de reconstruire le modèle.
Bonus non encore exploité : `satisfy_under_assumptions` extrait un **unsat core**, une piste directe pour expliquer « pourquoi aucun cheminement » (le jalon 8 veut « ce qui manque pour diplômer »).

Esquisse du modèle Pumpkin de B :

```
period[c] : IntVar sur offered_sessions(c)          // domaine = sessions offertes
selected[c] : BoolVar                                // sélection (règles à option)
// précédence simple :        post( period[p] #< period[c] ) réifié par selected[c]
// précédence OU :            post( or(child_before...) ) via booléens réifiés
// capacité par session t :   cumulative / pack sur { credits[c] } par period[c] ≤ cap
// offre :                    intrinsèque au domaine de period[c]
// règles de crédits :        ℓ_r ≤ Σ_{c∈L_r} credits[c]·selected[c] ≤ u_r
// objectif :                 minimise( imbalance + pref_cost )   // anytime
solver.optimise(&mut brancher, &mut TimeBudget::starting_now(budget), &mut resolver,
                LinearSatUnsat::new(Minimise, objective, on_incumbent))
```

- *Force* : plus haute qualité (optimise sur tout l'espace via propagation + branch-and-bound, pas seulement sur des candidats énumérés) ; moins de code (on écrit des contraintes, pas une recherche) ; même vitesse à cette échelle ; l'*anytime* donne exactement « rapide, de qualité, pas optimal ».
- *Coût* : dépendance pré-1.0 (Pumpkin est jeune — mais le modèle de B est petit et isolé dans un module, un changement de solveur est peu coûteux ; épingler `pumpkin-core` 0.4.0).
  **Le risque WASM est largement levé en amont (vérifié 2026-07-21)** : `pumpkin-core` est testé en CI sur `wasm32-unknown-unknown` (`wasm-pack test`), embarque son propre shim d'horloge (`web-time`), configure `getrandom` en backend `wasm_js` et n'utilise aucun thread.
  Reste à confirmer au spike que *notre* modèle passe `wasm-pack` avec notre graphe de dépendances, et la qualité/latence anytime sur le bac GEX réel.

**Garder A comme oracle dans les deux pistes.** Utiliser Pumpkin pour B ne veut **pas** dire dissoudre A dans un modèle monolithique (huit copies des contraintes de temps couplées au maître).
On garde la structure branch-and-check : le solveur optimise l'affectation de B, le bitset de A vérifie la faisabilité hebdomadaire par session sur les incumbents, et — l'infaisabilité de A étant rare (cinq cours par session, 90 % à option unique) — on ajoute un nogood et on réoptimise seulement quand ça mord.
Un nogood issu d'un veto de A vaut pour **toutes** les sessions de la même saison, pas seulement la session fautive : l'hypothèse des snapshots par saison rend la faisabilité fonction de `(ensemble, saison)` seulement.
Le solveur remplace la recherche de B ; A reste le contrôle bitset rapide.

**Pourquoi « pas nécessairement optimal » est la bonne cible, pas un compromis.** (1) L'objectif est une somme pondérée de préférences molles aux poids estimés — « optimal » contre un vecteur de poids fabriqué est une fausse précision.
(2) Les entrées pathologiques (très épinglées, quasi infaisables) sont exactement là où *prouver* l'optimalité traîne même sur une petite instance ; un *cutoff anytime* borne la latence.
(3) L'UI réoptimise interactivement à chaque bascule de contrainte.
L'optimisation *anytime* répond aux trois.

### 5.3 L'alternative connue mais non retenue sous les contraintes

**ASP / clingo** (via une compilation de clingo en WASM) est le modèle le plus *expressif* pour B — règles de choix et contraintes de cardinalité/poids transcrivent les règles presque littéralement, `#minimize` donne l'optimisation *anytime*, et c'est un solveur éprouvé et l'héritier vivant de la lignée CLP(FD)/Prolog.
Mais c'est un module C-compilé-en-WASM avec frontière JS, **pas** une crate Rust : il vit *hors* de `core`, dans `ui`, ce qui **brise l'invariant « toute la logique métier en `core` pur »** et alourdit le bundle.
Il gagne sur la maturité et l'expressivité du solveur, perd sur l'architecture.
À réserver au cas où le build WASM ou la maturité de Pumpkin poseraient un vrai problème.

**good_lp + microlp** (PLNE Rust pur, WASM) : léger en dépendance et compile sûrement en WASM, mais `microlp` est un *branch-and-bound* faible et la PLNE est un moins bon fit (big-M pour ET/OU).
Plancher acceptable, pas le choix qualité.

**L'optimum sans contrainte, pour l'honnêteté** : si « sans backend » tombait un jour, OR-Tools **CP-SAT** (ou tout solveur PPC/PLNE fort derrière une fonction serverless) est le meilleur en catégorie — *anytime*, primé, résout 40×8 quasi optimalement en millisecondes.
Pas de quoi casser l'architecture statique pour un problème de cette taille, mais c'est la catégorie que le crate `server` réservé anticipe ; « B veut un vrai solveur » n'est pas un déclencheur assez fort ici.

---

## 6. Analyse des crates — l'axe décisif est WASM

Le solveur tourne dans le navigateur ; Rust-pur-contre-FFI est la contrainte liante.
Tout ce qui exige une chaîne C/C++ (OR-Tools, HiGHS, CBC, SCIP, CPLEX, Kissat, CaDiCaL) est de facto exclu (builds Emscripten pénibles, bundle gonflé).

| Crate | Type | WASM | Rôle ici |
|---|---|---|---|
| *(fait main dans `core`)* | énumération par produit élagué + bitset ; recherche de placement complète | ✅ natif | **Retenu pour A et pour B** (placement seul, satisfaction — ADR `2026-07-b-placement-par-satisfaction-fait-main`). Zéro dépendance. |
| `pumpkin-core` (Pumpkin) | PPC *lazy clause generation*, optimisation | ✅ testé en CI amont sur `wasm32-unknown-unknown` (éviter l'ombrelle `pumpkin-solver` : `signal-hook` + `cc`) | **Repli documenté pour B** si la recherche main venait à thrasher. `cumulative`/`element`/linéaire/`maximum`, backend MiniZinc, orienté ordonnancement. |
| `good_lp` + `microlp` | PLNE, front-end + backend Rust pur | ✅ | Repli PLNE pour B ; solveur faible, encodage ET/OU maladroit. |
| `rustsat` + `rustsat-batsat` | SAT/MaxSAT, MiniSat en Rust pur | ✅ (« bon pour WebAssembly ») | Si voie SAT/MaxSAT pour B ; encodage CNF le plus coûteux. |
| clingo-wasm | ASP compilé en WASM | ✅ (module JS) | Le plus expressif ; **brise l'invariant cœur pur** (hors `core`). |
| `cp_sat`, `highs`, `russcip` | FFI vers OR-Tools/HiGHS/SCIP | ❌ | Rapides en natif, inutilisables au navigateur ; pertinents seulement si un `server` se matérialise. |

Frère de lignée à surveiller : **Huub** (CP 2025), autre solveur LCG en Rust avec backend MiniZinc — le repli naturel si Pumpkin fléchissait.
À éviter : `cassowary` (algorithme de mise en page *linéaire continue*, pas combinatoire) ; la crate `metaheuristics` (GPLv3, incompatible licence).

---

## 7. Défauts connus du modèle et décisions ouvertes

- **Aucun champ de localisation** sur `Slot`/`Section` — sans objet depuis que la distance entre pavillons sort de la portée.
  Si elle y rentrait, il faudrait un champ pavillon sur `Slot` et une table de distances.
- **Modes à distance/hybride** (a2026 : 861 sections à distance, 385 hybrides ; année 2026 entière : 2219 et 855 ; 2858 des 5369 options d'a2026 n'ont aucune plage) ont des plages vides ou partielles : ils ne conflitent jamais et « rentrent » toujours.
  Correct, mais le score ne doit pas compter une semaine tout-à-distance comme « maximalement compacte » par accident — le champ `mode` des sections existe déjà pour le distinguer.
- **Rapport de conflit par paires incomplet** : un ensemble peut être infaisable alors que toutes ses paires de cours sont compatibles (la cohérence deux à deux n'implique pas la cohérence globale — une solution est une clique, pas un tas d'arêtes).
  Rare avec 90 % de cours à option unique, mais le test du rapport doit couvrir ce cas ; la forme Max-CSP de §1.1 le couvre naturellement.
- **Plafond de crédits par session** : dur (17 ?) ou cible molle pénalisée à l'écart par le score ?
  Le chiffre n'apparaît dans aucun document existant — à confirmer avec le directeur avant de l'encoder.
- **Pondération des crédits en intervalle** : l'étudiant choisit une valeur par stage — requise d'emblée, ou par défaut à la borne basse pour la planification ?
- **Poids de l'objectif** de B — à calibrer contre des données réelles.
- **Interaction règles × profils** : la §5.1 vérifie les règles seules ; les profils ajoutent une couche à cadrer — désormais côté vérificateur/affichage, plus côté solveur (le jalon 6 introduit les profils).
- ~~Satisfaction vs optimisation pour B~~ — **tranché le 2026-07-21** : satisfaction, placement seul, fait main (§5.2 Résolution ; ADR `2026-07-b-placement-par-satisfaction-fait-main`).

---

## 8. Note historique

L'approche Prolog/CLP(FD) du directeur de recherche n'est pas un détour pittoresque : c'est l'ancêtre direct de l'option PPC recommandée pour B, et clingo/ASP en est l'héritier le plus vivant.
Si l'objectif premier du projet était la qualité de résolution plutôt que l'apprentissage de Rust, un modèle CLP(FD) ou ASP de B serait un choix parfaitement défendable — sans doute supérieur.
La vraie raison de faire main en Rust est **les objectifs du projet** (Rust en profondeur, livraison WASM, zéro dépendance d'exécution, cœur pur) plus le fait que les instances vivent dans le coin trivial — pas une supériorité de résolution.
Il faut le dire clairement pour que la décision reste lucide.
