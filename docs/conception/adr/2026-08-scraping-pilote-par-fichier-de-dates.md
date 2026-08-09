# Le scraping piloté par un fichier de dates committé

Date : 2026-08-09

## Contexte

La cadence du cron CI (jalon 5) était une question ouverte depuis le début du plan : hebdomadaire ou quotidien ?
Une cadence fixe est pourtant décorrélée du vrai rythme des données : ULaval publie et met à jour ses horaires à des moments précis du calendrier universitaire, pas chaque semaine.
Le calendrier utile, fourni par Antoine le 2026-08-09 :

- 10 novembre — téléchargement des programmes et des horaires de cours pour l'hiver
- 10 janvier — mise à jour des horaires de cours pour l'hiver
- 20 mars — téléchargement des horaires de cours pour l'été
- 1er mai — mise à jour des horaires de cours pour l'été
- 1er avril — téléchargement des programmes et des horaires de cours pour l'automne
- 1er septembre — mise à jour des horaires de cours pour l'automne

## Décision

Le scraping est piloté par un fichier committé, `data/dates_scraping.txt` : première ligne `mm-jj` (le format), puis une date par ligne, triées.
Les dates ne portent pas d'année — le fichier vaut pour toutes les années, sans maintenance annuelle.
Le cron se déclenche quotidiennement mais ne lance un scrape **complet** (catalogue + cours + programmes) que si le mois-jour du jour figure au fichier.
Les six dates initiales sont celles du calendrier ci-dessus ; le contexte de chaque date (quelle session, publication ou mise à jour) reste consigné ici, pas dans le fichier.
Cohérence avec le millésime des programmes : les deux dates de publication des programmes tombent juste sous la règle de datation — 10 novembre → hiver suivant, 1er avril → automne courant (ADR `2026-08-millesime-automne-ou-hiver-jamais-ete`).

## Alternatives rejetées

- **Cadence fixe hebdomadaire ou quotidienne** : des scrapes décorrélés des publications — soit trop tôt (rien de neuf), soit trop tard (données périmées entre deux runs).
- **Dates avec année** : imposerait de réécrire le fichier chaque année pour le même calendrier.
- **JSON structuré avec portée par date** (programmes vs cours seulement) : le scrape complet à chaque date listée suffit — ~20 min par run, six runs par année ; la distinction ne vaudrait pas la structure.
