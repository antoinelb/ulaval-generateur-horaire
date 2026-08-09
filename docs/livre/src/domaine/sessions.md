# Sessions et saisons

## Nommage

Une session se nomme saison + année :

| Forme | Exemple | Sens |
|---|---|---|
| minuscule + année pleine | `a2026`, `h2027`, `e2026` | la session demandée aux fonctions (`session`) |
| lettre majuscule + deux chiffres | `A26`, `H27` | un millésime (`Semester`) — version d'un programme |
| mot anglais | `"fall"`, `"winter"`, `"summer"` | la valeur `Season` dans les données et les rapports |

`a` = automne, `h` = hiver, `e` = été ; l'année d'un `h` est l'année civile de l'hiver (`h2027` suit `a2026`).

## L'hypothèse fondatrice

Une session future n'a pas d'horaire publié.
Le générateur suppose qu'elle réutilisera **l'offre la plus récente de la même saison** : demander `a2026` sert, pour chaque cours, son offre d'automne du millésime `last_offered`.
C'est une hypothèse de planification, pas une promesse — l'horaire réel peut différer à la publication.

Un cours dont la page ne montre aucune section de session est un cours nouveau : il est gardé comme offert automne et hiver, `last_offered` et `options` à `null`.

## L'horizon d'un organigramme

L'appelant décrit l'horizon (`start` + `study_sessions`, l'alternance automne/hiver), et le module y insère un été après chaque hiver.
Les étés sont fermés aux cours réguliers par défaut : seuls les stages et les cours épinglés s'y placent, `summers_open: true` les ouvrant tous.
