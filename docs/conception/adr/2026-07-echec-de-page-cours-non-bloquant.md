# Une page cours en échec est une anomalie, pas un arrêt

Date : 2026-07-19

## Contexte

Le scrape du catalogue avorte à la première erreur : `try_collect` propage, et la commande sort en code 2.
C'est tenable pour ~250 requêtes en 50 s.

Les pages cours sont ~8 850 requêtes, soit ~15 min sous le throttle à 10 req/s.
Y appliquer la même règle ferait jeter un quart d'heure de réseau pour une seule page injoignable — et le premier scrape réel a justement pour but de découvrir les pages que le parseur ne sait pas encore lire.

## Décision

- `course::scrape` collecte avec `collect` et non `try_collect`.
  Chaque page produit `(Option<CachedCourse>, Vec<CourseError>)` : l'échec est une donnée, jamais une interruption.
- Trois issues, toutes tracées dans `data/cours_errors.log`, une par ligne :
  - **fetch en échec** — aucun cours, l'anomalie porte l'URL (déjà présente dans `Display` de `FetchError`) ;
  - **forme de page non reconnue** (`parse` retourne `Err`) — aucun cours, l'anomalie ajoute l'URL, que `ParseError` ne porte pas ;
  - **anomalie interne** (mode inconnu, en-tête de session illisible) — le cours partiel est **conservé** et l'anomalie remontée, conformément à « ne jamais rien perdre silencieusement ».
- Un run propre supprime un journal laissé par un run précédent, comme pour le catalogue.
- La commande sort en 0 même avec des anomalies : l'échec dur reste réservé à ce qui empêche de produire un snapshot (catalogue absent, répertoire inutilisable).

## Alternatives rejetées

- **Avorter comme le catalogue** : un 404 sur un cours retiré tuerait le run entier, sans rien protéger — le snapshot existant reste servi jusqu'au `rename` final de toute façon.
- **Reprise sur processus tué (point de contrôle)** : reste au jalon 5.
  Le cache de cours parsés (ADR `2026-07-cache-de-cours-parses`) couvre déjà le coût réel — relancer ne refait que les pages non mises en cache.
- **Seuil d'anomalies au-delà duquel on échoue** : nombre arbitraire, et le premier scrape complet en produit légitimement beaucoup.

## Validé sur données réelles

Le premier scrape GEX (34 cours) a produit 9 anomalies en 4 familles, toutes des lacunes du parseur : sigles suffixés `*`, « Crédits exigés » sans programme, mode `Hybride`, et carte « Crédits » absente.
Aucune n'a interrompu le run, et le journal les nomme cours par cours.
