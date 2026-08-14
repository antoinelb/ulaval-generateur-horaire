# Glisser un cours de l'horaire vers une autre session

Date : 2026-08-13

## Contexte

Note 16 d'Antoine (2026-08-13) : on doit pouvoir glisser un cours de la grille hebdomadaire vers une autre session du ruban, en voyant lesquelles peuvent l'accueillir.

## Décision

- Les blocs pleins de la grille sont `draggable` ; `ondragstart` pose le code dans un signal partagé (`DraggedCourse`) et déclenche la sonde `admissible_sessions` si son cache est vide. **Aucune donnée dans `DataTransfer`** — le payload vit dans le signal, même circuit que la sélection des fantômes.
- Pendant le glissement, chaque carte du ruban (étés compris — un cours épinglé peut y atterrir) se marque d'après le cache : admissible → bordure pointillée verte (`--target`), interdite → estompée et **refusant le drop** (`ondragover` sans `prevent_default`), sonde encore en course → neutre (le drop reste permis : l'épinglage est ensuite jugé par la vérification automatique).
- Le drop appelle `place_course` — **le même chemin que les puces** « + H28 » : toute trace de l'ancienne session retirée, puis épinglage + placement, un seul pas annulable (« {code} déplacé vers {label} »).
- Les puces restent l'équivalent clavier complet (INP-4) ; le glissement est un raccourci souris, jamais le seul chemin.

## Alternatives rejetées

- `DataTransfer.setData` : accès verbeux à travers l'abstraction Dioxus pour transporter une chaîne qu'un signal porte déjà, et illisible pendant `dragover` dans plusieurs navigateurs.
- Bloquer le drop tant que la sonde n'a pas répondu : la sonde peut prendre plusieurs secondes ; geler le geste de l'utilisateur pour une validation qui re-tombe de toute façon sur la vérification automatique serait pire que l'accepter.
