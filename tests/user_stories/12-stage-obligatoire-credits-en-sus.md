# US-12 — Stages obligatoires, crédits en sus

**Persona** : Maude, au B-GEX, qui planifie ses stages `GEX-1580`, `GEX-2590` et `GEX-3590`.
**Intention** : placer ses stages sans fausser le compte des 120 crédits du programme.

Les crédits de stage sont *en sus* des crédits exigés par le programme.
Ils sont donc affichés mais exclus du total, et exclus des crédits accumulés qui servent aux préalables.

## Préconditions

- Programme « B-GEX », cheminement type A26 chargé.

## Scénario

1. Maude place `GEX-1580` (9 crédits) en E27.
2. Elle place `GEX-2590` en E28 et `GEX-3590` en E29.
3. Elle lit la règle « Stages » et la ligne « Total » du bilan.

## Résultats attendus

- La règle « Stages » apparaît dans le panneau et dans le bilan avec ses crédits accumulés.
- Ces crédits **ne sont pas** ajoutés au total global : le total reste sur les 120 crédits du programme.
- Les crédits de stage ne comptent pas non plus dans les crédits accumulés utilisés pour évaluer un préalable du type « Crédits exigés : N ».
- `GEX-2590` exige `GEX-1580` : le placer dans la même session ou avant déclenche l'erreur de préalable.
- `GEX-1580` exige 24 crédits accumulés : le placer trop tôt déclenche l'erreur de crédits (US-41).

## Repères pour le test e2e

- Le journal contient une ligne `Stages : … cr. / …` sous la section « Activités communes ».
- La ligne `Total : X cr. / 120 cr.` ne varie pas quand on ajoute ou retire un stage.
- L'exclusion repose sur le titre de règle contenant « stage » : un renommage côté scraper casse ce comportement.

## Variantes et cas limites

- La règle « Stages » du B-GEX A26 est bornée `{type: course, min: 1, max: 8}`; le panneau la convertit en crédits, ce qui donne un intervalle affiché sous la forme `min à max crédits`.
- Un stage à pondération variable porte des crédits `{min, max}` dans le catalogue; l'application retient la borne basse.
- Maude peut ne faire qu'un seul stage : la règle est alors satisfaite dès son minimum.
