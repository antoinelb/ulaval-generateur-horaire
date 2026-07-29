# La pondération des crédits se résout par `Credits::resolve`, hors du domaine

Date : 2026-07-28

## Contexte

Un stage à crédits en intervalle (`Credits::Range`, MED-1911 « 6 à 12 ») n'a de total qu'avec la pondération choisie par l'étudiant (`2026-07-credits-variables-en-enum`).
La Phase 0 disait « la pondération entre en paramètre » sans fixer où : dans `build_domain`, ou à part.
La recherche de A n'utilise pourtant que les masques — les crédits ne servent qu'aux totaux (UI v0) et plus tard à B.

## Décision

`Credits::resolve(self, chosen: Option<u32>) -> Result<u32, String>` dans `course.rs`, séparée de la construction du domaine :

- `Fixed(n)` sans choix → `n` ;
- `Fixed` avec **n'importe quel** choix → erreur, même si la valeur coïncide : un choix qui atteint un cours à crédits fixes est un bogue amont, remonté plutôt qu'absorbé (aucune perte silencieuse) ;
- `Range { min, max }` avec choix dans `[min, max]` → le choix ; hors bornes → erreur ;
- `Range` sans choix → erreur — « défaut à la borne basse ? » reste la question ouverte de `next_steps.md`, que `resolve` laisse ouverte en exigeant le choix.

## Alternatives rejetées

- **`build_domain(course, season, chosen) -> Domain { credits, opts }`** : plombe la recherche de A d'une donnée qu'elle n'utilise pas et couple deux décisions indépendantes.
- **Ignorer le choix sur `Fixed`** : une perte silencieuse — la valeur affichée ne serait pas celle que l'étudiant croit avoir saisie.
- **Défaut à la borne basse sur `Range` sans choix** : trancherait ici une question que « Encore à planifier » réserve explicitement.
