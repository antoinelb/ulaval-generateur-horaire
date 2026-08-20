# Les réglages voyagent avec l'instantané, seul `start` ensemence un document neuf

## Contexte

L'instantané par (programme, millésime) (ADR `2026-08-instantane-de-plan-par-programme-et-millesime`) pose la question des champs « transversaux » du `Plan` : plafond, sessions, étés, concomitance, préparatoire, corrections de préalables.

## Décision

- L'instantané est le `Plan` **entier** : « revenir redonne exactement le même panneau » l'exige, et le plafond, l'horizon ou les corrections de préalables sont des faits du programme étudié, pas de l'étudiant en général.
- Un document **neuf** (aucune étagère) part de `Plan::default()` + le `ProgramChoice` cliqué en emportant **seulement `start`** (`state::fresh_plan`) : la session d'entrée est l'identité calendaire de l'étudiant, et le défaut `A2026` pourrira avec le temps.
- Les cours manuels (`gh.v1.cours-manuels`) restent globaux : ils prolongent le catalogue, pas le document (même raisonnement que l'ADR `2026-08-bouton-tout-reinitialiser`).
- `View` reste global (`gh.v1.view`), remis à `View::default()` à chaque bascule : garder la recherche ou la session de l'autre programme ressusciterait des résultats périmés.

## Alternatives rejetées

- **Réglages globaux partagés entre programmes** : régler 11 sessions pour le B-GMC casserait le B-GEX au retour.
- **Emporter aussi plafond/étés dans le document neuf** : un réglage monté pour un programme chargé n'a rien à dire d'un programme jamais ouvert.
