# Catalogue : partition du scrape par matière

Date : 2026-07-18

## Contexte

L'index de recherche du site plafonne toute requête à 10 000 résultats : les pages 0–199 servent 50 entrées chacune, la page 200 est vide, alors que la bannière annonce 10 235 cours (vérifié en direct le 2026-07-18).
Le scrape non filtré ne peut donc jamais couvrir le catalogue entier.
Chaque page du catalogue embarque le widget de facettes matières (~174 entrées, `<input name="matieres[N]" value="N">` + `<label>CODE - Nom complet</label>`), et chaque facette reste très en dessous du plafond.

## Décision

- Le scrape du catalogue partitionne toujours par matière : la page 0 non filtrée fournit l'annuaire des matières (`parse_matieres`) ; un widget absent est une dérive de markup = erreur, jamais « zéro matière » ; un checkbox sans libellé ou sans valeur est une anomalie surfacée, jamais ignorée.
- Seule la forme Drupal `matieres%5B113%5D=113` filtre ; la forme plate `matieres=113` est **ignorée silencieusement** par le site et renvoie le catalogue non filtré (vérifié en direct) — un test unitaire épingle la chaîne encodée exacte et les mocks wiremock ne matchent que la forme bracketée.
- Chaque partition passe par le moteur par URL existant (fan-out borné, tolérance des pages vides, réconciliation par partition — ADR `2026-07-pagination-du-catalogue-par-comptage`, `2026-07-tolerance-des-pages-aucun-resultat-du-fan-out`), moteur privé et muet : l'affichage appartient à l'orchestrateur (compteur = matières terminées).
- Concurrence imbriquée `buffer_unordered(4)` sur les matières × `buffer_unordered(4)` par partition, sous l'unique `Fetcher` partagé : la limite de débit (~10 req/s) reste structurellement globale (ADR `2026-07-conception-du-fetcher`).
- La fusion est l'union dédupliquée par code (`Catalogue::from_entries` fait foi) ; le statut de cette union face à la bannière relève de l'ADR `2026-07-le-catalogue-est-lunion-des-facettes`.
- Si le total affiché tient déjà sur la page 0, aucune partition : la page est complète telle quelle.
- Ceci réintroduit l'orchestration multi-URL retirée comme spéculative par l'ADR `2026-07-cli-dans-la-lib-et-style-derreurs` : elle a maintenant un consommateur réel.

## Alternatives rejetées

- **Se contenter des 10 000 premiers résultats** : perte silencieuse du reste du catalogue.
- **Partitionner par cycle** : cinq partitions seulement, mais la dimension est trouée elle aussi (somme des facettes cycle : 10 207 sur 10 235).
- **Somme des totaux par partition comme vérification** : le chevauchement des facettes la rend invalide ; seule la déduplication par code compare des comptes comparables.
