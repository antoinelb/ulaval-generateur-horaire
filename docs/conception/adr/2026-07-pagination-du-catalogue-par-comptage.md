# Catalogue : pagination par comptage et fan-out parallèle

Date : 2026-07-17

## Contexte

Le scrape du catalogue était conçu séquentiellement : il fallait parser la page N pour savoir si la page N+1 existait, la terminaison étant la page vide avec preuve de forme (ADR `2026-07-page-aucun-resultat-et-total-optionnel`).
Cette dépendance borne le débit réel à max(100 ms, latence du site) par page — soit ~3–6 req/s sous le throttle de 10 req/s.
Or la page 0 contient déjà tout ce qu'il faut pour calculer l'espace d'adressage : le total affiché (`total_results`) et la taille de page (son propre nombre d'entrées).

## Décision

Deux étapes : **découverte, puis fan-out**.

- Page 0 → `total` et `page_size`; nombre de pages = ⌈total / page_size⌉.
- Les pages 1..N sont récupérées en parallèle (`buffer_unordered`) sous le throttle partagé — le même motif d'orchestration que la future phase cours.
- La terminaison par page-sentinelle est remplacée par une **réconciliation arithmétique**, vérifiée après fusion :
  1. le compte fusionné (avant déduplication) doit égaler le `total` de la page 0, sinon erreur qui arrête le run;
  2. chaque page doit afficher le même `total` que la page 0 (mutation du site en cours de scrape ou dérive → erreur);
  3. plafond dur sur le nombre de pages calculé (1000, contre ~205 réelles) — borne d'itération explicite;
  4. page 0 courte (moins d'entrées que `page_size` impliqué) alors que `total` en annonce davantage = contradiction = erreur, jamais d'inférence.
- Cas limites : `total ≤ page_size` → une seule page, aucun fan-out; variante « Aucun résultat » (`total_results: None`) → catalogue vide.
- Le parseur continue de reconnaître les formes « page vide » et « Aucun résultat » (fixtures `gex_2`, `all_last` inchangées) : ce sont des formes de page valides, plus un signal de terminaison.

La réconciliation est plus forte que la sentinelle : la sentinelle prouvait qu'on avait atteint la fin, jamais que les pages du milieu étaient complètes.
Le gain de vitesse (~20 s au lieu d'une minute et plus) est un effet secondaire, pas la motivation.

## Alternatives rejetées

- **Pagination séquentielle jusqu'à la page vide (décision précédente)** : débit borné par la latence, boucle de contrôle différente de la phase cours, et garantie plus faible (rien ne vérifie la complétude des pages intermédiaires).
- **Fan-out sans réconciliation** : une erreur de calcul du nombre de pages tronquerait silencieusement la fin du catalogue — violerait « jamais de perte silencieuse ».
