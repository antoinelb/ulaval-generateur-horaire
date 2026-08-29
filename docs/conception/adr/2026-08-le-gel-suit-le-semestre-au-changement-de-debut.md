# Le gel suit le semestre au changement de Début

Date : 2026-08-29

## Contexte

`Plan.frozen` est un ensemble d'*indices* de session (ADR `2026-08-sessions-gelees-generalisent-les-completees`), et changer « Début » renomme les sessions sans déplacer personne (ADR `2026-08-le-debut-n-herite-pas-d-un-placement-hors-saison`).
Sans correction, un gel posé sur « A26 » glissait sur « H1-H27 » — la même classe de mensonge que `set_start` venait de fermer pour les sièges, appliquée cette fois à la contrainte qui protège le passé réglé de l'étudiant.

## Décision

Dans `state::set_start`, le gel voyage par **semestre** : chaque indice gelé est traduit en son semestre sur l'ancienne ligne du temps, puis retrouvé sur la nouvelle — jusqu'à `MAX_STUDY_SESSIONS`, le plancher d'horizon (`binding_slot`) refaisant grandir l'horizon par-dessus un semestre gelé que la portée courante manquerait, exactement comme pour une épingle.
Le bloc gelé voyage **entier** : les sièges affichés et les épingles assis dans une session gelée survivante suivent le même déménagement — déplacer le gel sans son contenu viderait la session même qu'il protège.
Un semestre gelé que la nouvelle ligne du temps ne tient plus (avancer le Début au-delà) perd son gel, **nommé au bilan** (`StartMove.unfrozen`, phrasé par `present::start_move_note`) ; un indice qu'aucune ligne du temps n'a jamais tenu (sauvegarde corrompue) part aussi, nommé par son numéro.

Frontière assumée : `chosen`, `manual` et `special` restent indexés comme avant — le comportement préexistant de `set_start` pour toutes les sessions, gelées ou non ; à re-juger si un cas réel s'y coince.

## Alternatives rejetées

- **Laisser l'indice tel quel** : le gel affiché comme vrai sur le mauvais semestre — le mensonge que cet ADR ferme.
- **Retirer tous les gels au changement de Début** : simple, mais punit un changement accidentel (pourtant annulable) en jetant l'information la plus précieuse du plan ; le remappage est exact et guère plus coûteux.
- **Remapper aussi les sièges non gelés par semestre** : contraire à l'arbitrage de `2026-08-le-debut-n-herite-pas-d-un-placement-hors-saison` — un placement proposé se recalcule, seul le passé réglé mérite le déménagement.
