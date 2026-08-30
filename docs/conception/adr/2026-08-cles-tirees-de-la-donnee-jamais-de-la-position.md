# Toute liste rendue porte une clé tirée de la donnée, jamais de la position

Date : 2026-08-30

## Contexte

Le rapport d'Élodie du 2026-08-29 décrit des clics qui n'aboutissent pas : « le triangle a souvent nécessité 2 ou 3 clics pour se déplier, sans retour visuel entre les tentatives ratées », et une fois un clic qui a atterri ailleurs que sur sa cible.
La cause principale est la mise en page qui se déplaçait pendant le recalcul (ADR `2026-08-etat-d-attente-du-solveur-visible`).

L'audit qui l'accompagne a montré qu'une partie des listes du panneau était rendue **sans clé**, ce qui aggrave le même problème d'un cran : sans clé, Dioxus apparie les enfants par position.
Une ligne qui disparaît au milieu d'un bloc fait donc glisser l'identité de toutes les suivantes — les nœuds DOM sont réutilisés pour un *autre* contenu logique, et un clic parti avant le re-rendu peut se poser sur un élément qui n'est plus celui qui était visé.

Les listes en cause :

- `panel-verdicts` — les messages de manque de crédits, les cours bloqués du verdict ;
- `PanelBody` — les avertissements du modèle, les notes du programme ;
- la règle « Scolarité préparatoire », montée par un `if let` sans clé à côté de groupes qui, eux, en ont une.

Les entêtes de règles (`SectionView { key: "{section.key}" }`), les groupes (`key: "{group.title}"`), les options des deux menus déroulants (`key: "{title}"`), les toasts (`key: "{alert.key}"`) et les rangées du menu d'export (`key: "{entry.key}"`) étaient déjà corrects.

## Décision

Toute liste rendue par une boucle `for`, et tout nœud monté sous condition à côté de frères listés, porte une clé **tirée de la donnée** — le sigle, le titre de règle, le texte du message, la clé de section — jamais l'indice de position.

Concrètement : `key: "{message}"` sur les manques de crédits, `key: "{blocked.code}"` sur les cours bloqués, `key: "{warning}"` et `key: "{note}"` sur les avertissements et les notes, `key: "{preparatory.key}"` sur la section préparatoire.

L'invariant vaut pour toute liste ajoutée par la suite : un clic parti avant un re-rendu doit se poser sur le même élément logique, ou sur rien.

## Alternatives rejetées

- **`enumerate()` comme clé** — c'est la position déguisée en donnée : elle change dès qu'une ligne disparaît au-dessus, ce qui est exactement le cas à couvrir.
- **Ne rien changer, la mise en page étant maintenant stable** — la stabilité de la hauteur ne couvre que le recalcul du solveur ; un avertissement rejeté, une entente retirée, un cours déplacé recomposent ces mêmes listes à tout moment.
