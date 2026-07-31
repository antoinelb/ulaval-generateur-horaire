# Un snapshot unique `data/cours.json`, millésime `last_offered` par saison

Date : 2026-07-31

Remplace `2026-07-cours-par-session-et-annee` ; rend caduc `2026-07-nettoyage-des-snapshots-perimes`.

## Contexte

Le découpage par session (`data/cours/a2026.json`, 49 fichiers) implémentait l'hypothèse fondatrice **par les noms de fichiers** : le planificateur ne lisait que le fichier le plus récent de chaque saison, et un cours offert seulement dans un millésime plus ancien (offre aux deux ans, cours retiré puis republié) devenait invisible.
Le millésime dont le pli d'équivalence a besoin (`resolve_offering`) était piégé dans des noms de fichiers que `core` ne voit jamais, forçant l'appelant à transporter des paires `(offering, u16)`.
Enfin, un cours nouveau sans horaire publié (GCI-1011) n'appartient à aucune session nommable : aucun fichier `{saison}{année}` ne peut le porter — il disparaissait des données.

## Décision

- **Un seul `data/cours.json`** à côté de `catalogue.json`, `{"courses": [Course]}` trié par code, chaque `Course` multi-saisons porté entier.
- **`SeasonOffering` gagne `last_offered: Option<u16>`** — l'année du bloc le plus récent que la page affichait pour cette saison. `options` devient `Option<Vec<Vec<Section>>>` (voir l'ADR `2026-07-cours-sans-section-de-session-offert-automne-hiver`). Pas de `skip_serializing_if` : `null` explicite dans le snapshot et les fixtures.
- **Sémantique par cours** : tout cours du fichier est connu du planificateur ; chaque saison garde l'horaire de sa dernière année d'offre. L'hypothèse fondatrice — une session sans horaire publié réutilise le plus récent de la même saison — s'applique désormais au cours, plus au fichier.
- **`resolve_offering` lit le millésime dans la donnée** : signature `Option<&SeasonOffering> × 2 → Option<&SeasonOffering>` ; `None < Some(_)` fait perdre une offre sans millésime contre toute offre datée, égalité au cours lui-même (`2026-07-equivalences-par-millesime-de-session` inchangé sur le fond).
- **L'année de `parse_session` devient décorative** pour la sélection (`schedule a2019` et `schedule a2026` lisent la même donnée) — elle reste validée comme garde-typo.
- Le cache par cours (`data/cache/cours/{code}.json`) porte le `Course` nu ; l'ancien enveloppement `{course, years}` ne se désérialise plus → miss → re-fetch. **Une re-scrape complète est assumée** (~10 000 pages, ~20 min, une fois) : un lecteur toléré de l'ancien format vivrait pour toujours pour économiser 20 minutes, et seul un re-scrape fait entrer les cours nouveaux absents de tout ancien snapshot.
- Le remplacement du fichier reste atomique (rename) ; un run complet remplace tout, un run `--subjects` fusionne par code en gardant les matières étrangères. Plus rien n'est « périmé » à balayer : `2026-07-nettoyage-des-snapshots-perimes` tombe.
- Le sidecar manuel devient `data/cours.manuel.json` (`2026-07-contribution-de-cours-manuels` à ajuster à l'implémentation).

## Pourquoi le rejet d'alors tombe

`2026-07-cours-par-session-et-annee` rejetait « un seul `data/cours.json` » (l'UI devrait trier les saisons) et « l'année dans `core::Course` » (contamination du type métier, fixtures invalidées).
Le premier argument ne tient plus : `Course.seasons` est déjà indexé par saison, l'UI lit `seasons[season]` sans tri.
Le second est renversé sciemment : la sélection par cours, le pli d'équivalence et la règle du cours nouveau ont tous besoin du millésime *dans* la donnée — et les fixtures sont régénérées par le parseur, pas préservées.

## Alternatives rejetées

- **Garder les fichiers par session et indexer à la lecture** : conserve la perte des cours des millésimes anciens, et le millésime reste hors de portée de `core`.
- **`last_offered` requis (`u16`)** : casserait la lecture d'entrées manuelles futures sans millésime ; `Option` avec `null` explicite garde l'écriture stricte et la lecture tolérante.
- **Sélection « année max par saison » reproduite en lecture** (statu quo comportemental) : garde la longue traîne dans le fichier sans jamais s'en servir — c'est précisément le défaut corrigé.

## Plafond connu

`(last_offered: Some(y), options: None)` est improductible par le parseur mais exprimable dans `cours.manuel.json` ; une telle offre pourrait éclipser un horaire réel via le pli d'équivalence. Documenté, pas défendu.
