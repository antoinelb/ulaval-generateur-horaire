# La contrainte de règle étiquetée : {type, min, max}

> **Amendé le 2026-08-30.** Le total au-dessus du `max` n'est plus une erreur typée `CountOverMax` mais le statut `RuleStatus::OverMax` de la règle seule (ADR `2026-08-depassement-de-regle-en-statut-rouge`). L'étiquetage `min`/`max` lui-même est inchangé.

Date : 2026-08-02

## Contexte

`Constraint` était un enum serde *untagged* : un compte exact `{"count": N}` ou une plage de crédits `{"min": …, "max": …}`.
La promotion des stages (ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`) demande un compte borné — « au moins 1 stage, au plus 8 » — et « Un cours parmi » est de fait min 1, max 1.
Or en untagged, un compte `{min, max}` serait indistinguable octet pour octet d'une plage de crédits.

## Décision

- `Constraint` devient étiqueté : `{"type": "course" | "credits", "min": …, "max": …}` (serde `tag = "type"`) — ce qui retrouve la grammaire de conception d'origine (`{type: course|credits, …}`, `docs/conception/`).
- « Un cours parmi » → `{"type": "course", "min": 1, "max": 1}` ; les formes crédits ne changent que d'étiquette.
  La représentation (nombre exact quand min = max, intervalle sinon) est un choix d'affichage laissé à l'UI.
- Le verdict du vérificateur pour `course` devient le miroir de celui de `credits` : total > max → erreur typée `CountOverMax` (même sémantique en attente d'arbitrage que `CreditsOverMax`, ADR `2026-07-somme-au-dessus-du-max-en-erreur-typee`) ; total ≥ min → satisfaite ; sinon incomplète avec `missing {count}`.
  `verify_rules.py` suit en parité ; comme pour les crédits, aucune fixture ne peut exprimer l'erreur (le schéma capture un rapport, pas une erreur) — le chemin vit dans les tests inline des deux implémentations.
- Migration mécanique de toutes les contraintes existantes (6 fixtures programmes, 14 fixtures rules, 7 snapshots `data/programmes/`).
  Les rapports attendus n'ont pas bougé d'un octet : le schéma du rapport de couverture n'embarque jamais la contrainte, seulement ses conséquences (`missing {count|credits}`).

## Alternatives rejetées

- **Un `max` optionnel sur `Count`** (`{"count": 1, "max": 8}`) : garde l'untagged, mais crée deux vocabulaires (`count` vs `min`) pour la même borne basse ; min/max partout est plus uniforme et l'UI décide de l'affichage.
- **Une nouvelle variante untagged `{min, max}` pour les cours** : collision exacte avec `Credits` — indécidable à la désérialisation.
- **Un statut de rapport plutôt qu'une erreur au-dessus du max** : inventerait un verdict qu'aucune décision de la direction ne couvre ; l'erreur typée suit le précédent des crédits.
