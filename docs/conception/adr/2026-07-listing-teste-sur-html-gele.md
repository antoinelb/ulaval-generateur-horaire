# Listing : tests du parseur sur HTML gelé

Date : 2026-07-17

## Contexte

Deux décisions antérieures faisaient du listing une exception : ses tests rouleraient contre le HTML frais (e2e en direct sur la facette GEX), pour qu'une dérive du markup fasse échouer les tests visiblement (`2026-07-cas-de-test-listing-facette-gex.md`, `2026-07-catalogue-artefact-commite.md`).
Les pages cours et programmes, elles, se testent sur HTML gelé.
Au moment d'implémenter le parseur, l'exception coûte : des tests dépendants du réseau (lents, non déterministes, impossibles hors ligne), et un rythme de développement différent pour un seul des trois types de pages.

## Décision

Le listing se teste comme les autres pages : **sur HTML gelé**.

- Trois pages de la facette GEX gelées dans `tests/fixtures/html/listing/` : `gex-page-0.html` (50 cours), `gex-page-1.html` (2 cours), `gex-page-2.html` (0 cours — « Aucun résultat » avec `total-resultats` présent, le signal de terminaison).
- Le test d'intégration parse les trois pages gelées, fusionne, trie et déduplique, puis compare au fichier fusionné `tests/fixtures/test_cases/listing/gex.json` (inchangé).
- Les comportements que les pages réelles n'exercent pas (0 entrée **sans** `total-resultats` = dérive de markup, entrée malformée) restent épinglés par des tests unitaires avec extraits HTML en ligne.
- La détection de la dérive du markup ne repose plus sur la suite de tests mais sur les garde-fous du scrape réel, déjà décidés : `data/catalogue.erreurs.log` non vide → alerte CI, plancher de rétrécissement à 90 %.

## Alternatives rejetées

- **E2e en direct (décision précédente)** : couple les tests au contenu réel du site — un simple changement de contenu (nouveau cours GEX) casserait le test sans que le parseur soit fautif ; la dérive est de toute façon détectée par le cron réel, au moment où elle compte.
- **Geler seulement la page nominale** : la terminaison sur page vide est le comportement le plus délicat du scrape de listing ; sans page vide gelée, la partie la plus risquée resterait sans fixture réelle.
