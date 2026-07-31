# Une somme au-dessus du `max` d'une règle à crédits est une erreur typée

**Date :** 2026-07-30
**Statut :** accepté (décision Antoine) ; ferme « somme au-dessus du `max` d'une règle à crédits » de `docs/next_steps.md` pour le vérificateur, en attendant l'arbitrage du directeur.

## Contexte

« De 3 à 9 crédits parmi : » — que vaut une sélection qui en compte 12 ?
Violation, ou excédent qui ne compte simplement pas vers la règle ?
Aucun document ne le dit ; la référence Python s'arrêtait en erreur, les fixtures restent ≤ `max`.

## Décision

- Le vérificateur retourne une **erreur typée** (`CoverageError::CreditsOverMax` — règle, somme, max) plutôt qu'un statut : aucun verdict n'est inventé tant que le directeur n'a pas tranché.
- L'UI qui rencontre l'erreur affiche le dépassement tel quel ; c'est un état bloquant assumé, pas un oubli.
- À l'arbitrage, l'erreur sera remplacée par la sémantique décidée (statut dédié ou excédent non compté) — le type force ce point de passage.

## Alternatives rejetées

- **`satisfied` + excédent remonté** : décide implicitement que « le min couvert suffit » — c'est l'arbitrage lui-même, pris sans le directeur.
- **Nouveau statut `exceeded`** : étend le contrat gelé des fixtures sur une sémantique non documentée ; l'erreur typée est réversible, le statut publié ne l'est plus.
