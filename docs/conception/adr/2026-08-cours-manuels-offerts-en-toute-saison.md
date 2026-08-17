# Cours manuels : offerts en toute saison, horaire non publié

Date : 2026-08-15

## Contexte

L'interface JS soeur (`grille-de-cheminement-interactive`, `CORRECTIFS-AMONT.md` items 7-8) référence des sigles absents du catalogue : gabarits de choix (`OPT-ION1`, `AUC-HOIX`), échanges (`EHE-1GEX`), cours retirés du répertoire (`ENT-3020`).
Ses fiches locales n'avaient aucune saison, donc « non offert » dans toute session — faux positif systématique.
`data/cours.manuel.json` était prévu au plan mais n'existait pas.

## Décisions

- `data/cours.manuel.json` est créé : la forme de `cours.json` (`{"courses": [...]}`, entrées `Course` complètes), maintenu à la main, jamais écrit par le scraper (protection déjà testée).
- **Convention** : chaque entrée manuelle liste les **trois saisons** avec `{"last_offered": null, "options": null}` — la sémantique existante « offert, horaire non publié » (ADR `2026-07-cours-sans-section-de-session-offert-automne-hiver`). Aucun champ nouveau, aucun changement de code : un consommateur qui sait lire `cours.json` sait lire ce fichier.
- Contenu initial : l'union des `cours-hors-catalogue.csv` des quatre programmes du dépôt JS, moins les 0xxx désormais au catalogue (`BIO-0150`, `CHM-0160/0170/0250`), plus les échanges par programme (`EHE-1ANT/1GCI/1GIN/1GPH`, item 7). Titres et crédits repris tels quels des CSV — y compris `SAN-SÉCU` (sigle non standard) et les 0 crédits d'`ENT-3020`, jamais réinventés.
- Un test d'intégration côté scraper parse le fichier avec les types core et vérifie la convention (trois saisons, `null` partout).

## Alternatives rejetées

- Un champ dédié « offert en toute session » : nouveau format à faire comprendre à tous les consommateurs, alors que la sémantique `last_offered`/`options` nuls dit déjà exactement cela.
- `seasons: {}` (aucune saison) : c'est le symptôme — un cours jamais offert n'est plaçable nulle part.
