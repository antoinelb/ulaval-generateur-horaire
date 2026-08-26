# Les sigles du relevé absents du catalogue sont ignorés, jamais placés

## Contexte

L'import d'un relevé Capsule (`apply_transcript`, ADR `2026-08-import-de-releve-capsule`) épinglait chaque cours réussi ou en cours sans vérifier le catalogue.
Un relevé réel contient pourtant des sigles que `data/cours.json` ne porte plus — ECN-2901, retiré du répertoire, en est un cas vécu.
Un tel sigle dans `pinned_sessions`/`displayed_placement`/`credited` empoisonne tout le plan : `placement_intake` meurt en `IntakeError::UnknownCodes`, `auto_propose` traite cette erreur comme « rien à placer » et l'organigramme ne se complète jamais, pendant que la vérification affiche une erreur non traduite.
C'était le constat majeur de la revue du 2026-08-25 : `validate_new_code` est la porte unique des ajouts, et l'import Capsule était le premier chemin à la contourner.

## Décision

`apply_transcript` reçoit l'ensemble des sigles connus du catalogue et classe tout code hors de cet ensemble en `Ignored(NotInCatalogue)` — avant toute sémantique de section, échec ou réussite confondus.
Le bilan du tiroir l'affiche « introuvable dans le catalogue » ; rien n'est avalé silencieusement.
Les sessions du relevé continuent d'ancrer `start` et de faire croître l'horizon : l'étudiant les a fréquentées, peu importe ce que le répertoire d'aujourd'hui retient.
`apply_to_plan` purge de plus tout reliquat `NotInCatalogue` déjà présent dans le plan — la corruption laissée par un import antérieur à cette porte — mais jamais un échec replanifié par l'étudiant, qui est un geste volontaire.

## Alternatives rejetées

Laisser passer et attraper l'erreur d'intake à l'affichage : le solveur resterait muet et le bilan aurait déjà annoncé le cours « placé » — un mensonge d'interface.
Ajouter le cours inconnu au catalogue à la volée depuis le relevé : le relevé ne porte ni préalables, ni sessions offertes, ni options — un `Course` fabriqué fausserait la couverture et l'horaire ; la voie propre reste `cours.manuel.json`.
