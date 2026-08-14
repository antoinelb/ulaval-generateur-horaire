# US-64 — B-GCI, concentration « Eau et environnement »

**Persona** : Ariane, en génie civil, orientée vers l'hydraulique et l'assainissement.
**Intention** : combler les 15 crédits de sa concentration.

## Préconditions

Mêmes préconditions de déploiement qu'en US-63.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 12 crédits parmi 10 cours (`FOR-2020`, `GAE-3006`, `GCI-3101`, `GCI-4004`, `GCI-4201`, `GCI-4301`, `GEX-1000`, `GEX-2001`, `GEX-3001`, `GGL-2600`).
- Règle 2 : 3 crédits **à prendre dans la Règle 1 du cheminement sans concentration** — le fichier l'exprime par une référence croisée, pas par une liste.
- `credits_required` vaut 15.

## Scénario

1. Ariane choisit la concentration « Eau et environnement ».
2. Elle place quatre cours de la Règle 1.
3. Elle cherche les cours de la Règle 2.

## Résultats attendus

- La Règle 1 affiche ses dix cours et se comble normalement à 12 crédits.
- L'en-tête de section affiche `Eau et environnement : X cr. / 15 cr.`

## Repères pour le test e2e

- La carte de la Règle 1 contient 10 `.course-line`.
- La carte de la Règle 2 contient le texte `Aucun cours défini pour cette règle.`
- La ligne `Règle 2 : 0 cr. / 3 cr.` du journal porte la classe `log-warning`, quoi qu'Ariane place.

## Variantes et cas limites

- **Écart connu et bloquant** : le champ `courses` de la Règle 2 vaut `{"concentration": "Cheminement sans concentration", "rule": "Règle 1"}`. Le frontend ne retient une liste que si `courses` est un tableau, donc la règle se charge avec zéro cours tout en exigeant 3 crédits. Elle reste en avertissement pour toujours et fausse le total exigé de la section. Deux issues : résoudre la référence au chargement, ou l'aplatir côté scraper.
- Le même défaut touche les concentrations « Géotechnique » (US-65) et « Structures et matériaux » (US-66) : c'est un seul correctif pour trois histoires.
- Tant que la référence n'est pas résolue, `Eau et environnement` ne peut jamais afficher `15 cr. / 15 cr.`
