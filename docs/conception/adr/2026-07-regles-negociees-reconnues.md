# Règles « négociées » reconnues plutôt que signalées

Date : 2026-07-21

## Contexte

Trois règles décrivent un contenu réel mais sans liste de cours énumérable — les cours sont convenus avec la direction ou renvoyés à une autre structure.
Toutes trois tombaient en `RuleCourses::Raw` et levaient une anomalie à chaque scrape :

- génie physique, un profil : « Le profil est satisfait par la réussite des cours convenus entre la direction de programme et l'étudiante ou l'étudiant. »
- génie industriel : « Réussir les cours requis par sa concentration. »
- génie mécanique, profil « Passage intégré au deuxième cycle » : la page fragmente une phrase entre l'entête de l'accordéon (« Règle 1 – Réussir la scolarité de ») et son corps (« deuxième cycle suivante : »), d'où **deux** anomalies — contrainte illisible et prose hors grammaire.

Ce sont des cas connus et valides, pas des ratés du parseur.
Les laisser en anomalie, c'est du bruit permanent dans `data/programmes_errors.log` : le journal doit signaler ce qui demande une attention humaine, pas ce qu'on a décidé d'accepter.

## Décision

1. Nouvelle valeur reconnue de la grammaire : `Keyword::Negotiated`, sérialisée `{"courses": "negotiated", "raw": "…"}`.
   La variante `RuleCourses::Any` devient `RuleCourses::Keyword { courses: Keyword, raw }` (JSON inchangé pour `any`) : deux variantes `{courses, raw}` distinctes ne se départageraient pas en `#[serde(untagged)]`, donc l'enum `Keyword` porte désormais `Any` et `Negotiated`.
2. `classify_prose` reconnaît, **avant** le repli `Raw` + anomalie, les tournures négociées et renvoie `Keyword::Negotiated` sans anomalie :
   « convenus entre la direction », « requis par sa concentration », « deuxième cycle suivante » / « scolarité de deuxième cycle ».
   `raw` garde la phrase entière — reconnu n'est pas interprété.
3. Une entête dont la fente contrainte échoue à `parse_constraint`, **ne contient aucun chiffre et n'est pas « Un cours »**, est de la prose (« Réussir la scolarité de »), pas une dérive de markup : `constraint: None`, sans anomalie.
   Toute vraie contrainte numérique contient un chiffre ; « Un cours » est la seule sans chiffre, et elle est déjà filtrée.

## Alternatives rejetées

- **Les laisser en anomalie** : bruit permanent qui noie la seule anomalie qu'on veut voir — « 3 blocks under no heading » du génie mécanique, un garde-fou délibéré (ADR `2026-07-blocs-de-la-page-programme`).
- **Une variante `RuleCourses::Negotiated { raw }` distincte** : entrerait en collision avec `Raw { raw }` en `untagged` (même forme `{raw}`) ; réutiliser le patron `Keyword` discrimine par la valeur de `courses`.
- **Résoudre « requis par sa concentration » en référence de cours** : la cible est la concentration entière, pas une règle nommée comme le veut `RuleReference` ; l'y forcer inventerait une structure que la page ne donne pas.
