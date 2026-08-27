# Déplacer des cours obligatoires conserve le cheminement type

Date : 2026-08-26

## Contexte

L'export passait du mode « cheminement type » au mode personnel dès qu'un cours était épinglé ou déplacé.
Le geste `place_course` ajoute aussi le cours déplacé à `Plan.electives`, même lorsque ce cours est obligatoire, afin que le solveur conserve le cours épinglé.
Une direction de programme ne pouvait donc pas réorganiser les obligatoires pour produire un autre cheminement type.

## Décision

Un export reste un cheminement type tant que tous les cours explicitement retenus appartiennent aux cours obligatoires du programme, de la concentration choisie ou du profil choisi, ou correspondent au premier cours de la règle obligatoire `Stages`.
Le premier stage est le parcours attendu; les stages suivants restent des choix additionnels et rendent donc l'organigramme personnel lorsqu'ils sont explicitement retenus.
Les codes explicites sont lus dans `electives`, `pinned_sessions`, `manual`, `credited` et `rule_grants` afin que les anciennes représentations persistées et tous les chemins d'édition suivent la même règle.
Le placement automatique seul ne constitue toujours pas un choix explicite.
Dès qu'un de ces codes n'est pas obligatoire dans la portée effective, l'export devient un organigramme personnel.
Sans programme disponible pour prouver qu'un code est obligatoire, tout code explicitement retenu rend l'export personnel.

## Alternatives rejetées

- **Toute épingle rend le plan personnel** : cela empêche la direction de programme de construire un cheminement type par déplacement.
- **Tester seulement si `electives` est vide** : `place_course` y ajoute aussi les obligatoires déplacés, ce qui reproduirait le bogue.
- **Ignorer toutes les épingles** : une épingle portant sur un cours non obligatoire est bien le choix d'un cours supplémentaire et doit produire un organigramme personnel.
