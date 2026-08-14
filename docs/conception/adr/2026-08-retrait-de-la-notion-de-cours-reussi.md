# Retrait de la notion de « cours réussi » de l'interface

Date : 2026-08-13

## Contexte

L'interface livrée aux jalons 3–9 offrait « marquer réussi » (✓/↩) sur chaque cours, persisté dans `Plan.passed` et transmis tel quel à `PlacementRequest.passed`.
En essayant l'application, Antoine a constaté que cette notion n'apporte rien à l'organigramme : c'est le fait qu'un cours est placé dans une session antérieure qui permet aux sessions suivantes d'assumer le préalable — le double marquage « réussi » ne fait que dupliquer l'information et ajouter des clics.

## Décision

- La notion disparaît **entièrement de l'interface** : plus de boutons ✓/↩, plus de `Plan.passed`, plus d'état de rangée « réussi ».
  Un cours complété se place dans la session (passée) où il a été suivi, et se déplace s'il doit être refait.
- Le style « session passée » du ruban est désormais **dérivé de la date réelle** (`semester_of_epoch_ms` + `semester_precedes` dans `ui::state`) : toute session strictement antérieure au semestre du jour est grisée — purement visuel, aucun effet sur le solveur.
- **`core` garde `PlacementRequest.passed`.** Analyse faite pendant l'implémentation : pour les cours réguliers, `passed` est bien redondant avec un placement en session antérieure ; mais c'est le seul mécanisme qui retire du placement des cours acquis **sans leur faire occuper une session ni des crédits de l'horizon** — exactement ce qu'exige la scolarité préparatoire « faite » (`core::intake::course_list` met les 0xxx de la règle en tête de liste à placer, et seul `passed` les en retire).
- L'unique alimentation de `passed` devient donc `Plan.preparatory_done` (défaut : `true`, la case « scolarité préparatoire faite ») : cochée → `PlaceQuery.passed` reçoit les codes de la règle « Scolarité préparatoire » du programme choisi ; décochée → liste vide et les 0xxx redeviennent du travail à placer.
  Côté couverture, `panel_model` ajoute ces mêmes codes à la sélection quand la case est cochée.

## Alternatives rejetées

- **Retirer `passed` aussi de core** (chirurgie du programme : retrancher la règle préparatoire du `Program` envoyé au solveur) : gros churn — wasm, protocole worker, cinq familles de fixtures `organigrammes/`, référence Python — pour aboutir à la même sémantique déplacée hors de core, où elle appartient (« aucune règle métier dans la vue »).
- **Sessions passées à bascule manuelle** : un clic de plus et un état à persister pour une information que le calendrier donne déjà ; la correction reste possible en déplaçant les cours.

## Conséquences

- Les vieilles sauvegardes portant `passed` déclenchent une note unique de `restore` (champ inconnu) et repartent proprement.
- Un étudiant en cours de bac exprime ses acquis en plaçant ses cours faits dans les sessions passées de l'horizon — l'horizon couvre toujours le début du bac.
