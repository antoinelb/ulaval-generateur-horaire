# Horaire hebdomadaire

Deux fonctions répondent à la question « à quoi ressemble ma semaine ? » pour une session donnée :

- `generate_schedule(input)` **construit** : les options déjà choisies (`chosen`) sont épinglées, chaque autre cours prend la première combinaison sans conflit.
- `verify_schedule(input)` **vérifie** : chaque cours demandé doit porter son option choisie, et le rapport juge *cette* combinaison-là.

## L'entrée (`ScheduleInput`)

```js
const report = generate_schedule({
  // le snapshot (ou tout tableau de cours qui contient les codes demandés)
  courses,
  // la session visée : a2026 = automne 2026, h2027 = hiver, e2026 = été
  session: "a2026",
  // les cours voulus ; la casse est normalisée (gex-1000 → GEX-1000)
  codes: ["GEX-1000", "GCI-1001"],
  // facultatif : les options déjà arrêtées, un ensemble de NRC par cours —
  // une option n'a pas d'identifiant propre, on la nomme par ses NRC
  chosen: {"GEX-1000": ["84664", "84665"]},
});
```

Un champ inconnu dans l'objet est refusé (erreur), jamais ignoré : une faute de frappe ne doit pas se lire comme une valeur par défaut.

## La sortie (`ScheduleReport`)

```json
{
  "valid": true,
  "courses": [
    {
      "code": "GEX-1000",
      "selected": [
        {
          "nrc": "84664",
          "section": "A",
          "mode": "in-person",
          "slots": [{"day": "monday", "start": "08:30", "end": "11:20"}]
        }
      ],
      "alternatives": [
        {
          "sections": [{"nrc": "84666", "section": "B", "mode": "in-person", "slots": []}],
          "valid": false
        }
      ]
    }
  ]
}
```

- `valid` (au sommet) : la semaine entière est-elle sans conflit ?
- `courses[i].valid` : **absente quand vraie** ; présente à `false` si la sélection de ce cours chevauche celle d'un autre.
- `selected` : les sections de l'option retenue, embarquées entières — le rapport se suffit, pas besoin de retourner au snapshot pour dessiner la grille.
- `alternatives` : les options non retenues, dans l'ordre du snapshot, chacune avec sa validité *en échange seul* : serait-elle valide si on la substituait, les autres cours ne bougeant pas ?

## Vérifier plutôt que construire

`verify_schedule` prend la même entrée, mais exige un `chosen` complet :

```js
verify_schedule({courses, session: "a2026", codes: ["GEX-1000"], chosen: {}});
// → lève « verification needs a chosen option for every course : GEX-1000 »
```

Un cours sans option choisie est une question incomplète : la fonction lève une erreur plutôt que de rendre un verdict sur une combinaison qu'elle aurait elle-même choisie.
