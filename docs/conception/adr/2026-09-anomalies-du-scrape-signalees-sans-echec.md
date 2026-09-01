# Les anomalies du scrape sont signalées, elles ne font plus échouer le job

**Date :** 2026-09-01
**Statut :** accepté (décision Antoine). Renverse la dernière puce de l'ADR `2026-08-ci-et-publication-sur-github-pages`.

## Contexte

Le job `scrape` sortait en échec après le commit dès qu'un `data/*_errors.log` existait.
L'échec n'était pas un échec : le snapshot produit est valide (écritures atomiques, garde ≥ 90 % du catalogue) et poussé, le déploiement est déclenché, tout a fonctionné.
Le rouge servait uniquement de canal de notification — GitHub envoie un courriel au propriétaire sur un run en échec.

Le prix : un run rouge à chaque scrape porteur d'anomalies, alors que les anomalies sont le régime **normal** du scraper.
Les ADR `2026-07-echec-de-page-cours-non-bloquant` et `2026-07-echec-de-page-programme-non-bloquant` établissent qu'une page illisible est une anomalie et jamais un arrêt ; un premier scrape complet en produit légitimement beaucoup.
Un signal qui est presque toujours allumé n'est plus un signal, et il rend l'historique des runs illisible : impossible de distinguer d'un coup d'œil un scrape qui a échoué (fetch impossible, push rejeté trois fois) d'un scrape qui a réussi avec de la prose hors grammaire.

## Décision

L'étape devient « Report the scraper anomalies » et ne sort jamais en erreur :

- le contenu des `data/*_errors.log` est ajouté au **résumé du run** (`$GITHUB_STEP_SUMMARY`), rendu en tête de la page du run, pas enfoui dans la sortie d'une étape ;
- une annotation `::warning::` marque le run en jaune dans la liste des Actions, ce qui garde un scrape avec anomalies distinguable d'un scrape propre.

Le rouge redevient ce qu'il devrait être : le scrape n'a pas produit ou pas publié de snapshot.

Conséquence acceptée : **plus de courriel**. GitHub ne notifie que sur échec ; l'annotation et le résumé se consultent, ils ne se poussent pas.
Les anomalies restent lues au moment où on les cherche — après un changement de grammaire, ou en ouvrant le dernier run.

## Alternatives rejetées

- **Un seuil d'anomalies au-delà duquel on échoue** : déjà rejeté par l'ADR `2026-07-echec-de-page-cours-non-bloquant` — nombre arbitraire, et le premier scrape complet dépasse n'importe quel seuil raisonnable.
- **Échouer seulement sur une anomalie d'un type nouveau** : demande de mémoriser l'état antérieur du journal entre deux runs, pour une distinction que le résumé rend déjà visible en le lisant.
- **Canal de notification dédié** (webhook, courriel custom) : un secret et un service externe pour un signal qu'on veut désormais moins bruyant, pas plus.
