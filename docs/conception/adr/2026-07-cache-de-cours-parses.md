# Cache de cours parsés, seulement quand le parsing est propre

Date : 2026-07-19

Étendu le 2026-07-20 par `2026-07-cache-du-verdict-hors-perimetre` : le cache porte aussi le verdict « hors périmètre », une seconde forme de fichier `{"out_of_scope": …}` lue *untagged* à côté de `{"course": …}`.

## Contexte

Un scrape complet des pages cours dure ~15 min sous le throttle.
Pendant le développement il sera relancé souvent, et refaire ~8 850 requêtes à chaque itération est le coût dominant — bien avant la mémoire (~25 Mo pour tout le run) ou le temps de parsing.

## Décision

- Un fichier JSON par cours sous `{output_dir}/cache/cours/{code}.json`, `.gitignore`d : `data/cache/`.
  Contenu : `{"course": …, "years": …}` — le `Course` parsé plus les années de ses saisons, tout ce qu'il faut pour assembler les snapshots sans réseau.
- **Seul un parsing sans anomalie est mis en cache.**
  Un cours dégradé est reparcouru à chaque run, donc une correction du parseur l'atteint sans purge manuelle, et son anomalie reste visible dans `cours_errors.log` tant qu'elle n'est pas réellement corrigée.
  Une page dont la forme n'est pas reconnue ne produit rien et n'écrit donc rien : elle se répare d'elle-même.
- Pas de TTL, pas de drapeau `--refresh` : `rm` est l'interface — le répertoire entier après un changement de parseur, un fichier précis pour un cours suspect.
  Le cron part d'un checkout propre, donc CI refait toujours un scrape complet.
- Pas d'écriture atomique pour le cache : un fichier tronqué par un arrêt brutal échoue à se désérialiser, ce qui vaut défaut de cache, donc re-fetch.
  Même traitement pour un fichier illisible ou d'un format périmé.
- Un échec d'écriture du cache est une anomalie (`CourseError::Cache`), pas une erreur fatale : le snapshot produit reste correct.
  Le répertoire est créé en amont pour qu'un chemin inutilisable échoue avant la première requête, pas à la millième.

## Alternatives rejetées

- **Cacher le HTML brut** (comme le prévoyait `project_plan.md` § flux) : survivrait aux changements de parseur, mais coûte ~110 Ko par page — ~1 Go pour le catalogue complet, ~200 Mo compressé.
  `cours_errors.log` désigne déjà, cours par cours, ce qu'il faudrait invalider, et les échecs durs ne mettent rien en cache : le gain ne justifie pas le volume.
  Le HTML gelé reste la référence pour les *fixtures*, ajoutées à la main quand une page révèle un cas nouveau.
- **Cacher tout parsing réussi, anomalies comprises** : moins de requêtes, mais il faudrait relire le journal et supprimer les fichiers concernés à chaque correction du parseur.
  Le coût évité (quelques dizaines de pages par run) ne vaut pas cette étape manuelle.
- **TTL sur l'âge du fichier** : introduit un nombre arbitraire et une horloge, pour un problème que le checkout propre du cron résout déjà.
