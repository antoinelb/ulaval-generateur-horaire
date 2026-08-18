# Correction des préalables par millésime d'admission

Date : 2026-08-17

## Contexte

Les préalables d'un cours changent d'une version de programme à l'autre, et un étudiant reste régi par la version de son admission — le principe déjà acté pour les programmes par `2026-08-plusieurs-millesimes-de-programme-offerts`.
Mais `data/cours.json` est un snapshot unique de 8 834 cours dont la clé `prerequisites` est hors saison et hors millésime.
Un étudiant admis en A24 voyait donc les *règles* de A24 et les *préalables* de 2026 : le solveur refusait des placements qui lui étaient ouverts, ou lui imposait un ordre qui n'était pas le sien.

Aucune ADR ne consignait ce trou.
Les seules échappatoires étaient détournées : recréer le cours en manuel sans préalables, ou l'épingler à la main.
`Plan.rule_grants` (ADR `2026-08-entente-cours-regle-et-scolarite-preparatoire`) est bien une dérogation, mais elle porte sur l'appartenance à une règle de couverture, jamais sur un préalable.

## Décisions

- **Une correction est un `Prerequisites` de remplacement, exprimé en texte source** — la même grammaire que le répertoire, parsée par le même parseur (déplacé dans `core`, ADR `2026-08-parseur-de-prealables-deplace-dans-core`).
  Chaîne vide = « ce cours n'a aucun préalable », une réponse et non une absence.

- **Deux couches, une seule fonction.**
  `data/cours.manuel.json` gagne une surcouche `vintages: {"A24": {"prerequisites": {"GCI-2000": "…"}}}`, maintenue par la direction et appliquée automatiquement à tout étudiant de ce millésime ; `Plan.prereq_overrides` porte les corrections de l'étudiant, qui l'emportent sur celles du fichier.
  Les deux fusionnent dans `data::effective_overrides` avant `core::apply_prereq_overrides`.

- **La correction est appliquée sur les `Course` eux-mêmes, en amont du solveur.**
  Seuls trois endroits lisent `Course::prerequisites` (`organigramme::flat_tree`, `intake::parsed_tree`, `preparatory::preparatory_rule`) : réécrire le champ les couvre tous sans qu'aucune signature de solveur, aucun schéma `deny_unknown_fields` ni aucune fixture gelée ne bouge.

- **Réécrite en place, jamais par clone.** `Snapshot::overridden` garde l'original des cours touchés, indexé par position, si bien qu'appliquer un jeu de corrections coûte le nombre de corrections et non les 8 834 cours — et qu'un nouveau jeu remplace toujours le texte du répertoire, jamais la correction précédente.

- **La clé de millésime est comparée exactement** à `ProgramChoice.semester`, sans logique d'intervalle.
  Une clé qui ne nomme aucune session est remontée en avertissement : elle ne corrigerait personne, en silence.

- **Rien n'est appliqué en silence.** `OverrideNote` remonte une expression illisible (le cours garde alors ses préalables officiels — une correction ne retombe **jamais** sur `Prerequisites::Raw`, cette variante préserve ce que l'université a écrit, elle n'accueille pas une faute de frappe), un sigle absent du catalogue, et le fait que le répertoire ait bougé depuis la correction.

- **Le lien de partage devient `ShareV2`** — `ShareV1` gelée est encore décodée, mais seul V2 est écrit : un lien qui perdrait les corrections montrerait au destinataire un autre verdict que celui de l'expéditeur.

- **Le worker reçoit les mêmes corrections** par un troisième argument de `init_snapshot`, et redémarre quand elles changent — le motif déjà en place pour un cours manuel ajouté.

## Limite connue

`preparatory::preparatory_rule` s'exécute dans le scraper et son résultat est figé dans le snapshot de programme.
Une correction de préalable ne la recalcule pas : corriger un préalable qui changerait les cours 0xxx atteignables ne déplacera pas la règle « Scolarité préparatoire ».
Acceptable — les préalables préuniversitaires sont stables et la règle se résume à une case.

## Alternatives rejetées

- **Un champ `waived` passé dans le solveur** (`PlacementRequest` → `OrganigrammeInput` → `PrerequisitesInput`) : trois schémas gelés, les fixtures d'organigramme et la référence B touchés, pour un pouvoir identique.
- **Une simple case « ces préalables ne s'appliquent pas à moi »** : le solveur cesserait alors d'ordonner le cours et pourrait le placer en A1 ; l'étudiant devrait l'épingler à la main.
- **Une liste de sigles cochables** : retirer une feuille d'un OU imbriqué ne veut pas dire la même chose que d'un ET ; l'arbre reconstruit ne serait pas celui que l'étudiant croit.
- **Un millésime de préalables scrapé** (`data/cours-A24.json`) : aucune source. Le scraper ne voit que la page d'aujourd'hui ; rien ne peut être reconstitué rétroactivement.
- **Étendre les équivalences** pour qu'un cours renuméroté satisfasse le préalable exigeant son successeur : hors portée (Antoine, 2026-08-17) — c'est un changement de sémantique du solveur, qui toucherait les fixtures gelées.
