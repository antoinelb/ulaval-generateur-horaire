# Le nombre de sessions par défaut suit les crédits du programme

## Contexte

`DEFAULT_STUDY_SESSIONS = 8` ouvrait tout nouveau document à huit sessions, quel que soit le programme choisi.
Un bac de 90 crédits (six sessions à 15 cr) s'ouvrait donc avec deux sessions d'horizon en trop, invitant à un premier contact vide ou déroutant plutôt qu'à un cheminement plausible.

## Décision

`state::default_study_sessions(credits_required)` calcule `ceil(credits_required / 15)`, borné à `[2, 16]` — les mêmes bornes que le champ « Sessions » du panneau.
15 crédits est la charge d'une session à temps plein : 120 cr (B-GEX) donne 8, 90 cr donne 6.
Un `credits_required` non positif (le champ est un `i64`, le fichier pourrait porter n'importe quoi) retombe sur `DEFAULT_STUDY_SESSIONS`, qui reste la valeur de repli — jamais retiré.
Seul un document **neuf** (aucune étagère à restaurer) en hérite : `fresh_plan` prend désormais `study_sessions` en paramètre, ensemencé au clic « Choisir » depuis le millésime précis choisi (jamais `ProgramVintages::credits_required`, qui nomme le millésime le plus récent).
Le réglage reste un champ ordinaire du `Plan`, modifiable par l'étudiant comme avant.

## Alternatives rejetées

- **Garder la constante `8`** : plausible pour le B-GEX seul, faux dès qu'un programme de crédits différents s'ouvre.
- **Dériver de `credit_cap`** : le plafond est un réglage de rythme (ADR `2026-08-plafond-par-defaut-17-credits`), pas un fait du programme ; les deux varient indépendamment.
- **Changer le défaut de restauration serde** : la tolérance de sauvegarde endommagée (`persist::restore`) doit rester `8`, indépendante du programme — sinon un même fichier corrompu restaurerait des horizons différents selon le programme visé au moment du chargement.

## Renvoi

Cette exception à « seul `start` est emporté » est documentée dans l'ADR `2026-08-reglages-transversaux-dans-linstantane`.
