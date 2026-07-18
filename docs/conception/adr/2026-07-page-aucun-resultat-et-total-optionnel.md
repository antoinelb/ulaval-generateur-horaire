# Catalogue : variante « Aucun résultat » et total optionnel

Date : 2026-07-17

> **Précision** (2026-07-17, après re-gel des fixtures via `curl`) : la classe `resultats--offre-etudes` est le conteneur de résultats de **toutes** les pages, pas un marqueur de page vide.
> Le marqueur est donc le **texte** « Aucun résultat » dans le `<p>` de ce conteneur ; sa seule présence en classe ne prouve rien (une dérive de `total-resultats` serait sinon lue comme « fin des résultats »).

> **Partiellement remplacé** (2026-07-17) : la « page vide avec preuve de forme » n'est plus le signal de terminaison du scrape — la pagination se calcule depuis la page 0 et se vérifie par réconciliation arithmétique (voir `2026-07-pagination-du-catalogue-par-comptage.md`).
> Tout le reste tient : les trois issues du parseur, `total_results: Option<usize>` et les quatre fixtures restent en vigueur — ce sont des formes de page à reconnaître, plus une condition d'arrêt.

## Contexte

Le site rend une page « vide » de deux façons différentes selon la requête :

- **Facette active (GEX), page au-delà de la fin** (`gex_2.html`) : markup normal — `total-resultats` affiche toujours « 52 résultats », `<div id="resultats">` est vide.
- **Catalogue sans facette, page au-delà de la fin** (`all_last.html`) : markup entièrement différent — aucun `total-resultats`, aucun `cours-element` ; un seul `<div class="… resultats--offre-etudes"><p>Aucun résultat</p></div>`.

La règle de terminaison prévue (« 0 entrée sans `total-resultats` = dérive de markup = erreur ») classerait la seconde page, pourtant légitime, comme une dérive.
Et `total_results: usize` n'a pas de valeur honnête pour une page qui n'affiche aucun nombre (0 serait une donnée fausse : le catalogue complet n'est pas vide).

## Décision

- **Quatre fixtures gelées** dans `tests/fixtures/test_cases/catalogue/` : `gex_0.html` (50 cours), `gex_1.html` (2 cours), `gex_2.html` (0 cours, `total-resultats` présent), `all_last.html` (variante « Aucun résultat »).
- **`CataloguePage.total_results` devient `Option<usize>`** : `None` sur la variante « Aucun résultat », qui n'affiche aucun compte.
- **Terminaison à trois issues** : entrées présentes = page normale ; 0 entrée **avec** preuve de forme (`total-resultats` **ou** marqueur `resultats--offre-etudes`/« Aucun résultat ») = fin des résultats ; ni l'un ni l'autre = dérive de markup = `Err`.

## Alternatives rejetées

- **`total_results: usize` avec 0 par défaut** : enregistre une donnée fausse ; viole « une anomalie est une donnée » — la page ne dit pas 0, elle ne dit rien.
- **Enum `Results`/`NoResults`** : rend la variante vide impossible à ignorer, mais impose un `match` à chaque appel pour une distinction qu'un seul consommateur (le fetcher) exploite.
- **Traiter « Aucun résultat » comme dérive** (règle initiale) : contredite par le HTML réel gelé ; la page est un état légitime du site.
