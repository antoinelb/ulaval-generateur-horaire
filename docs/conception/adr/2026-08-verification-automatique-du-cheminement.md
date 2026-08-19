# Vérification automatique du cheminement

Date : 2026-08-13

## Contexte

« Vérifier le cheminement » était un bouton (jalon 8). Note 6 d'Antoine (2026-08-13) : la vérification doit se faire d'elle-même à chaque changement — un verdict qu'on doit demander est un verdict qu'on oublie de demander.

## Décisions

- Le bouton disparaît. Un effet débouncé (500 ms, motif compteur-génération) envoie `verify` dès que : le snapshot est chargé, un programme est choisi, le solveur est libre, aucun verdict frais n'existe, et **chaque cours demandé a une session** (`solve::unplaced_codes`, qui réutilise `core::placement_intake` — pas de logique dupliquée). L'effet converge : la requête pose `running`, la réponse pose `verification`, les deux le stoppent ; toute édition du plan efface le verdict et relance le cycle.
- Quand des cours flottent encore, le panneau l'explique passivement (« N cours sans session — … la vérification se relancera d'elle-même ») ; une entrée illisible (code fautif) s'affiche comme « Vérification impossible : … ».
- **La couverture des règles quitte le protocole worker** : `Response.Report.coverage`, ainsi que `PlaceQuery.concentration`/`profile` qui ne servaient qu'à elle, sont supprimés. Le verdict « règles à combler » vient du `coverage_report` local du panneau (déjà calculé pour les badges, ententes et préparatoire comprises) — une seule source de vérité au lieu de deux comptages parallèles.
- « Proposer un organigramme » et « Chercher plus longtemps » restent des boutons : ce sont des actions qui changent le document, pas des lectures. *(Supersédé le 2026-08-19 : les deux boutons disparaissent, le placement tourne en continu — ADR `2026-08-organigramme-en-continu-sans-bouton`.)*

## Alternatives rejetées

- Vérifier à chaque frappe sans débounce : une rafale d'éditions lancerait autant de requêtes ; le worker n'en traite qu'une à la fois et la file ne dirait rien de plus.
- Garder la couverture dans le worker : le panneau la calculait déjà localement — deux comptages pouvaient diverger (ententes appliquées d'un côté seulement).
- Envoyer verify même incomplet et afficher l'erreur anglaise du worker : bruyant à chaque édition et contraire à ERR-7 (français d'abord).

## Conséquences

- Annuler une vérification en cours la fera repartir une fois le solveur relancé — anodin, elle est bornée par le petit budget ; l'annulation vise la proposition longue.
