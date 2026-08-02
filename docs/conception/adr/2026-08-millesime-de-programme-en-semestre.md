# Le millésime d'un programme en semestre, et les sessions d'admission

Date : 2026-08-02

## Contexte

L'ADR `2026-07-annee-de-programme-selon-la-date-de-scrape` datait chaque snapshot de programme d'une année civile (`year: u16`, fichier `{code}-{year}.json`).
Une année est pourtant plus grossière que le rythme réel : un programme peut changer entre deux sessions, et une cohorte s'inscrit à une session précise, pas à une année.
Par ailleurs, la page de programme affiche un bloc « Sessions d'admission » (Automne/Hiver, plus Été pour la maîtrise) que rien ne capturait — l'UI en aura besoin pour proposer les sessions de départ possibles.

## Décision

1. **Le millésime devient un semestre** : `Semester { season, year }` dans `core`, sérialisé en une chaîne `A26`/`H27`/`E27` — la lettre de saison que le nommage de session (`a2026`) utilise déjà, en majuscule, plus deux chiffres d'année.
   `Program.year` devient `Program.semester`, le fichier `data/programmes/{code}-{semestre}.json` (ex. `baccalaureat-en-genie-des-eaux-A26.json`).
   Le format vit à un seul endroit : `Display` sur `Semester`, réutilisé par serde (`collect_str`), le nom de fichier et le flag CLI (`FromStr`).
2. **Règle de datation** (`semester_after` dans `cli.rs`, remplace `academic_year`) : le run livre la session qui le **suit** — janvier–avril → `E` (été, année courante), mai–août → `A` (automne, année courante), septembre–décembre → `H` (hiver, **année civile suivante**).
   Même arithmétique `civil_from_days`, même plancher au jour zéro pour une horloge d'avant 1970 (→ `E70`, visible dans le nom de fichier).
3. **`--semester` remplace `--year`** sur la sous-commande `program` (ex. `--semester A26`) ; les fixtures sont gelées sous `A26`.
   Le repli par slug reconnaît les deux suffixes — `A26` et l'ancien `2026` à quatre chiffres — pour que les fichiers d'avant la migration se replient encore sur leur slug.
4. **Nouveau champ `possible_semester_start`** sur `core::Program` : la liste des sessions d'admission lue du bloc `div.admission--liste-sessions` de la page, sérialisée en lettres (`["A", "H"]`), dans l'ordre de la page.
   Bloc absent, bloc vide ou libellé hors Automne/Hiver/Été : anomalie signalée (jamais silencieux), le programme est gardé et la liste reste vide — une liste d'admission est de l'affichage, pas de la structure.
   Le champ est `default` + omis quand vide : les fixtures qui embarquent un programme sans lui restent valides.
5. **Migration** : les sept snapshots `{code}-2026.json` (scrapés en juillet 2026, donc millésime `A26` sous la nouvelle règle) sont re-scrapés en `{code}-A26.json` — le re-scrape fournit du même coup le `possible_semester_start` réel — puis les anciens fichiers sont supprimés.

## Alternatives rejetées

- **Sérialiser `possible_semester_start` avec la forme anglaise de `Season`** (`["fall", "winter"]`, comme `data/cours.json`) : incohérent avec le format `A26` du millésime voisin ; les lettres sont le vocabulaire choisi pour tout ce qui nomme une session côté programme.
- **Année sur quatre chiffres (`A2026`)** : le gain de lisibilité est nul dans un horizon d'un siècle et la forme courte est celle demandée ; `FromStr` rejette explicitement `A2026`.
- **Garder `year` à côté de `semester`** : redondant — la saison et l'année se lisent dans la même chaîne, et un seul champ ne peut pas se désynchroniser.
- **Un champ optionnel plutôt qu'une anomalie pour un bloc admission absent** : contraire à l'invariant « jamais d'entrée non reconnue avalée » ; les six pages gelées portent toutes le bloc, son absence est un signal.
