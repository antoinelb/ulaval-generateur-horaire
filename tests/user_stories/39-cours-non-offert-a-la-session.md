# US-39 — Cours non offert à la session choisie

**Persona** : Charlotte, qui a placé `CHM-1903` à l'hiver alors que ce cours n'est offert qu'à l'automne.
**Intention** : être avertie avant de bâtir tout son cheminement sur une session impossible.

## Préconditions

- `CHM-1903`, `GCI-1000`, `GCI-1003` et `GEX-1002` ne sont offerts qu'à l'automne.
- `GMN-2901` n'est offert qu'à l'hiver.

## Scénario

1. Charlotte place `CHM-1903` dans une colonne `H27`.
2. Elle lit l'alerte, puis le déplace en `A27`.

## Résultats attendus

- La pastille est marquée non offerte et le journal indique « Le cours CHM-1903 ne sera pas offert à la session H27. »
- Déplacé vers une colonne d'automne, l'avertissement disparaît.
- Un cours offert aux trois saisons ne déclenche jamais cette alerte.
- Un cours placé dans « Cours complétés » n'est jamais évalué pour l'offre.

## Repères pour le test e2e

- `.dropped-tile[data-code="CHM-1903"].cours-non-offert` dans la colonne `H27`.
- L'entrée correspondante existe dans `#log-content .log-error`.
- Aucune de ces classes dans la colonne `A27`.

## Variantes et cas limites

- L'année de référence est le `last_offered` le plus récent du snapshot, **pas** l'horloge du navigateur : si le cron de scrape s'arrêtait, l'application ne déclarerait pas soudainement tout le catalogue périmé.
- Un cours dont la saison est connue mais dont l'horaire n'est pas publié reste considéré offert.
- Un cours retiré du dernier horaire publié (plus de deux ans sans offre) est signalé non offert dans toutes les sessions.
- Un cours hors catalogue n'a aucune saison : il est signalé non offert partout, ce qui est un faux positif pour les pseudo-cours (US-03, US-16).
