# Le stage obligatoire compte dans le rapport de couverture

Date : 2026-08-30

## Contexte

Un étudiant en génie mécanique qui place GMC-1590 (stage optionnel) sans GMC-2580 (le stage exigé pour diplômer) voyait la règle « Stages » **verte**.
La prose promue en règle par l'ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle` dit pourtant deux choses distinctes : « l'étudiant doit réussir le stage de formation pratique GMC-2580 pour obtenir son diplôme. Il peut également suivre trois autres stages de formation pratique **optionnels** ».
La règle promue ne porte qu'une contrainte `{"type": "course", "min": 1, "max": 8}` sur les quatre sigles, et `over_or` ne connaît que `total >= min` : un stage optionnel suffisait à la satisfaire.
Les cinq bacs de génie sont touchés ; les grilles officielles ne plaçaient que le stage obligatoire, donc le défaut ne se manifestait qu'au premier étudiant qui s'écartait de la grille.

L'invariant « le premier sigle listé est le stage obligatoire » existait déjà et était lu à deux endroits — `intake::course_list` (ADR `2026-08-stage-obligatoire-et-scolarite-preparatoire-dans-lintake`) et `is_required_stage_course` de l'export organigramme.
Le seul consommateur qui l'ignorait était le vérificateur de couverture.

## Décision

- Une seule définition de l'invariant : `core::program::mandatory_stage(rule) -> Option<&str>`, qui rend le premier sigle d'une règle intitulée « Stages » portant une contrainte `Course` de minimum positif et une liste non vide.
  Aucune autre règle du répertoire ne gagne de sémantique nouvelle ; une règle qui n'exige rien ou ne nomme rien rend `None`.
  Les deux lecteurs existants s'y branchent, et la règle cesse de pouvoir dériver entre eux.
- `rules::evaluated` déclasse un verdict `Satisfied` en `Incomplete` (`Missing::Count { count: 1 }`) quand le stage obligatoire n'est pas compté.
  `OverMax`, `Incomplete` et `Uncounted` restent intacts : chacun est déjà en faute pour une raison propre, et masquer un dépassement derrière le stage manquant coûterait à l'étudiant la seule action qu'il peut poser (ADR `2026-08-depassement-de-regle-en-statut-rouge`).
- Le panneau dit lequel : la ligne du stage obligatoire porte « - exigé pour diplômer » (même mécanique que `mark_required_language_course` pour ANL-2020), et le libellé de contrainte devient « GMC-2580 + 0–7 optionnels » au lieu de « 1–8 parmi », qui se lisait « n'importe lequel des quatre » — la lecture même qui masquait le défaut.
- Fixture témoin `tests/fixtures/test_cases/rules/stage-optionnel-ne-satisfait-pas.json` : B-GMC tel que livré, sélection `["GMC-1590"]`, attendu `incomplete` avec `GMC-2580` en candidat.
  Les dix-huit autres fixtures `rules/` restent bit-pour-bit identiques : aucune ne sélectionnait de stage.

## Alternatives rejetées

- **Un champ `Rule.required` sérialisé** : plus honnête en données, mais les 24 snapshots de programmes livrés ne le portent pas ; tant qu'ils ne sont pas re-scrapés, `serde(default)` les dégraderait en silence — exactement le défaut qu'on corrige. L'ordre déjà écrit dans les fichiers suffit.
- **Deux règles séparées, « Stage obligatoire » et « Stages optionnels »** : casse `PlacementIntake::stages` (qui restreint les stages aux étés par la liste complète), les cartes d'été de l'export, et introduit une dépendance à l'ordre d'évaluation par le mécanisme `claimed` d'`2026-08-un-cours-compte-dans-une-seule-regle-par-portee`. Exige aussi de re-scraper.
- **Ne lister que le stage obligatoire dans la règle** : déjà rejeté par l'ADR de promotion, et pour la même raison — les stages optionnels cesseraient d'être des candidats connus, donc d'être restreints aux étés.
- **Une variante `Missing::Course { code }` nommant le stage dans le rapport** : `Missing` perdrait `Copy`, la surface TypeScript changerait, et les dix-huit fixtures seraient à régénérer — pour une information que la note de la règle, le marqueur de ligne et le libellé de contrainte portent déjà.
