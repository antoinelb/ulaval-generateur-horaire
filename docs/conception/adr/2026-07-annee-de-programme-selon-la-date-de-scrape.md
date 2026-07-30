# L'année d'un programme selon la date de scrape

Date : 2026-07-29

## Contexte

Un programme est modifié en hiver pour l'automne suivant, à une date imprécise que la page n'annonce nulle part.
Or un étudiant reste régi par la version de l'année de son inscription : écraser le snapshot d'un programme à chaque scrape perd la version sous laquelle les cohortes précédentes cheminent.
Il faut donc conserver les millésimes — comme les cours le font déjà par session (`a2026.json`) — alors que `data/programmes/{code}.json` n'en gardait qu'un.

## Décision

1. **Un fichier par programme et par année** : `data/programmes/{code}-{year}.json`, et le champ `year: u16` dans `core::Program` (le fichier reste autoportant).
   La page ne portant pas l'année, `parser::program::parse(html, year)` la reçoit de l'appelant.
2. **Règle de datation** (`academic_year` dans `cli.rs`) : un scrape de **mai à décembre** livre l'année civile courante, de **janvier à avril** l'année précédente.
   Le scraper roule au début de chaque session ; le run de mai (session d'été) est le premier qui puisse voir la version modifiée en hiver — la borne de mai range chaque capture sous le millésime qu'elle décrit.
   Le calcul calendaire est l'arithmétique d'ères de Howard Hinnant (`civil_from_days`), sans dépendance ni chemin d'erreur ; une horloge d'avant 1970 est plafonnée au jour zéro, visible dans le nom de fichier plutôt qu'avalée.
3. **`--year` optionnel** sur la sous-commande `program` : la valeur explicite fige les re-runs et les tests à comparaison exacte (les fixtures sont gelées sous 2026) ; sans elle, la règle s'applique à l'horloge.
4. **Migration** : les sept snapshots existants (scrapés en juillet 2026) deviennent `{code}-2026.json` avec `"year": 2026` inséré.
   Le futur `{code}.manuel.json` (cheminement type, maintenu à la main) reste **sans année** : une seule version de référence à la fois.

## Alternatives rejetées

- **Borne à septembre** (année scolaire officielle) : le run de mai rangerait la version nouvellement publiée sous l'année précédente — précisément la confusion à éviter.
- **Année lue sur la page** : elle n'y figure pas ; toute heuristique de contenu serait plus fragile que la date de capture.
- **Dépendance calendrier (`chrono`/`time`)** : dix lignes d'arithmétique suffisent pour année et mois ; une dépendance entière pour deux champs ne se justifie pas.
- **Écraser et versionner par git** : l'historique git n'est pas interrogeable par l'application servie statiquement ; les millésimes doivent exister comme fichiers.
