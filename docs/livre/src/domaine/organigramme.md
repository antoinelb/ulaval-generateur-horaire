# Organigramme et cheminement

L'**organigramme** est la grille cours × sessions du programme : quels cours à l'automne 1, à l'hiver 2, et ainsi de suite.
Le **cheminement type** (A1→H8) est l'organigramme de référence publié par la direction — encodé à la main pour le génie des eaux, car aucune source lisible par machine n'existe ; il sert de *germe* (`seed`) pour ordonner la recherche, jamais de contrainte.

## Ce que le placement garantit

Chaque solution retournée respecte :

1. **l'offre** : un cours n'est placé qu'à une saison où il est offert ;
2. **les préalables** : satisfaits avant la session du cours — la même session suffit pour une feuille `concomitant` (l'étoile du répertoire), et `concomitant: true` étend cette tolérance à toutes les feuilles ;
3. **le plafond de crédits** (`credit_cap`) par session, les fourchettes comptant leur borne basse ;
4. **les étés fermés** : sans `summers_open`, un été n'accueille que stages et cours épinglés ;
5. **les stages en été** : un stage non épinglé ne se place qu'en été — l'épingler ailleurs est un geste explicite qui lève la règle ;
6. **la faisabilité hebdomadaire** : les cours d'une même session doivent admettre au moins une combinaison d'options sans conflit d'horaire.

## Les trois issues, jamais confondues

| Issue | Où la lire |
|---|---|
| implaçable prouvé avant recherche | `blocked`, raison nommée |
| infaisable prouvé par recherche complète | `completion: "complete"` et `solutions: []` |
| énumération tronquée par budget | `completion: "node-budget"` ou `"solution-cap"` |

Le germe, les épingles et les cours réussis réduisent l'espace ; `solutions` énumère tout ce qui reste faisable, dans l'ordre de recherche.

Une exception : **`generate_organigramme` avec un `seed` non vide rend une seule solution**, celle qui minimise la somme des distances à ce germe (ADR `2026-08-b-minimise-la-distance-au-seed`).
`max_solutions` est alors ignoré, `completion: "complete"` veut dire « optimum prouvé » et `"node-budget"` « meilleure grille trouvée dans le budget ».
Sans germe, la première grille proposée est équilibrée sur l'horizon plutôt qu'entassée dans les premières sessions (ADR `2026-08-equilibrage-glouton-du-placement-initial`).

## Construire, puis vérifier

Le flux prévu pour une interface :

1. `generate_organigramme` propose un cheminement (première solution) ;
2. l'étudiant déplace des cours — chaque déplacement devient une épingle ;
3. `verify_organigramme` avec tout épinglé prouve le cheminement final et compte les règles (`coverage`).

Ainsi la grille reste toujours éditable, et chaque état affiché est soit prouvé, soit accompagné de la raison exacte de son refus.
