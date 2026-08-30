# Un bouton « Tout geler » dans la barre du haut

Date : 2026-08-30

> Suite : le libellé perd son ❄ (« Tout dégeler » tout court) et l'écart de `.header-reset` passe à 1 rem — ADR `2026-08-tout-geler-sur-une-ligne-et-verdict-ecourte`.
> Suite : la bascule élargissait le bouton de 15 px et déplaçait « Réinitialiser »; une largeur plancher de 6,75 rem la fige (LAY-1) — ADR `2026-08-largeur-constante-du-bouton-tout-geler`.

## Contexte

Geler une session la ferme au solveur sans la fermer à l'étudiante (ADR `2026-08-sessions-gelees-generalisent-les-completees`) : il n'y ajoute ni n'en déplace plus rien, elle continue d'y toucher librement.
Le geste n'existait qu'au détail, une session à la fois.
Le cas courant — « mon organigramme me convient, arrête de le remanier » — demandait alors autant de clics que l'horizon a de sessions, étés compris, soit douze pour un bac de huit sessions d'étude.

Antoine a demandé un bouton qui gèle tout, à côté de « Réinitialiser ».

## Décision

Un bouton unique dans `HeaderBar`, classe `status-undo` comme ses voisins, **placé avant `ResetButton`**.

C'est une **bascule** : tant qu'une session de l'horizon reste dégelée le bouton dit « Tout geler » et gèle tout; quand tout est gelé il dit « ❄ Tout dégeler » et dégèle tout.
Le libellé porte l'état, jamais la couleur seule (INP-3), et le `title` reprend mot pour mot la formulation de la bascule par session (« le solveur n'ajoutera ni ne déplacera plus rien […] vous pourrez toujours le modifier vous-même »), pour que les deux gestes ne racontent pas deux histoires différentes.

Tout ce qui se décide vit dans `present::freeze_all(&Plan) -> FreezeAll` (libellé, titre, étiquette d'annulation, ensemble à écrire), testé nativement; le composant n'écrit que l'`edit_plan` correspondant, donc l'acte entre dans la pile d'annulation comme les autres (ACT-2).
Les sessions sont numérotées comme le ruban les numérote — 1-based sur l'horizon *déplié*, étés inclus (`core::horizon_sessions`) — et non de 1 à `study_sessions`, qui ne compte que l'alternance A/H et laisserait chaque été dégelé.
Un gel que l'horizon n'atteint plus est conservé par l'union plutôt que silencieusement effacé.

## Alternatives rejetées

- **Deux boutons, « Tout geler » et « Tout dégeler » côte à côte** : l'un des deux est toujours sans effet, et il faut lire les deux pour savoir dans quel état on est. Surtout, un « Tout geler » seul n'a pas d'inverse sous la main — un clic de travers se défait alors session par session, ou par « Annuler » qu'il faut penser à aller chercher ailleurs. ACT-2 veut l'inverse là où l'œil est déjà.
- **Une confirmation avant de tout geler** : rejetée par principe (AIR : sous pression on clique à travers). Le geste est annulable, cela suffit.
- **Le ranger après « Réinitialiser »** : `.header-reset` porte `margin-left: 2rem`, écart qui existe pour tenir un geste destructeur à distance de ce qui le borde (ACT-5, ADR `2026-08-barre-du-haut-degarnie`). Un bouton posé après lui viendrait se coller à « Réinitialiser » du côté nu et détruirait la séparation; placé avant, il en hérite.
- **Le mettre dans le menu « Exporter ▾ » ou un autre repli** : un geste qui change l'organigramme entier reste à découvert (LAY-7).
