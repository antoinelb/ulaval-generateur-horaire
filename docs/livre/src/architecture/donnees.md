# Les données

Les snapshots sont produits par le scraper (jamais par l'application), commis dans le dépôt et servis tels quels sur Pages sous `/data`.

## `data/cours.json`

Un seul fichier, tous les cours, triés par code.
Chaque entrée est un `Course` complet : code, titre, crédits (nombre ou `{min, max}` pour un stage à pondération choisie), cycle, préalables (texte brut + arbre parsé), équivalences, et l'offre par saison.

L'offre d'une saison (`SeasonOffering`) porte deux champs dont les `null` sont significatifs :

- `last_offered` : le millésime le plus récent où la saison a offert le cours ; `null` = cours nouveau, jamais encore à l'horaire ;
- `options` : les inscriptions complètes possibles ; `null` = offert mais horaire pas encore publié — distinct de `[]`, aucune combinaison valide.

À côté, `data/cours.manuel.json` est maintenu à la main et jamais écrit par le scraper.

## `data/programmes/{code}-{semestre}.json`

Un fichier par programme **et par millésime** : `B-GEX-A26.json` est le baccalauréat en génie des eaux, version automne 2026.
Le code est le code officiel du répertoire, préfixe de grade abrégé à une lettre (`B-GEX`, `M-GEX`) ; le `slug` (dernier segment de l'URL de la page) est un champ du fichier.
Le millésime est la session *suivant* le scrape : les étudiants conservent la version sous laquelle ils se sont inscrits.

Le contenu est un `Program` : crédits exigés, sessions d'admission (`possible_semester_start`, en lettres `A`/`H`/`E`), cours obligatoires, règles, concentrations, profils, notes en prose, et l'exigence linguistique en champ dédié.

À côté, `data/programmes/{code}.manuel.json` (sans millésime) porte le `cheminement_type` encodé à la main — l'organigramme A1→H8 de référence, qui n'existe sous aucune forme lisible par machine.

## Cycle de rafraîchissement

1. Un cron quotidien (`scrape.yml`) vérifie si la date du jour figure dans `data/dates_scraping.txt` ; sinon il s'arrête.
2. Le scrape complet tourne (catalogue → cours → programmes), throttlé à ~10 requêtes/seconde.
3. Les fichiers sont remplacés **atomiquement** : les snapshots existants restent servis jusqu'au `rename` final.
4. Le commit du bot redéclenche `ci.yml`, qui republie Pages — `pkg`, `data` et ce livre.

Un consommateur peut donc mettre l'URL `data/cours.json` en cache court : le contenu change quelques jours par année, aux dates du fichier.
