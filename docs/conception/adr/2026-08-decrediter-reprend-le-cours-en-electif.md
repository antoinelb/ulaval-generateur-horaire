# Décréditer reprend le cours en électif

Date : 2026-08-19

## Contexte

Créditer un cours (« crédité par entente ») purge toute trace de placement (`state::purge_codes` : électifs, épinglages, placement affiché, sessions manuelles, sections forcées) — l'invariant « un cours acquis n'occupe aucune session » l'exige.
Mais décréditer ne faisait que retirer le code de `Plan.credited` : le cours n'était plus dans aucune source de la sélection, donc classé « manquant » (poussé en fin de liste des obligatoires) et retranché des crédits.
Aggravant : la puce « automatique » d'un obligatoire court-circuite (elle est déjà cochée), donc aucun clic ne pouvait le reprendre — seul un nouveau « Proposer » le ramenait.
Le réordonnancement de la liste a aussi révélé des listes de composants sans `key` (diff positionnel de Dioxus → handlers périmés qui recréditaient le mauvais cours), corrigées dans le même changement.

## Décisions

- **Décréditer ré-entre le cours en électif** dans la même transaction annulable (`state::uncredit_code` : `credited.remove` + `electives.push` sans doublon).
  Le cours redevient compté immédiatement (obligatoires, crédits), affiché « choisi - à placer par le solveur », et le prochain « Proposer » lui redonne une session.
- Le crédit passe par la fonction symétrique `state::credit_code` (purge + insert) — qui **ne touche pas** `rule_grants` : le crédit se cumule avec l'entente (créditer dit qu'on détient le cours, l'entente dit dans quelle règle il compte).
- **Le ✕ (retrait volontaire) purge aussi l'entente** (`state::remove_course` = purge + `rule_grants.remove`) : retirer un cours retire l'accord qui le rattachait à une règle. Les purges d'hypothèse (crédit, guérison `heal_acquired`, déplacement `place_course`) gardent l'entente.
- Le court-circuit de la puce « automatique » d'un obligatoire reste : un obligatoire hors de toute source est l'état normal d'avant le premier solve (l'intake le place d'office).

## Alternatives rejetées

- **Ne pas purger le placement au crédit** : viole « un cours acquis n'occupe aucune session » — la guérison `heal_acquired` le repurgerait de toute façon, bruyamment.
- **Mémoriser la session pré-crédit dans le `Plan`** : nouveau champ sérialisé pour une donnée périssable (l'horizon peut avoir changé entre-temps) et une migration de sauvegardes — trop pour le gain.
- **Re-solve automatique au décrédit** : un effet asynchrone dans une transaction annulable ; l'électif suffit, « Proposer » reste le geste explicite.
