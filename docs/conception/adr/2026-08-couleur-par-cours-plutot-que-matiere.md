# La teinte d'un cours vient de son rang parmi les cours de l'horaire, plus de sa matière

Date : 2026-08-26

## Contexte

La teinte d'une carte de cours dans la grille horaire hebdomadaire venait de sa matière (ADR `2026-08-couleurs-derivees-de-la-matiere`) : rang alphabétique de la matière parmi toutes les matières distinctes du catalogue courant, divisé par leur nombre, ×360°.
Deux cours d'une même matière (`STT-1900` et `STT-2500`) recevaient donc exactement la même teinte — voulu à l'époque, la teinte identifiait la matière, pas le cours.
Antoine veut maintenant que chaque cours effectivement présent dans l'horaire reçoive sa propre teinte : deux cours ne doivent plus jamais partager une couleur au seul motif qu'ils partagent un préfixe.

## Décision

La teinte vient désormais du rang alphabétique du code du cours — le cours lui-même, pas sa matière — parmi les codes distincts de `schedule.report.courses`, divisé par leur nombre, ×360°.
Même formule OKLCH (clarté `45%`, chroma `0.12` fixes, seule la teinte varie) et même portée (grille horaire hebdomadaire et son export imprimé, qui lisent tous deux la même variable `--course-h`).

`grid_model` (`crates/ui/src/present.rs`) construit `codes: Vec<&str>` à partir de `schedule.report.courses` (triés, dédupliqués) plutôt que d'appeler `panel::subjects(snapshot)` ; `course_hue(codes, code)` remplace `subject_hue(subjects, code)`.
`codes` provient de la même liste que celle parcourue pour assigner la teinte, donc `code` s'y trouve toujours : `codes.binary_search(&code).expect(...)` documente cette certitude plutôt que d'ajouter une branche `None` inatteignable qu'il faudrait par ailleurs feindre de tester pour la couverture à 100 %.

`panel::subjects`/`panel::subject_of` restent inchangées, encore utilisées ailleurs (le sélecteur de la règle libre, `panel.rs`).

Cette décision remplace uniquement la source de la teinte de l'ADR `2026-08-couleurs-derivees-de-la-matiere` : le choix de l'OKLCH à clarté et chroma fixes, l'astuce du dégradé plat posé sur un fond blanc opaque, et le champ `Block.hue: f32` restent valides tels quels et ne sont pas repris ici.

## Alternatives rejetées

- **Garder le regroupement par matière** — rejeté, contredit directement la demande que chaque cours de l'horaire soit distinguable.
- **Hachage du code** — rejeté pour la même raison que le hachage de matière l'était déjà : une teinte imprévisible d'un cours à l'autre, alors que le rang alphabétique reste déterministe et stable.
