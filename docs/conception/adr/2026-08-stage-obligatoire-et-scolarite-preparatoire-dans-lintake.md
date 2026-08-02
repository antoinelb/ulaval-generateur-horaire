# Le stage obligatoire et la scolarité préparatoire entrent dans la liste d'intake

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

`intake::course_list` ne lisait que `program.mandatory` : ni les cours d'appoint de la règle « Scolarité préparatoire » (`2026-08-regle-scolarite-preparatoire`) ni le stage obligatoire de la règle « Stages » (`2026-08-stage-obligatoire-en-prose-promu-en-regle`) n'atteignaient le placement.
Les 0xxx non réussis étaient *présumés satisfaits* (`Solution.assumed`) au lieu d'être placés avant leurs dépendants.

## Décision

- `course_list` chaîne, dans l'ordre : cours de la règle « Scolarité préparatoire » (liste entière) → `mandatory` → **premier sigle** de la règle « Stages » (le stage obligatoire, premier par l'ADR de promotion) → electives → réussis ; dédupliqué comme avant.
- Les règles sont retrouvées par leurs titres, promus en constantes publiques : `PREPARATORY_RULE_TITLE` (`core::preparatory`) et `STAGES_RULE_TITLE` (`core::program`, désormais la source du parseur scraper aussi).
- Les trois profils d'étudiants passent par `passed`, sans autre mécanisme : scolarité préparatoire entièrement faite (tous les 0xxx dans `passed`, rien de placé), partiellement faite (le reste est placé avant ses dépendants), à faire au complet (tout est placé). Un étudiant *exempté* (dossier collégial) marque les cours réussis — même sémantique solveur.
- Les stages optionnels restent des electives : les ajouter d'office sur-placerait des cours que l'étudiant ne fera jamais.
- `PlacementIntake` gagne `stages` : l'intersection de la liste de la règle « Stages » avec les cours **sélectionnés** (pas la liste entière : un stage écarté en `set_aside` faute de snapshot ne doit pas atteindre `PlacementRequest.stages`, qui exige un `Course` par stage). Un stage optionnel tapé en elective est donc lui aussi restreint aux étés.
- Un 0xxx absent de `data/cours.json` dégrade en `set_aside` par le chemin existant (dérivé du programme, pas explicite) — surfacé, jamais bloquant.
- Conséquence assumée : la sélection d'intake contient toute la liste, donc le rapport de couverture piloté par l'intake compte toujours la règle préparatoire en entier — le comptage partiel ne s'observe que sur une sélection construite à la main (fixtures `rules/`).

## Alternatives rejetées

- **Tous les stages de la règle placés d'office** : min 1 / max 8 — seuls le premier est obligatoire selon la prose ; les optionnels sur-contraindraient chaque placement.
- **Détection du stage obligatoire par la prose** (re-parser la note) : l'ordre mandatory-first est déjà garanti par l'ADR de promotion ; `first()` suffit.
- **Ne placer que les 0xxx « nécessaires »** (résoudre les branches OU par étudiant) : l'exemption dépend du dossier collégial que `core` ne voit pas ; `passed` est le canal existant et suffisant.
- **Un champ dédié `Program.stages`** : la règle « Stages » porte déjà la liste ; un second emplacement créerait deux sources de vérité.
