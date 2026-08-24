# Les ententes nomment leur portée dans tous les libellés

Date : 2026-08-23

## Contexte

Les clés `p/Règle 1`, `c/Règle 1` et `f/Règle 1` étaient distinctes, mais le menu affichait trois fois `Règle 1`.
Le journal annulable reprenait aussi le titre nu.

## Décision

Les libellés visibles prennent les formes `Programme — Règle 1`, `Concentration « … » — Règle 1` et `Profil « … » — Règle 1`.
Le menu, l'historique et les erreurs emploient le même libellé, sans changer la clé persistée.

## Alternatives rejetées

- **Afficher la clé technique** — `p/`, `c/` et `f/` ne sont pas du vocabulaire étudiant.
