# `prerequisites_met` et `admissible_sessions` : deux coutures UI exposées par `core`

## Contexte

L'interface (design 5a/9a) a deux besoins que la vue ne peut pas calculer sans logique métier (invariant « aucune règle métier dans la vue ») :

- « préalables non remplis » (jalon 6) : estomper un cours dont les préalables ne sont pas satisfaits par les acquis de l'étudiant — l'évaluation du `PrereqTree` vivait entièrement en privé dans `organigramme.rs` ;
- les puces « + H28 » (onglet Bac complet) : quelles sessions de l'horizon peuvent accueillir un cours, offre + préalables + plafond + veto hebdomadaire compris.

## Décision

Deux fonctions publiques dans `core::organigramme`, décidées avec Antoine (2026-08-13) :

- `prerequisites_met(course, satisfied, credits) -> Result<PrereqStatus, PlacementError>` — la question *statique* : contre ce que l'étudiant détient (codes acquis + crédits accumulés), jamais contre un placement futur. Mêmes sémantiques de feuilles que la recherche : code inconnu non préuniversitaire bloquant (ADR `2026-07-presomption-limitee-au-preuniversitaire`), opérande brut ou préuniversitaire présumé **et remonté** dans `assumed`. Réutilise `flat_tree` et les combinateurs `all_verdict`/`any_verdict` — aucune règle dupliquée.
- `admissible_sessions(request, code) -> Result<BTreeSet<usize>, PlacementError>` — une **sonde de `place` par session**, le cours épinglé là, le reste de la requête tel quel. Un épinglage est exactement ce que le clic sur la puce ferait : la sémantique du pin (intersection avec l'offre, restriction d'été levée) répond à la question que le clic pose. Tout épinglé, chaque sonde valide une seule affectation — coût linéaire, pas de recherche.

## Alternatives rejetées

- **Évaluer le `PrereqTree` dans la vue** : logique métier dans la vue, divergence garantie avec le solveur.
- **Exposer la machinerie interne (`SearchCtx`/`eval`)** : surface d'API énorme pour deux questions ; la représentation en préfixe du chemin de recherche ne modélise pas « tout placé sauf un ».
- **Puces calculées côté UI par saisons offertes seulement** : ignore préalables, plafond et veto hebdomadaire — la puce mentirait, et le mensonge se découvrirait au clic.
