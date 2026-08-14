# US-63 — B-GCI, cheminement sans concentration

**Persona** : Christophe, admis au baccalauréat en génie civil, qui ne veut pas se spécialiser.
**Intention** : choisir librement ses 15 crédits à option.

## Préconditions

Le B-GCI n'est pas encore servi par ce dépôt.
Pour jouer le scénario : déposer `data/programmes/B-GCI-A26.json`, ajouter `b-gci;Baccalauréat en génie civil` à `index-programmes.csv`, créer `b-gci/cours/cours-hors-catalogue.csv` et régénérer `data/programmes/index.json`.

## Ce que la spécialisation ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 25 cours (`FOR-2020`, `GAE-3006`, `GBO-2040`, `GBO-4015`, `GBO-4070`, `GCI-2101`, … `GMN-2001`).
- `credits_required` vaut 15.
- Particularité : « Cheminement sans concentration » est encodé comme une **concentration**, pas comme l'absence de concentration; c'est aussi la liste à laquelle les trois autres concentrations renvoient.

## Scénario

1. Christophe choisit « B-GCI » : la première concentration, « Cheminement sans concentration », est sélectionnée d'office.
2. Il place cinq cours de la Règle 1.
3. Il lit le bilan.

## Résultats attendus

- L'en-tête de section affiche `Cheminement sans concentration : 15 cr. / 15 cr.` une fois les cinq cours placés.
- La carte de la Règle 1 liste les 25 cours, chacun glissable.
- Les crédits sont plafonnés à 15 : placer un sixième cours de la liste n'augmente pas la contribution de la section.

## Repères pour le test e2e

- `#cheminement-select` contient quatre concentrations puis deux profils, dans cet ordre.
- `.rule-card` de la Règle 1 contient 25 `.course-line`.
- La contribution de section plafonne à 15 dans `#log-content`.

## Variantes et cas limites

- Le B-GCI partage plusieurs cours avec le B-GEX (`GAE-3006`, `GCI-3101`, `GCI-4201`) : les couleurs par matière restent cohérentes d'un programme à l'autre puisqu'elles dépendent du préfixe du sigle et du nombre de matières du programme affiché.
- Cette concentration sert de réservoir aux trois autres : la modifier change leur Règle 2 (US-64, US-65, US-66).
