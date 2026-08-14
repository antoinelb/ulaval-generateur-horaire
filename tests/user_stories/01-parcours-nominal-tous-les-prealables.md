# US-01 — Parcours nominal : tous les préalables préuniversitaires acquis

**Persona** : Émile, admis au B-GEX à l'automne 2026, arrivant d'un DEC en sciences de la nature.
**Intention** : partir du cheminement type et vérifier que rien n'accroche.

## Préconditions

- Programme « B-GEX », session d'admission « A26 », spécialisation « Cheminement sans concentration ».
- La grille est vide.

## Scénario

1. Émile ouvre l'application.
2. Il choisit « B-GEX » puis « A26 ».
3. Il ouvre « Charger un cheminement » et clique le bouton « A26 ».
4. Il lit le journal « Analyse du cheminement ».

## Résultats attendus

- La grille se remplit de A26 à H30, une pastille par cours du cheminement type.
- La case « Scolarité préparatoire complétée » est cochée par défaut, et la section « Scolarité préparatoire » du panneau est repliée.
- Aucune pastille ne porte de bordure d'erreur.
- Le journal ne contient aucune ligne d'erreur, seulement le bilan des crédits.
- Le total affiché est `120 cr. / 120 cr.` ou moins si le cheminement type laisse des cours à option à déterminer.

## Repères pour le test e2e

- `#programme-select` = `b-gex`, `#admission-select` = `A26`.
- `#modal-chargement.visible` puis un bouton dans `#modal-cheminements-dynamiques`.
- Aucun `.dropped-tile.prerequis-manquants`, `.cours-non-offert` ni `.cours-en-conflit`.
- `#log-content .log-error` est vide.
- La dernière ligne de `#log-content` commence par `Total :`.

## Variantes et cas limites

- Sans charger de cheminement type, la grille reste vide et le journal n'affiche que le bilan à `0 cr.`.
- Le cheminement type du B-GEX A26 place `MAT-1900` en A26 alors que ses préalables `MAT-0130 ET MAT-0150 ET MAT-0260` sont préuniversitaires : le scénario ne passe que si la case « Scolarité préparatoire complétée » couvre bien ces trois sigles (voir US-38).
