# US-25 — Charger les cours complétés depuis Capsule

**Persona** : Laurie, en troisième année, qui veut importer son relevé de notes plutôt que de tout ressaisir.
**Intention** : remplir la colonne « Cours complétés » en collant le code source de sa page Capsule.

## Préconditions

- Le code HTML d'un relevé de notes Capsule, dont les lignes de tableau ont au moins quatre cellules : sigle, …, …, note.

## Scénario

1. Laurie clique « Charger un cheminement » puis « De Capsule… ».
2. Elle colle le HTML dans la zone de texte.
3. Elle clique « Charger ».

## Résultats attendus

- Seuls les cours dont la note est réussie sont retenus : `D, D+, C-, C, C+, B-, B, B+, A-, A, A+, P`.
- Un cours échoué puis repris et réussi est retenu une seule fois; un cours seulement échoué est ignoré.
- Les cours retenus sont placés dans la colonne « Cours complétés », un par cellule libre, des rangées étant ajoutées au besoin.
- Un cours déjà placé dans une session est déplacé vers la colonne 0; un cours déjà en colonne 0 n'est pas dupliqué.
- Un message du journal indique le nombre de cours chargés.
- Un cours absent du catalogue est ignoré silencieusement — c'est le cas des cours d'un autre programme ou d'une autre université.

## Repères pour le test e2e

- `#modal-capsule` passe à `display: flex`.
- Coller un HTML vide et cliquer « Charger » déclenche une alerte native.
- Après chargement, `#log-content` contient `Capsule : N cours complété(s) chargé(s).`
- Un HTML sans aucune ligne valide produit l'avertissement `Capsule : aucun cours complété trouvé dans le code HTML.`

## Variantes et cas limites

- Le HTML collé est analysé avec `DOMParser` : aucun script ne doit s'y exécuter, et une balise `<script>` dans le collage ne doit rien déclencher.
- Une note en cours (`EN`, `X`, vide) ne compte pas comme réussie.
- Un sigle mal formé (`IFT 1004` avec une espace) ne correspond pas au motif attendu et est ignoré.
- Le bouton « De Capsule… » ferme d'abord la fenêtre de chargement, puis ouvre la sienne; les deux ne sont jamais visibles ensemble.
