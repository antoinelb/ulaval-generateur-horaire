# Référence de règle structurée (et forme unique pour « any »)

Date : 2026-07-13

## Contexte

Dans le bac en génie civil, la « Règle 2 » de chaque concentration renvoie aux cours d'une autre concentration : « tous les cours de la Règle 1 du cheminement sans concentration ».
Cette phrase revient à l'identique dans les trois concentrations nommées ; la laisser en `raw` seulement obligerait à la re-parser plus tard.
Par ailleurs, deux fixtures encodaient « tous les cours de premier cycle… » sous deux formes différentes : `"courses": "any"` avec `raw` en champ frère (génie civil) et `"courses": {"any": "<texte>"}` (génie des eaux) — un seul type ne peut pas produire les deux.

## Décision

1. La phrase « tous les cours de la Règle N du cheminement X » devient une troisième forme reconnue de la grammaire des règles : `"courses": {"concentration": "X", "rule": "Règle N"}`, le texte source conservé dans `raw`.
   La résolution en liste de cours se fait dans `core` (recherche par titres, issus de la même page donc cohérents dans un snapshot) ; une référence dont la cible est elle-même une référence est une erreur, pas une chaîne à suivre.
2. Forme unique pour les phrases parsées : la valeur parsée dans `courses`, le texte source dans `raw` en champ frère (même patron que `Prerequisites.raw`/`tree`). La fixture génie des eaux est alignée sur cette forme.
3. Côté Rust, `courses`/`raw` ne sont pas deux `Option` indépendantes : un seul enum (`RuleCourses`, aplati via `#[serde(flatten)]`) n'admet que les combinaisons légales — liste explicite sans `raw`, phrase parsée (`any` ou référence) toujours avec `raw`, ou `raw` seul (hors grammaire, jamais perdu).

## Alternatives rejetées

- **Garder la phrase en `raw` seulement** : la référence est nécessaire au calcul de couverture des règles ; la re-parser hors du parseur disperse la grammaire.
- **Résoudre la référence au parsing (inliner la liste de cours)** : duplique ~24 codes qui peuvent se désynchroniser de la page, fait entrer une logique de résolution inter-sections dans le scraper, et perd l'intention affichable (« mêmes cours que la Règle 1 du tronc commun »).
- **Deux champs `Option` indépendants (`courses`, `raw`)** : autorise des états impossibles (`None`/`None`, phrase parsée sans texte source).
