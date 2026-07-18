# Scraper : un seul but, le catalogue complet

Date : 2026-07-18

## Contexte

Le CLI généralisait pour un scrape scopé (URL arbitraire, routage « URL nue vs query string », deux fonctions publiques) alors que le seul cas réel est « scraper tous les cours ».
Le scrape d'une seule matière ne sert qu'aux tests, pour rester rapides.

## Décision

- Une seule fonction publique : `catalogue::scrape(fetcher, base_url)` = le catalogue complet partitionné.
- Le moteur par URL est privé (`scrape_partition`) et muet ; ses tests wiremock vivent en tests unitaires dans le module.
- CLI : `ulaval-scraper catalogue [--output-dir <DIR>] [--url <URL>]` — deux drapeaux optionnels, défauts `data` et `https://www.ulaval.ca/etudes/cours` ; ils n'existent que pour que les tests redirigent la sortie et le site (wiremock) — forme finale via clap, ADR `2026-07-adoption-de-clap`.
- Remplace le routage « URL nue = partitionné, query string = scopé » ; la règle « un scrape scopé n'écrit jamais `data/` » (ADR `2026-07-catalogue-artefact-commite`) devient sans objet : il n'y a plus de scrape scopé.

## Alternatives rejetées

- **Garder une fonction publique scopée** : surface morte, aucun consommateur hors tests.
- **URL codée en dur sans override** : le test e2e du binaire ne pourrait pas viser wiremock.
