# Erreurs et budgets

## Les erreurs sont des chaînes levées

Une entrée que le module refuse de deviner lève une exception dont la valeur est une **chaîne** décrivant le problème, coupable nommé :

```js
try {
  generate_schedule({courses, session: "a2026", codes: ["ZZZ-9999"]});
} catch (e) {
  console.error(e); // "ZZZ-9999 names no course of the snapshot" (par exemple)
}
```

Lèvent une erreur, entre autres : un champ inconnu dans l'objet d'entrée (faute de frappe), une session malformée, un code absent du snapshot, un cours non offert à la saison demandée, une option épinglée qui ne correspond à aucune option du cours, un numéro de session hors horizon, une concentration ou un profil inexistant.

La règle générale : **une question incomplète ou incohérente est une erreur, jamais un faux verdict.**
`verify_schedule` sans option choisie pour chaque cours, ou `verify_organigramme` avec un cours laissé sans session, lève — un `valid: false` serait un mensonge sur une question qui n'a pas été posée.

## Ce qui n'est *pas* une erreur

Certaines situations sont des réponses, pas des refus :

- un cheminement infaisable → `completion: "complete"` avec `solutions: []`, et `blocked` nomme les cours prouvés implaçables — mais `generate_organigramme` ne s'y arrête pas : il enchaîne sur un [remplissage au mieux](organigramme.md#le-repli-au-mieux), dont `solutions[0].left_out` dit ce qu'il a laissé de côté ;
- un code exigé par le programme mais absent du snapshot → `set_aside` ;
- un opérande de préalable invérifiable → `assumed` sur la solution ;
- un conflit d'horaire → `valid: false` dans le rapport.

## Les budgets de l'organigramme

La recherche de placements s'exécute sur le fil JavaScript : les budgets par défaut arrêtent l'énumération bien avant de geler l'onglet.

| Champ | Défaut | Rôle |
|---|---|---|
| `max_nodes` | 1 000 000 | borne de travail : assignations partielles explorées |
| `max_solutions` | 100 | borne de mémoire : solutions retournées |

La troncature n'est jamais silencieuse — `completion` dit quelle borne a été touchée :

- `"complete"` : l'énumération est totale ; `solutions` est *l'ensemble* des placements faisables, et vide veut dire infaisable **prouvé** ;
- `"node-budget"` : budget de travail épuisé ; l'ensemble est partiel — on ne peut pas conclure à l'infaisabilité ;
- `"solution-cap"` : plafond de solutions atteint ; l'ensemble est partiel, il en existe peut-être d'autres.

Un appelant qui sait ce qu'il fait (un worker, un test) passe des budgets plus grands :

```js
generate_organigramme({...input, max_nodes: 10_000_000, max_solutions: 1000});
```
