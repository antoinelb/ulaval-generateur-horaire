# Le budget de B est une double borne : nœuds développés et solutions retenues

**Date :** 2026-07-30
**Statut :** accepté (décision Antoine) ; ferme la question « définition d'un nœud » laissée ouverte par `2026-07-b-enumere-toutes-les-solutions` et le schéma des fixtures (`2026-07-schema-des-fixtures-de-placement`, `expected.complete: false` réservé).

## Contexte

B doit borner à la fois le **travail** (latence de la recherche complète, y compris la queue sur-contrainte) et la **mémoire** (taille de l'ensemble de solutions retourné, contrainte WASM).
Un seul nombre ne peut pas faire les deux : un espace vide de solutions peut coûter cher à épuiser (travail sans mémoire), une liste partielle lâche peut produire des solutions en masse à bas coût (mémoire sans travail).
La fixture « budget atteint = ensemble partiel » attendait une définition observable et déterministe du comptage.

## Décision

- Deux bornes distinctes, toutes deux **paramètres d'entrée** de `place` (aucune constante en dur dans `core`) :
  - `max_nodes` : un nœud = une **affectation partielle développée** — un préfixe retiré de la pile de travail et étendu à ses successeurs.
    Le compte est indépendant des détails d'itération des domaines, donc stable et documentable.
  - `max_solutions` : plafond de la taille de l'ensemble retourné.
- La recherche est un **parcours en profondeur sur pile explicite** dans un `try_fold` borné par `max_nodes` — pas la frontière par cours de A : une frontière ne matérialise les solutions qu'au dernier cours, donc un arrêt au budget y perdrait tout le travail ; en profondeur, les solutions déjà trouvées survivent à l'arrêt.
  La pile est bornée par cours × sessions, la mémoire de recherche reste plate (WASM).
- Trois issues, jamais confondues, portées par un enum `Completion` : `Complete` (recherche épuisée — ensemble total, vide = infaisabilité **prouvée**), `NodeBudget` (travail épuisé — ensemble partiel), `SolutionCap` (plafond mémoire atteint — ensemble partiel).

## Alternatives rejetées

- **Nœud = extension tentée** : plus fin, mais le compte dépend de l'ordre et du filtrage internes des domaines — fragile au refactor, invisible de l'extérieur.
- **Nœud = solution trouvée** : ne borne pas le travail — un espace sur-contraint sans solution s'épuise sans jamais décrémenter.
- **Une seule borne** : voir le contexte — travail et mémoire ne sont pas corrélés sur les entrées réelles (bac complet vs tronc seul).
- **Frontière par cours comme A** : perd toutes les solutions si le budget tombe avant le dernier cours ; la profondeur rend l'arrêt anticipé utile.
