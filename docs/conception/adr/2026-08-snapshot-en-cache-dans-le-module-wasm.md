# Le catalogue chargé une fois : `courses` optionnel, snapshot en cache

## Contexte

Les huit fonctions de la frontière JavaScript prenaient le catalogue **dans chaque appel**.
Le frontend l'expédiait donc à chaque question (`js/verifications.js`, `js/horaire.js`, `js/generation.js` : `courses: catalogue`) : 8 834 cours clonés par `postMessage`, puis désérialisés par `serde_wasm_bindgen::from_value`.
Le worker de l'app Dioxus, lui, gardait déjà son snapshot (`init_snapshot`) — c'était même la raison invoquée pour ne pas réutiliser le paquet (ADR `2026-08-crate-ui-calculations-et-worker`).
La fusion des deux crates (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`) met ce cache à portée des deux consommateurs.

## Mesures

Paquet réel (`--target nodejs`), catalogue réel de 8 834 cours, B-GEX A26, 11 sessions d'étude, moyenne de 5 appels :

| Appel | `courses` dans l'appel | catalogue en cache | écart |
| --- | --- | --- | --- |
| `coverage_report` | 67,3 ms | **0,4 ms** | 67 ms (×168) |
| `generate_schedule` (5 cours) | 66,1 ms | **0,26 ms** | 66 ms (×254) |
| `admissible_sessions` (`CHM-1903`) | 829,5 ms | 763,2 ms | 66 ms (8 %) |

Le coût du transport est **constant, ~66 ms par appel**, quelle que soit la question.
Pour les fonctions courtes il *est* la totalité du temps : `generate_schedule` passait 99,6 % de sa durée à recevoir un catalogue qu'elle relit à chaque fois.
Une passe de vérification de la grille du B-GEX (un `coverage_report` plus un `generate_schedule` par session) portait ainsi ~790 ms de marshalling ; en cache, ~3 ms.

Ce que la mesure **ne** règle pas : l'item 3 de `CORRECTIFS-AMONT` (`admissible_sessions` trop lente pour un affichage par ligne).
Les 66 ms gagnés ne pèsent que 8 % de ses 830 ms — la recherche domine, et l'hypothèse « c'est le marshalling » est fausse pour cette fonction-là.
L'item reste ouvert et appelle un profilage du solveur, pas de la frontière.

## Décision

- `courses` devient **optionnel** (`Option<Vec<Course>>`) dans `OrganigrammeInput`, `ScheduleInput` et `CoverageInput`.
  Les appels actuels continuent de passer inchangés : le frontend adopte le cache quand il veut, sans fenêtre de déploiement à coordonner.
  `PrerequisitesInput` ne porte qu'un `Course` et `HorizonInput` aucun : rien à y changer.
- `init_snapshot(snapshot_json, manual_json)` remplit un `thread_local!` partagé par les deux surfaces, et répond le nombre de cours retenus plus les sigles manuels éclipsés.
  Il prend des **chaînes JSON**, pas des objets JS : construire le graphe d'objets serait précisément le coût qu'on supprime.
- La résolution est unique et testée (`catalogue::resolve`) : l'appel l'emporte sur le cache, une liste explicitement vide reste celle de l'appelant, et **ni l'un ni l'autre est une erreur** — jamais un catalogue vide, qui répondrait « rien n'est plaçable » à toute question.
  Côté protocole du worker, ce refus prend l'id réservé 0, comme une requête illisible.

## Alternatives rejetées

- **Rendre `courses` obligatoirement absent** (le cache seul) : cassait le frontend déployé au moment même de la publication du paquet, pour un gain nul — l'`Option` suffit.
- **Passer le catalogue en objet JS à `init_snapshot`** : le coût qu'on cherche à supprimer est justement la traversée de ce graphe ; la chaîne JSON se parse une fois côté Rust.
- **Un cache par question (mémoïsation sur `courses`)** : il faudrait comparer 8 834 cours pour décider du hit — plus cher que le hit.
