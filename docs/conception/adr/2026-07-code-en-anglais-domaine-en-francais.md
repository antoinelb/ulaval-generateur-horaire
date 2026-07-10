# Langue du code : anglais pour le code, français pour le domaine

Date : 2026-07-10

## Contexte

Le domaine est entièrement francophone (Université Laval) : la prose, la documentation et l'interface parlent de `cours`, `préalables`, `matière`, `cheminement`, `session`, `pavillon`.
Rien n'imposait jusqu'ici la langue des *identifiants* du code, des messages d'erreur ni des clés de données.

Les cas de test du parseur — le contrat de sortie — utilisent pourtant déjà des clés anglaises : `title`, `credits`, `prerequisites`, `mandatory`, `rules`, `raw`, `seasons`, `slots`, `profiles`.
La prose de `CLAUDE.md` et de `project_plan.md` décrivait au contraire ces mêmes champs en français (`titre`, `crédits`, `préalables`, `obligatoires`, `regles`, `sous_groupes`, `brut`, types `Cours`/`PlageHoraire`/`Horaire`), en contradiction avec les fixtures.

## Décision

Séparer explicitement la langue du domaine de la langue du code :

- **Français** : vocabulaire métier dans la prose, la documentation et le texte affiché à l'utilisateur.
- **Anglais** : tous les identifiants (variables, fonctions, types), les messages d'erreur, et les clés de données sérialisées (JSON).

Les cas de test existants font foi (`title`, `credits`, `prerequisites`, …).
La documentation est réalignée sur ce contrat : `CLAUDE.md` § Domain quick facts et les types du domaine de `project_plan.md` (`Course`/`Section`/`TimeSlot`/`Schedule`) réécrits en identifiants/clés anglais.

Exception laissée en suspens : `cheminement_type` (organigramme A1→H8, encodé à la main, GEX seulement) reste en français, car « cheminement » est un terme métier sans équivalent anglais net et n'est pas une sortie du parseur.
Son renommage éventuel fera l'objet d'une décision séparée.

## Alternatives rejetées

- **Tout en français (identifiants et clés compris)** : cohérent avec le domaine, mais Rust pousse aux identifiants ASCII (accents fragiles dans `crédits`/`préalables`), et cela aurait exigé de réécrire les fixtures anglaises déjà en place. Plus de travail pour un résultat moins idiomatique.
- **Statu quo (rien de fixé)** : laissait se perpétuer la contradiction fixtures anglaises / doc française ; chaque nouveau champ aurait rouvert la question.
