# Les verdicts du panneau se regroupent sans hauteur réservée

Date : 2026-08-27

## Contexte

L'audit LAY-2 (V2) a montré que les paragraphes de verdict du bloc organigramme (`OrganigrammeControls`) étaient des `if`/`match` conditionnels rendus directement dans `div.panel-organigramme`, juste avant la liste des règles.
`left_out`, `verification` et `credit_shortfalls` sont écrits par la réponse du worker (`mod.rs`), déclenchée seule 500 ms après la dernière saisie par `auto_propose`/`auto_verify` — aucun geste de l'étudiant ne les accompagne.
Chaque paragraphe qui apparaît ou disparaît décale donc la liste des règles rendue en dessous.
Un premier correctif regroupait ces paragraphes dans un `div.panel-verdicts` toujours rendu **avec** un `min-height: 4.125rem` réservant trois lignes ; Antoine a retiré la réserve.

## Décision

Exception LAY-2 assumée, décidée par Antoine : les paragraphes de verdict (conflit d'horaire, plafond dépassé, manques de crédits, readiness du placement, verdict de vérification et notes bloquées) restent regroupés dans un `div.panel-verdicts` unique, toujours rendu au même endroit avant la liste des règles, mais **sans hauteur réservée**.
Le panneau est une colonne défilante : son contenu bouge déjà sous le doigt de l'étudiant au gré du défilement, la présomption « rien ne bouge » y est structurellement plus faible que dans la grille ou le ruban, et une réserve vide y gaspillerait la hauteur la plus disputée de l'interface.
Le regroupement garde sa valeur propre : les verdicts apparaissent toujours au même endroit, jamais éparpillés entre les règles, et le retrait des manques de crédits injectés dans les rangées (V5, ADR `2026-08-manques-de-credits-hors-des-rangees`) reste entier.

## Alternatives rejetées

- **Réserver la hauteur du chevauchement typique** (`min-height: 4.125rem`, le premier correctif) : dans un panneau qui défile de toute façon, la réserve coûte de la hauteur en permanence pour une invariance que le défilement ne garantit déjà pas.
- **Laisser les verdicts éparpillés en conditionnels dans `panel-organigramme`** : perdrait le regroupement, qui à lui seul rend les apparitions prévisibles — l'étudiant sait où regarder.
