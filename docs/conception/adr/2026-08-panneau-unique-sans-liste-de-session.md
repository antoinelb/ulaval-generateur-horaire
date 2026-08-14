# Panneau unique, sans onglets ni liste de session

Date : 2026-08-13

## Contexte

Le panneau livré aux jalons 3–9 avait deux onglets (session affichée / « Bac complet ») et, dans l'onglet session, une liste des cours de la session.
À l'usage (notes d'Antoine, 2026-08-13), les onglets doublaient l'information : les cours de la session se lisent déjà dans l'horaire hebdomadaire et dans le ruban d'organigramme.
Deux défauts s'y ajoutaient : le bouton « où le placer ? » ne produisait rien, et les cours obligatoires n'étaient pas déplaçables depuis le panneau.

## Décisions

- **Un seul panneau** : sélecteur de programme (tant qu'aucun n'est choisi), contrôles d'organigramme, règles en accordéons, recherche du catalogue entier, ajout par code, formulaire de cours manuel. `View.tab` et la liste `SessionCourses` disparaissent.
- **La densité disparaît aussi** (note 1) : un seul réglage d'espacement, « aéré » ; `View.density`, le bouton et `.shell--compact` sont supprimés (les vieux champs persistés déclenchent la note « champ inconnu » une fois, puis la sauvegarde suivante nettoie).
- **Actions unifiées sur chaque rangée** : `+` ajoute à la session affichée ; les puces de sessions placent — ou **déplacent**, obligatoires compris — le cours vers une session admissible (l'édition retire d'abord toute trace de l'ancienne session) ; `✕` retire un cours placé ou ajouté.
- **Rangée « ajouté en … »** : un cours ajouté à la main à une session est traité comme placé (état `Placed`), sinon la recherche offrait de l'ajouter une seconde fois.
- **« Où le placer ? » réparé** — c'étaient deux bogues superposés :
  1. la sonde épinglait tout le `displayed_placement` mais n'envoyait que le cours sondé en électif — chaque code épinglé sans son `Course` faisait échouer `place` (« passed or pinned but has no Course ») ; la sonde embarque désormais les cours étalés, et exclut le cours sondé de ses propres épinglages (sinon la réponse serait « sa session actuelle » et rien d'autre) ;
  2. les erreurs des sondes *fire-and-forget* étaient avalées (`handle_worker_answer` ne montrait que celles de la requête bloquante) ; les sondes en vol sont maintenant suivies (`pending_probes`) et leurs échecs affichés — un bouton muet est un bouton cassé.
- Le filtre « rentre dans l'horaire » reste jugé contre la **session affichée** (le libellé le dit) ; décoché, la recherche couvre tout le catalogue sans filtre de saison.

## Alternatives rejetées

- Liste de session déplacée sous la grille : redondante avec les blocs de la grille eux-mêmes ; « forcer/libérer une section » vit déjà sur les blocs (fantômes), « retirer » vit sur les rangées du panneau.
- Garder les deux onglets : c'est la structure même que l'essai utilisateur a jugée en trop.
