# `data/meta.json` : la provenance du snapshot, estampillée par le scraper

## Contexte

Les règles d'interface (AIR TRU-2, `docs/ux/interface-rules.md`) exigent que l'âge des données soit visible : l'UI doit afficher « données du … ».
Or rien ne porte la date d'un scrape : git ne conserve pas les mtimes (un checkout les réécrit), et l'en-tête HTTP `Last-Modified` servi par GitHub Pages reflète le déploiement, pas la récolte.

## Décision

`ulaval-scraper courses` écrit `data/meta.json` — `{"scraped_at": "<ISO-8601 UTC, suffixe Z>", "course_count": <n>}` — par la même écriture atomique que `cours.json`, à chaque run (complet ou `--subjects`).
L'horodatage réutilise le `civil_from_days` déjà présent (étendu au jour) ; une horloge pré-1970 est plafonnée à l'époque, visible dans le fichier plutôt que fatale.

## Alternatives rejetées

- **`Last-Modified` de Pages** : date de déploiement, pas de scrape ; assez proche en temps normal (le cron enchaîne scrape → commit → déploiement) mais faux dès qu'un déploiement survient sans scrape.
- **Déduire la date des `last_offered`** : millésime d'offre, pas date de récolte — un mensonge de provenance.
- **Epoch en secondes** : plus simple à parser mais illisible pour un humain qui ouvre le fichier ; l'ISO-8601 se suffit à lui-même (TIM-1 : jamais de datetime naïf).
