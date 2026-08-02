# La scolarité préparatoire devient une règle calculée du programme

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

Les cours préuniversitaires (0xxx, ex. MAT-0150) n'apparaissent sur aucune page de programme : ils se cachent dans les arbres de préalables des cours obligatoires (`2026-07-prealables-preuniversitaires-en-expression`).
Un étudiant ne peut donc pas voir quels cours d'appoint son programme peut exiger.

## Décision

- Au scrape (`ulaval-scraper program`), chaque `core::Program` gagne une règle appendée en dernier, titrée « Scolarité préparatoire », listant tous les codes 0xxx atteignables **transitivement** depuis les `mandatory` du programme (portée programme seulement, pas les concentrations/profils) via les arbres de préalables de `data/cours.json`.
- Pas de contrainte min/max : les cours à faire dépendent du dossier collégial de chacun — la règle existe pour que les bons cours d'appoint soient pris, pas pour compter.
- Les branches OU (`any`) sont aplaties : tout code 0xxx nommé entre dans la règle, quelle que soit sa branche.
- Les feuilles `raw` (sigles cégep « BIO-NYA », prose) et `program_credits` ne sont pas des codes : ignorées.
- Un code 0xxx absent du snapshot est quand même listé — le préfixe 0 est le seul signal de cycle disponible (`2026-07-presomption-limitee-au-preuniversitaire`).
- Aucun cours atteignable ⇒ aucune règle (omission plutôt que liste vide, même convention que les champs skippés en sérialisation) — les snapshots sans scolarité préparatoire restent byte-identiques.
- Le calcul est pur dans `core::preparatory` (`preparatory_rule(mandatory, courses)`), la marche est une worklist bornée (pas de récursion) ; le CLI l'applique après le fetch et exige `data/cours.json` **avant tout fetch** : échec immédiat sur le modèle de `read_catalogue` (« Run `ulaval-scraper courses` first. »).
- Sur les données réelles, B-GEX donne `[BIO-0150, CHM-0150, CHM-0160, CHM-0170, MAT-0130, MAT-0150, MAT-0260, PHY-0150]`.

## Alternatives rejetées

- **Calcul dans le parseur** : il faudrait les données de cours (IO) dans `parser::program::parse` ; la pureté HTML→Program et les paires de fixtures gelées seraient cassées. L'enrichissement en aval (comme `extract_language_requirement`, mais côté CLI) garde les fixtures parseur inchangées.
- **Contrainte comptée** : un min/max serait inventé — rien sur les pages ne le donne, et le bon nombre dépend de l'étudiant.
- **Règle vide émise** : du bruit dans chaque programme sans scolarité préparatoire, et tous les snapshots existants changeraient.
- **Garder la structure ET/OU dans la règle** : une règle est une liste plate ; l'arbre exact reste disponible sur chaque cours.
- **Calcul côté UI** : la règle est une donnée du snapshot, lue telle quelle par le vérificateur de couverture et l'UI (logique métier dans `core`, jamais dans la vue).
- **Un seul saut (non transitif)** : les préuniversitaires s'enchaînent (PHY-0250 exige PHY-0150) ; un seul saut listerait un cours dont le préalable manque.
