# Les sessions déjà complétées sont fermées au solveur

## Contexte

Un import de relevé Capsule ancre le plan dans le passé : les premières sessions de l'horizon sont déjà vécues et notées.
Le solveur, lui, traitait toutes les sessions également et pouvait proposer un nouveau cours en A24 — une session terminée où personne ne s'inscrira jamais.

## Décision

`PlacementRequest` gagne `completed_sessions: usize` : les sessions `1..=n` n'acceptent aucun cours **non épinglé** — le mécanisme calque `open_summers`, un filtre de domaine dans `value_ordered_domain`.
Un épinglage (celui du relevé, ou un geste explicite de l'étudiant — ajout manuel compris) reste souverain : il occupe une session complétée de plein droit.
La valeur vient du relevé : `apply_transcript` calcule l'indice de la dernière session *notée* (section « CRÉDITS DE L'UNIVERSITÉ LAVAL ») ; les sessions « CRÉDITS EN COURS » restent ouvertes, l'étudiant peut encore les ajuster.
Elle voyage `TranscriptApplication.completed_sessions` → `Plan.completed_sessions` (0 par défaut, `serde(default)`) → requête du solveur → `OrganigrammeInput` (`serde(default)`, les anciens appels JS restent valides).
Les jetons « + H28 » (`admissible_sessions`) passent par le même domaine et n'offrent donc plus de session complétée pour un cours non épinglé — cohérent : ce sont des placements proposés par le solveur.

## Alternatives rejetées

- **Dériver « complétée » de l'horloge du navigateur** : le code pur n'invente jamais le temps (discipline du dépôt), et la date ne dit pas si la session est notée ; le relevé le dit.
- **Fermer aussi les sessions en cours** : l'inscription de la session courante peut encore bouger ; le relevé distingue déjà noté / en cours, on suit sa distinction.
- **Porter la fermeture dans l'URL de partage** : hors périmètre — un organigramme partagé se rouvre entièrement éditable ; à ajouter à l'état de partage si cela induit en erreur (commentaire `ponytail:` posé dans `persist.rs`).
