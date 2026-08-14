# US-11 — Millésime antérieur contenant un cours retiré du répertoire

**Persona** : Félix, inscrit au B-GMC sous le millésime A22, à deux sessions de diplômer.
**Intention** : garder sa grille conforme à la version du programme sous laquelle il a été admis.

Un étudiant conserve son millésime d'admission.
Certains cours de ces anciennes versions ont depuis disparu du répertoire ULaval et ne sont plus dans `data/cours.json`.

## Préconditions

- Programme « B-GMC », session d'admission « A22 ».

## Scénario

1. Félix choisit « A22 ».
2. Il charge son cheminement.
3. Il repère les cours dont la pastille n'a ni titre ni crédits.

## Résultats attendus

- Les millésimes convertis (A22 à H27 pour le B-GMC, A23 à H27 pour le B-GIN) se chargent comme les millésimes scrapés : mêmes règles, même bilan.
- Un sigle du programme absent du catalogue déclenche un avertissement `Sigle introuvable dans le catalogue de cours : XXX-0000` dans la console, et sa ligne du panneau affiche un titre vide et `0` crédit.
- Un tel cours ajouté à `{prog}/cours/cours-hors-catalogue.csv` retrouve titre et crédits, mais reste sans horaire ni préalables.

## Repères pour le test e2e

- `#admission-select` contient `A22` pour `b-gmc`.
- Une `.course-line` dont `data-credits` vaut `0` correspond à un sigle introuvable.
- Le message de console est observable via `page.on('console')` dans Playwright.

## Variantes et cas limites

- Les anciens millésimes ne comptent qu'en crédits : leurs contraintes converties sont toutes de type `credits`, jamais `course`.
- La règle « Cours compensateurs » des anciens fichiers a été renommée « Scolarité préparatoire » : la case à cocher doit donc fonctionner aussi sur ces millésimes.
- Les fichiers de millésimes antérieurs ne sont jamais réécrits par le cron; un test qui les modifie corrompt le dépôt.
