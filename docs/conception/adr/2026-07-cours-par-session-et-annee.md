# Un snapshot par session, l'année portée par `CoursePage`

Date : 2026-07-19

## Contexte

`data/cours/{session}.json` est nommé par saison **et** année (`a2026`), mais `core::Course` est indexé par saison seule (`fall`/`winter`/`summer`).
`parse_seasons` lisait pourtant l'année — « Automne 2026 – » — pour ne garder que le bloc le plus récent de chaque saison, puis la jetait à la dernière ligne.

Les pages portent des sessions historiques : GCI-1007 affiche Automne 2024, 2025 et 2026, et un cours retiré de l'offre garde son dernier Automne, quelle qu'en soit l'année.
Une même page alimente donc jusqu'à trois sessions à la fois (ECN-4901 : Hiver 2026 et Été 2026).

## Décision

- `parse_seasons` retourne `BTreeMap<Season, (u16, SeasonOffering)>` et `parse` répartit le résultat : les offres dans `Course.seasons`, les années dans un nouveau champ `CoursePage.years`.
  L'année qualifie le snapshot, pas le cours : la mettre dans `Course` aurait changé les six fixtures `courses/*.json`, qui restent identiques au bit près.
- Un fichier par couple (saison, année) **effectivement rencontré**, sans horloge ni liste blanche.
  Un cours dont le dernier Automne est 2022 produit `a2022.json` — l'information est vraie, et c'est exactement l'hypothèse fondatrice : une session sans horaire publié réutilise le plus récent snapshot de la même saison.
- Chaque snapshot vaut `{"courses": [Course]}`, comme le catalogue, et chaque `Course` y est projeté sur la seule saison que le fichier concerne — le nom du fichier porte déjà la session.
- La sérialisation passe par un type `SessionSnapshot` et non par un littéral `json!` : `serde_json::Value` est un `BTreeMap`, qui alphabétiserait les clés.
  Ces snapshots sont commités, et l'ordre de déclaration (`code`, `title`, `credits`, …) garde les diffs lisibles et alignés sur les fixtures.
- Les cours de chaque snapshot sont **triés par code**, comme les entrées du catalogue et pour la même raison.
  `buffer_unordered` rend les cours dans leur ordre d'achèvement, que les aléas du réseau rendent arbitraire : sans tri, deux scrapes identiques produisaient deux fichiers différents, donc un diff git énorme et vide de sens à chaque passage du cron.
  Vérifié en relançant le scrape : les snapshots sont identiques au bit près.

## Alternatives rejetées

- **Liste blanche `--sessions a2026 h2026`** : demande d'être tenue à jour à chaque rentrée, pour ne rien apporter que le nom du fichier ne dise déjà.
- **Heuristique « année maximale par saison »** : une seule section d'Automne 2027 publiée en avance ferait disparaître tout l'Automne 2026.
- **Un seul `data/cours.json` portant les `Course` entiers** : repousse le découpage, mais oblige l'UI à trier les saisons elle-même alors que le plan la veut servie par session.
- **Ajouter l'année dans `core::Course`** : contamine le type métier avec une préoccupation de snapshot et invalide les fixtures.

## Plafond connu

Un scrape restreint par `--subjects` écrase le snapshot complet de la session, puisque l'écriture est un remplacement atomique du fichier entier.
C'est acceptable tant que le livrable du jalon 1 *est* un snapshot GEX ; la fusion avec l'existant devient nécessaire le jour où le cron voudra des scrapes incrémentaux.
