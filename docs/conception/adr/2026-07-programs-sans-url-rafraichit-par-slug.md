# `program` sans URL rafraîchit les snapshots existants par leur slug

Date : 2026-07-29

> **Mise à jour** (`2026-08-millesime-de-programme-en-semestre`) : le suffixe de millésime est désormais un semestre (`-A26`) ; le repli par slug reconnaît ce suffixe **et** l'ancien `-{year}` à quatre chiffres.

## Contexte

La sous-commande `program` exigeait sa liste d'URL : une page programme est un slug qu'aucun code de cours ne permet de reconstruire, et seuls les programmes dont on veut les règles méritent un scrape.
Mais une fois un programme dans `data/programmes/`, le re-scraper périodiquement (cron, nouvelle année) obligeait à retenir et repasser les mêmes URL — alors que le répertoire les connaît déjà : le nom de fichier `{slug}-{year}.json` porte le slug, qui **est** le dernier segment de l'URL canonique (`parse_code` le lit dans `link[rel="canonical"]`).

## Décision

- **`urls` devient optionnel** : sans argument, la liste de travail est reconstruite depuis `data/programmes/` — stems des `*.json`, `-{year}` retiré, plusieurs millésimes d'un même programme dédoublonnés en un seul slug (`BTreeSet`), `*.manuel.json` exclu (maintenu à la main, jamais scrapé).
- **URL = `{base_url}/{slug}`**, avec `--base-url` (défaut `https://www.ulaval.ca/etudes/programmes`) pour pouvoir tester le rafraîchissement contre un serveur mock.
- **Rien à rafraîchir = erreur** nommant le répertoire, jamais un no-op silencieux.
- Un stem legacy sans suffixe d'année reste un slug entier ; son rafraîchissement écrit le nom suffixé à partir de maintenant.

## Alternatives rejetées

- **Stocker l'URL dans `core::Program`** : redondant — `code` est déjà le slug canonique — et exigerait une migration de format pour une donnée reconstructible.
- **Dériver la liste des pages de cours** (« Cette activité est contributoire dans : ») : re-scraper des milliers de pages pour apprendre ce que le répertoire sait déjà.
- **Garder l'obligation d'URL** : condamne le cron à une liste dupliquée à la main, qui divergerait du contenu réel de `data/programmes/`.
