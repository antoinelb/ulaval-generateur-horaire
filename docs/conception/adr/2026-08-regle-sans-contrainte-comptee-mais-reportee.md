# Une règle-liste sans contrainte est comptée mais reste « reported »

**Date :** 2026-08-02
**Statut :** accepté (décision Antoine).

## Contexte

Le vérificateur de couverture rangeait toute règle sans contrainte dans `reported(...)` : `counted`, `candidates` et `raw` à `None`.
La règle « Scolarité préparatoire » (`2026-08-regle-scolarite-preparatoire`) — une liste sans contrainte — était donc invisible dans le rapport : l'UI ne pouvait ni montrer les cours d'appoint restants ni ceux déjà faits.

## Décision

- `rule_report` gagne un bras intermédiaire : une **liste résolue sans contrainte** produit `status: reported` avec `counted` (intersection avec la sélection) et `candidates` (le reste), `missing` absent — le même découpage ensembliste qu'une règle évaluée (`split_selection` partagé), mais aucun verdict : lesquels des cours listés s'appliquent dépend de faits que `core` ne voit pas (le dossier collégial pour les cours d'appoint).
- Une `Reference` sans contrainte gagne le même découpage **plus** son `raw` — strictement plus d'information, aucune fixture existante ne touchait ce cas.
- Les règles `Keyword` et `Raw` restent inchangées : rien à compter, `raw` seul.
- La référence Python (`verify_rules.py`) est à parité, l'ordre des clés calqué sur l'ordre serde (`scope, title, status, counted, missing, candidates, raw`) pour le `check` bit-à-bit.
- Les 14 fixtures `rules/` sont régénérées (leurs programmes embarqués gagnent la règle préparatoire) ; nouvelle fixture `preparatory-rule-partially-counted` figeant le comptage partiel.

## Alternatives rejetées

- **Laisser opaque et faire lire la règle dans `Program` par l'UI** : le rapport est l'API produit du jalon 8 ; contourner le rapport disperserait la logique métier hors de `core::rules`.
- **Un statut dédié (« informational »)** : `reported` dit déjà « surfacé sans verdict » ; un quatrième statut compliquerait l'UI pour la même sémantique.
- **Inventer un verdict** (satisfied quand tout est coché) : la règle n'a pas de min — un étudiant exempté de tout n'a rien à cocher et la règle n'en serait pas moins « satisfaite » ; aucun verdict n'est honnête.
