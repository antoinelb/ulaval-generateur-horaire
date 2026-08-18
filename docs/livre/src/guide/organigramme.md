# Organigramme

Deux fonctions répondent à la question « quels cours à quelles sessions, d'ici la fin du bac ? » :

- `generate_organigramme(input)` **construit** : chaque cours du programme (et les cours à option retenus) est placé sur l'horizon de sessions, `pinned` fixant ce que l'étudiant a déjà arrêté.
- `verify_organigramme(input)` **vérifie** le cheminement assemblé par l'étudiant : le placement est prouvé avec tous les cours épinglés, puis les règles du programme sont comptées (`coverage`).

## L'entrée (`OrganigrammeInput`)

```js
const report = generate_organigramme({
  courses,                        // le snapshot des cours
  program,                        // facultatif : le snapshot du programme
  concentration: "Hydrogéologie", // facultatif : titre exact d'une concentration
  profile: null,                  // facultatif : titre exact d'un profil
  electives: ["GGL-2601"],        // cours à option retenus, hors programme inclus
  passed: ["GEX-1000"],           // cours déjà réussis — jamais replacés
  pinned: {"GCI-1001": 2},        // code → numéro de session (base 1)
  start: "fall",                  // saison de la première session
  study_sessions: 8,              // l'alternance automne/hiver seulement
  credit_cap: 17,                 // plafond de crédits par session
  concomitant: false,             // préalables concomitants tolérés ?
  summers_open: false,            // les étés acceptent-ils des cours réguliers ?
});
```

L'horizon n'est jamais listé par l'appelant : il est **décrit** par `start` et `study_sessions`, et le module insère lui-même un été après chaque hiver.
`start: "fall"` avec `study_sessions: 2` donne ainsi `[automne, hiver, été]`.
Les numéros de session (`pinned`, `placement`) indexent cet horizon en base 1.

Un été fermé (`summers_open: false`) n'accepte que les stages et les cours épinglés — épingler est un geste explicite qui lève la restriction.

## La sortie (`OrganigrammeReport`)

```json
{
  "sessions": ["fall", "winter", "summer"],
  "placement": {
    "completion": "complete",
    "solutions": [
      {
        "placement": {"GCI-1001": 2, "GGL-2601": 1},
        "assumed": [],
        "left_out": []
      }
    ],
    "blocked": []
  },
  "set_aside": []
}
```

- `sessions` : l'horizon calculé — les numéros de `placement` n'ont de sens qu'à côté des saisons qu'ils indexent.
- `completion` : `complete` (l'énumération est totale — vide veut alors dire *infaisable prouvé*), `node-budget` ou `solution-cap` (ensemble partiel, jamais « infaisable ») ; voir [Erreurs et budgets](erreurs-et-budgets.md).
- `solutions` : chaque placement faisable trouvé, dans l'ordre de recherche ; les cours réussis n'y figurent pas.
- `solutions[i].assumed` : les opérandes de préalables que le verdict a dû présumer satisfaits (texte libre ou code inconnu du snapshot) — remontés, jamais imposés.
- `solutions[i].left_out` : les cours que ce placement n'a pas pu asseoir. Vide sauf en **repli au mieux** (voir plus bas) — ce qui est placé respecte toujours toutes les contraintes.
- `blocked` : les cours prouvés implaçables *avant* la recherche, avec leur raison (`empty-domain`, `unsatisfiable-prerequisites`, `stage-without-summer`) — une liste non vide est une preuve d'infaisabilité qui nomme ses coupables.
- `set_aside` : les codes exigés par le programme mais absents du snapshot de cours — écartés et remontés, jamais perdus en silence.
- `coverage` : présent seulement pour `verify_organigramme` avec un `program` — le rapport de couverture des règles (voir [Programmes, règles et couverture](../domaine/programmes.md)).

## Le repli au mieux

`generate_organigramme` tente d'abord l'agencement exact.
S'il ne rend rien — un cours bloqué, une infaisabilité prouvée, un budget épuisé — il enchaîne de lui-même sur un **remplissage au mieux** plutôt que de rendre une grille vide (ADR `2026-08-placement-au-mieux-en-repli`).

La règle est : *des trous, jamais une faute.*
Chaque cours placé respecte toutes les contraintes ; ceux qui ne rentrent nulle part sont dans `left_out`, et `blocked` en donne la raison quand le pré-écran les a désignés.
Laisser un cours de côté **cascade** sur ce qui l'exigeait : ses dépendants ne peuvent pas être placés non plus, et se retrouvent dans `left_out` avec lui.

```js
const report = generate_organigramme(input);
const [solution] = report.placement.solutions;
if (solution?.left_out.length) {
  // grille partielle : à compléter à la main
}
```

Le repli rend **une** solution : la sentinelle « pas placé » étant essayée en dernier à chaque profondeur, la première trouvée est le remplissage glouton et les suivantes sont strictement pires.
Son `completion` décrit donc l'énumération relâchée, pas la question « existe-t-il un agencement complet » — cette question-là, c'est la première passe qui y a répondu par la négative.

`verify_organigramme` et `admissible_sessions` ne relâchent **jamais** : prouver reste prouver, et une sonde relâchée déclarerait toute session admissible en sortant tout le reste.

## Vérifier plutôt que construire

`verify_organigramme` exige que chaque cours à placer soit épinglé :

```js
verify_organigramme({...input, pinned: {"GEX-1000": 1}});
// → lève « verification needs a session for every course left to place : … »
// si d'autres cours restent sans session
```

Le placement épinglé est alors *prouvé* (préalables, plafond, étés fermés, faisabilité hebdomadaire) : une solution unique confirme le cheminement, zéro solution avec `completion: "complete"` le réfute.
