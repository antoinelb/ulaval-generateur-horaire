# Le collage Capsule est un tiroir, jamais un modal

## Contexte

Le plan (`docs/plans/2026-08-25-corrections-a-corriger.md`, item 5) décrit le bouton « Charger depuis Capsule » avec « un modal avec zone de collage et le mode d'emploi ».
Cette formulation contredit deux règles déjà en vigueur dans le dépôt : `docs/ux/interface-rules.md` ALR-6 (« No alert modal ever occludes live data ») et LAY-4 (« never block »), et le code lui-même porte déjà la décision — `crates/ui/src/components/mod.rs` documente la région d'alerte comme « a reserved region, never a modal ».
L'organigramme et l'horaire déjà placés restent la donnée vivante que l'étudiant compare pendant qu'il colle son relevé ; un modal les masquerait pendant l'opération la plus sujette à relecture (un long collage HTML, une erreur de format possible).

## Décision

Le collage Capsule est un tiroir non bloquant dans le panneau gauche, à l'intérieur d'`OrganigrammeControls`, juste sous les réglages Début / Sessions / Plafond qu'il réécrit — bâti sur le même patron que `ImportDrawer` (`crates/ui/src/components/panel.rs`) : replié au repos, un bouton bascule son ouverture, le mode d'emploi, la zone de texte et le bilan s'ajoutent tous *sous* les réglages sans jamais les déplacer (LAY-1).
Une erreur de collage garde le tiroir ouvert avec le message ERR-1 en place, exactement comme `ImportDrawer` le fait pour un import par URL raté ; rien n'est jamais masqué derrière un overlay.

## Alternatives rejetées

- **Un modal bloquant, comme l'écrit le plan** : occulterait la grille déjà placée (ALR-6) et bloquerait le reste de l'interface pendant la lecture des instructions ou la correction d'un collage refusé (LAY-4) — aucun modal n'existe ailleurs dans ce dépôt, et `components/mod.rs` documente explicitement ce choix pour la région d'alerte.
- **Un tiroir séparé, hors d'`OrganigrammeControls`** : le collage Capsule réécrit `start`, `study_sessions` et `summers_open` — les mêmes faits que les réglages Début / Sessions ; le tenir à part de ces réglages aurait éloigné la cause de son effet.
