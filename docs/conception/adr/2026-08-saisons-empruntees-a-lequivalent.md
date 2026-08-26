# Les saisons inventées d'un cours neuf sont empruntées à son équivalent daté

## Contexte

Une page du répertoire sans aucune section de session est un cours dont l'horaire n'est pas encore publié.
L'ADR `2026-07-cours-sans-section-de-session-offert-automne-hiver` lui donne alors automne + hiver, millésime et horaire inconnus — une hypothèse de repli, pas un fait lu sur la page.

GEX-4002 « Hydrologie opérationnelle » est dans ce cas, et il a un équivalent, GEX-7002, que le répertoire date : hiver 2026, vendredi 08:30–11:20.
Le mécanisme d'équivalence (`resolve_offering`, ADR `2026-07-equivalences-par-millesime-de-session`) lui prêtait déjà cet horaire — mais saison par saison, et seulement pour une saison déjà présente dans son propre `seasons`.
Résultat : GEX-4002 affichait le bon horaire à l'hiver, et restait proposé à l'automne avec un masque vide, alors que le seul calendrier connu de la paire dit « hiver seulement ».

Le chemin placement aggravait l'écart : `select_known` clone le cours entier sans jamais consulter les équivalents, tandis que la grille hebdomadaire passe par `select_courses` → `effective_course`, qui les consulte.
Les deux solveurs ne lisaient pas le même calendrier.

## Décision

Une passe sur le catalogue complet, `borrow_seasons_from_equivalents`, appliquée juste après la fusion (`merge_manual`) dans les deux surfaces — `crates/ui/src/data.rs` et `crates/wasm/src/boundary.rs` — comme `apply_prereq_overrides` l'est déjà.

Un cours dont **toutes** les saisons portent la forme inventée (`last_offered: null` et `options: null`) et dont au moins un équivalent du catalogue porte une saison **datée** voit son jeu de saisons remplacé par l'union des saisons datées de ses équivalents.
Chaque saison empruntée garde le repli du cours lui-même (`SeasonOffering::UNPUBLISHED`) : seule la liste des saisons change, jamais l'horaire, qui continue d'être résolu contre les équivalents au moment de la requête.

Trois garde-fous :

- un `seasons` **vide** n'est pas la forme inventée — c'est un fait sur le cours, et on ne lui ajoute jamais de saison ;
- une seule saison datée suffit à prouver que le calendrier a été lu : le cours est laissé intact ;
- si aucun équivalent n'est daté, l'hypothèse automne + hiver reste la seule réponse disponible.

La passe est idempotente : un cours réécrit satisfait encore la garde et recalcule le même ensemble.

## Conséquences

Les deux solveurs lisent désormais le même calendrier, sans que la signature d'aucun d'eux bouge.
La correction s'applique aussi aux cours de `cours.manuel.json` et aux cours importés localement, puisqu'elle agit sur le catalogue fusionné plutôt que dans le scraper.
Le fichier `data/cours.json` reste fidèle aux pages : rien n'y est réécrit, et une régénération des données n'est pas nécessaire.

Sur le snapshot du 2026-08-26, 321 cours portent un calendrier inventé et 8 voient leurs saisons changer — la paire premier cycle / deuxième cycle du même cours (`COM-4160`, `CSO-4030`, `ECN-4900`, `GCI-4004`, `GEX-4002`, `MUS-4069`, `SIO-1002`, `SOC-6110`).
Six se resserrent, deux s'élargissent : `ECN-4900` et `SIO-1002` gagnent l'été, que leur équivalent daté offre.
L'union élargit donc autant qu'elle resserre, et c'est voulu : si l'hypothèse automne + hiver ne porte aucune information, elle n'en porte pas davantage dans un sens que dans l'autre, et n'appliquer que le resserrement laisserait `ECN-4900` faux à l'envers.

## Alternatives rejetées

**Corriger dans le scraper**, en écrivant les saisons de l'équivalent dans `data/cours.json`.
Le snapshot cesserait alors de dire ce que la page dit, la correction n'atteindrait ni le catalogue manuel ni les imports locaux, et il faudrait re-scraper pour en bénéficier.

**Corriger dans `effective_course`** seulement.
Ne touche que la grille hebdomadaire ; le placement, qui passe par `select_known`, continuerait de proposer l'automne.

**Emprunter aussi l'horaire, saison par saison, dans `select_known`.**
Duplique la logique d'équivalence dans un troisième endroit au lieu de normaliser le catalogue une fois.

**Ne rien faire et signaler l'écart à l'écran.**
Une note « cours neuf, calendrier supposé » sur chaque session concernée fait porter à l'étudiant un arbitrage que les données tranchent déjà.
