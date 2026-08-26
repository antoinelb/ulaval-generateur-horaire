# L'import Capsule ouvre la concomitance quand l'historique l'exige

> **Remplacée le 2026-08-26 par `2026-08-etoile-de-concomitance-au-parsing`.**
> L'étoile du répertoire est maintenant conservée au parsing et honorée par feuille : la détection et l'ouverture du réglage global n'ont plus d'objet, et `history_needs_concomitance` / `CONCOMITANCE_NOTE` sont retirées. Le reste du fichier décrit la mesure provisoire, gardé pour la trace.

## Contexte

La grammaire des préalables perd le `*` du répertoire (« peut être suivi en concomitance ») : l'arbre parsé de GEX-3001 exige GCI-2010 strictement avant, alors que la page permet la même session.
Un relevé réel suit ces paires en concomitance (GEX-3001 avec GCI-2010 en H26, MAT-2910 avec IFT-1903 en H25) : le solveur refusait le propre passé de l'étudiant tant que la case « Permettre un préalable en concomitance » restait décochée (relevé d'Antoine, 2026-08-26).

## Décision

`capsule::history_needs_concomitance` détecte la preuve : un cours épinglé dont les préalables échouent contre les sessions antérieures seules mais passent quand les cours de sa propre session comptent comme acquis.
L'import ouvre alors le réglage global `concomitant`, dans le même acte annulable, et ne le ferme jamais — même porte à sens unique que `summers_open`.
Le bilan le dit d'une phrase (`CONCOMITANCE_NOTE`).
Mesure provisoire : le plan `docs/plans/2026-08-26-concomitance-au-parsing.md` garde l'étoile au parsing et rendra cette détection inutile.

## Alternatives rejetées

- **Exempter des préalables tout le passé du relevé** : réglait aussi les dérives de répertoire (GCI-1011), mais masque de vraies incohérences de données — Antoine a préféré les voir nommées (décision 2026-08-26).
- **Parser l'étoile immédiatement** : le bon geste, mais plus large (grammaire, modèle, solveur, fixtures, re-scrape, aval JS) — planifié séparément plutôt que bâclé ici.
- **Laisser l'étudiant cocher lui-même** : le refus de son propre passé, sans explication, est exactement le symptôme rapporté.
