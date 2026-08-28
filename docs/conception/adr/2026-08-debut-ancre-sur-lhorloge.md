# Le « Début » d'un document neuf est ancré sur l'horloge

Date : 2026-08-27

## Contexte

Rapport étudiante 2026-08-27 (S4) : à l'ouverture d'un programme, « Début » proposait A26 — une session déjà passée — et le sélecteur n'offrait que la fenêtre gelée A24–A31.

`Plan::default().start` est un A26 écrit en dur, hérité par `fresh_plan` et par toute reprise à neuf.
L'horloge réelle n'était lue que par le ruban.

## Décision

Trois fonctions pures dans `state.rs` portent la règle, et les trois chemins qui fabriquent un début l'appliquent.

- `next_admission_semester(today)` : aucun bac n'admet en été (`possible_semester_start` ne lit que « A » et « H »), donc un été renvoie l'automne de la même année civile ; un automne et un hiver sont leur propre session d'admission.
- `floor_start(start, today)` : ne **relève** qu'un début antérieur à cette session, jamais l'inverse.
- `start_year_window(start, today)` : les bornes du sélecteur, en années à deux chiffres — toujours assez larges pour contenir le début réel du plan (un relevé Capsule peut l'ancrer des années en arrière) et cinq ans au-delà de l'horloge.

Les points d'application :

- `fresh_plan(start, choice, study_sessions, today)` plancher le début hérité du document courant.
- `persist::restore_plan(stored, today)` ne re-date **que** le plan exactement égal à `Plan::default()` — première visite, ou sauvegarde illisible repartie à neuf.
- `components/header.rs` `ResetButton` repart sur `next_admission_semester(today)`.
- `components/panel.rs` borne le sélecteur par `start_year_window`.

Un début passé **explicite** reste souverain dans son document : relevé Capsule (`capsule::apply_to_plan` écrit `plan.start` directement), lien partagé (`decode_organigramme` installe le plan entier), réglage manuel du sélecteur.
Aucun de ces chemins ne passe par `fresh_plan`, et `restore_plan` ne touche pas un plan qui porte la moindre autre donnée.

## Alternatives rejetées

- Lire l'horloge dans `Plan::default()` : casserait la comparaison « exactement l'usine » sur laquelle reposent `restore_plan` et une soixantaine de tests, et rendrait le type impur.
- Ne planchérer qu'à la restauration d'étagère : écraserait un choix explicite, celui d'un relevé Capsule compris.
- Garder la fenêtre A24–A31 et n'ancrer que la valeur : le sélecteur cesserait d'offrir les années réellement planifiables dès 2032.

## Conséquences

- L'année de l'horloge devient un paramètre de `fresh_plan`, `restore_plan` et `enter_document` — passé depuis les composants, jamais lu dans les modules purs (AP-7).
