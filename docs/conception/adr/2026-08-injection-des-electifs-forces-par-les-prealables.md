# Injection des électifs forcés par les préalables d'un obligatoire

Date : 2026-08-15

## Contexte

Au B-GMC A26, l'obligatoire GMC-3002 exige GLO-1901 — un électif de la Règle 1 (à choix : GLO-1901 ou IFT-1903).
La présomption du solveur étant limitée au préuniversitaire (ADR `2026-07-presomption-limitee-au-preuniversitaire`), GLO-1901 non choisi rendait GMC-3002 `unsatisfiable-prerequisites`, donc « aucun placement possible » pour tout le programme (`CORRECTIFS-AMONT.md` item 13).
Or aucun étudiant ne finit ce bac sans GLO-1901 : l'électif est obligatoire de fait.
Décision d'Antoine (2026-08-15) : injection automatique.

## Décisions

- **Dans l'intake, pas dans `place`** : `placement_intake` étend la liste de cours (`inject_forced_electives`) avant toute recherche. Le contrat de `place` et les fixtures d'organigramme (niveau `PlacementRequest`) sont intouchés — la référence versionnée n'a rien à réimplémenter, l'injection se prouve par les tests d'intake.
- Un code est injecté quand il est **forcé** — l'arbre de préalables d'un candidat est insatisfiable sans lui, même en accordant tout opérande qui pourrait un jour tenir (réussi, candidat, préuniversitaire, ou électif listé injectable) — **et** qu'une règle du programme le liste (vrai électif). Transitif : l'arbre de l'injecté peut forcer d'autres électifs (point fixe borné par la taille du bassin ; aplatissement sans récursion, budget de 10 000 nœuds comme le solveur — un arbre au-delà n'injecte rien et reste au solveur, qui le refuse bruyamment).
- **L'injection ne choisit jamais** : un choix entre deux électifs listés (`any`) ne force ni l'un ni l'autre et reste bloqué ; un code forcé hors programme reste `unsatisfiable-prerequisites` (comportement inchangé).
- **Jamais silencieux** : `PlacementIntake.injected` remonte les codes, propagés au rapport wasm (`OrganigrammeReport.injected`), au protocole worker (`Report.injected`) et à l'UI Dioxus, qui **adopte** les injectés dans `plan.electives` au moment d'appliquer la proposition (dans le même `edit_plan`, donc annulable d'un geste) avec une alerte les nommant — la leçon de la purge préparatoire : le plan doit posséder ce que la grille montre.
- Effet de bord assumé sur `verify` : un organigramme assemblé sans l'électif forcé devient une question incomplète (« il manque une session pour GLO-1901 »), qui nomme le remède, plutôt qu'un placement muettement insoluble.

## Alternatives rejetées

- Blocage avec remède nommé (proposé en premier) : honnête mais laisse chaque UI implémenter le geste ; Antoine a préféré que le solveur assume l'obligatoire de fait.
- Documenter seulement : le geste réparateur JS (« placez-le puis relancez ») fonctionnait mais laissait « proposez un organigramme » échouer sur un programme neuf.
- Présumer (`assumed`) l'électif au lieu de le placer : produirait un organigramme mensonger — le cours doit réellement être suivi.
- Trancher les `any` (injecter une alternative arbitraire) : le solveur choisirait à la place de l'étudiant.
