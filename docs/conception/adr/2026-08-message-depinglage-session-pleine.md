# Les messages d'échec de placement reçoivent le plan et nomment le geste

## Contexte

Deux inexactitudes rapportées le 2026-08-19 : un cours épinglé à **une** session pleine recevait le message générique « …remplissent déjà **chaque session** où il est offert » (faux — l'étudiante avait choisi une seule session, rapport étudiante-gex) ; et le verdict du panneau « rempli au mieux » était identique qu'il manque 6 cours ou 37 sur 37 (échec total minimisé, rapport étudiante-cegep, B-GMC à 0/120).

## Décision

- `left_out_note(answer)` (agrégat) devient `left_out_line(code, blocked, plan)` (une ligne par cours) + `empty_grid_note()` (l'agrégat du cas « rien placé ») : la note par cours reçoit le `Plan` et distingue trois cas — raison du pré-écran (prime le pin : un pin vers une saison non offerte est un domaine vide), session épinglée qui ne peut pas accueillir, défaut honnête « aucune place ne restait ».
- `apply_proposal` pousse un toast par cours laissé de côté ; le cas « rien placé » garde UN toast agrégé (35 toasts seraient du bruit).
- Le panneau affiche une branche d'échec total (`displayed_placement` et `manual` vides) au ton net, en `panel-verdict--bad`, qui nomme les leviers restants — les étés ne sont plus un levier à suggérer, l'escalade les essaie d'elle-même (ADR `2026-08-escalade-etes-ouverts-dans-le-repli`).
- `blocked_note` sans jargon : « insatisfiables » remplacé par l'action concrète (ajouter le préalable aux cours à option, ou entente avec la direction).

## Alternatives rejetées

- **Garder l'agrégat multi-lignes** : un seul toast pour plusieurs causes ne peut pas se périmer cours par cours (préparait la péremption par cause, ADR `2026-08-peremption-des-toasts-par-cause`).
- **« La session est pleine »** : le singleton épinglé peut aussi échouer par conflit d'horaire ou préalable — le message nomme les trois sans prétendre savoir laquelle.
