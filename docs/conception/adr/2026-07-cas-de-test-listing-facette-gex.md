# Cas de test du listing catalogue : pages facettées GEX

Date : 2026-07-09

> **Partiellement remplacé** (2026-07-13) : l'unité de fixture par page est abandonnée au profit d'un fichier fusionné `gex.json`, insensible à la repagination ; les comportements par page migrent vers des tests unitaires avec HTML en ligne.
> Voir `2026-07-catalogue-artefact-commite.md`. Les observations (facette 113, pagination, liens « Dernière page » trompeurs) restent valides.

> **Renversé en partie** (2026-07-17) : le rejet du HTML gelé est annulé — le listing se teste finalement sur pages gelées, comme les pages cours et programmes.
> Voir `2026-07-listing-teste-sur-html-gele.md`.

## Contexte

La démarche test-first exige des cas de test e2e pour la page catalogue (https://www.ulaval.ca/etudes/cours), dernier cas manquant dans `docs/next_steps.md`.
Transcrire les ~10 000 cours n'a pas de sens ; il faut un sous-ensemble vérifiable à la main.

## Décision

L'unité de test est **une page de listing** : un HTML de page → un JSON attendu, comme pour les pages cours et programmes.
La vérité terrain est le listing filtré par la facette matière GEX (52 cours, pertinents au mandat) :

```
https://www.ulaval.ca/etudes/cours?search=&matieres%5B113%5D=113&day=All&start=All&end=All&field_sections_course_nbcred_min=All&field_sections_course_nbcred_max=All&page=N
```

Trois fichiers dans `tests/fixtures/test_cases/listing/` : `gex-page-0.json` (50 cours), `gex-page-1.json` (2 cours), `gex-page-2.json` (0 cours — la page « Aucun résultat », signal de terminaison).
Chaque entrée porte `code`, `title`, `url` (extraits des spans `cours-element--sigle` / `cours-element--titre` dans le lien `cours-element--lien`) ; le fichier porte aussi `total_results` (le « 52 résultats » de `total-resultats`), présent même sur la page vide.
JSON seulement, pas de HTML gelé : les tests du parseur rouleront contre le HTML frais, pour qu'une dérive du site fasse échouer bruyamment plutôt que silencieusement.

## Observations consignées

- Les facettes matière existent bel et bien (id interne 113 = GEX) — cela précise la note du spike du 2026-07-02 (« aucune facette nécessaire ») ; utiliser les facettes pour scoper un scrape par matière reste une question ouverte.
- Pagination : 50 entrées par page, `?page=N` indexé à 0.
- Sur le catalogue non filtré, le lien « Dernière page » (`?page=204`) dépasse le contenu réel : les pages 203–204 affichent « Aucun résultat » (0 entrée, vérifié sur le HTML brut le 2026-07-09) alors que la page 195 a du contenu.
  Le scraper doit donc terminer en itérant jusqu'à une page vide, jamais en se fiant aux liens de pagination.

## Alternatives rejetées

- **Page 0 du catalogue non filtré** : structurellement identique, mais contenu arbitraire (cours ACT) sans lien avec le mandat, et 50 entrées à vérifier sans intérêt.
- **Clé `next_page` dans le schéma** : spéculative ; la terminaison se fait sur page vide, le parseur ne rapporte que ce qui est sur la page.
- **HTML gelé en fixture dès maintenant** : rejeté par l'utilisateur — la dérive du catalogue doit faire échouer les tests visiblement.
- **Sessions « À l'horaire » et indicateurs D/H/C par entrée** : redondants, ces données viennent des pages cours déjà parsées.
