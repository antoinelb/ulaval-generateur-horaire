# La présomption d'un code inconnu est limitée au préuniversitaire (0xxx)

**Date :** 2026-07-31
**Statut :** accepté (décision Antoine) ; restreint `2026-07-prealable-inconnu-non-bloquant-remonte` pour les feuilles de type code de cours.

## Contexte

`2026-07-prealable-inconnu-non-bloquant-remonte` présumait satisfait **tout** code hors de la liste fournie et non réussi.
Sur les données réelles, cela présumait aussi bien MAT-0130 (préuniversitaire, normalement réglé avant l'admission) que GCI-1011 — un cours universitaire obligatoire absent des snapshots (trou de données), que l'étudiant devrait pourtant réellement suivre.
Présumer un cours universitaire cache un vrai manque du cheminement.

## Décision

- Une feuille code inconnue n'est présumée satisfaite (et remontée dans `assumed`) que si son numéro commence par 0 (`MAT-0130`) : préuniversitaire, réglé avant l'admission dans le cas normal.
- Tout autre code inconnu rend la feuille **non satisfaite** : le placement est rejeté, comme si le cours manquant devait être suivi — ce qui est le cas.
- Le numéro est le seul signal de cycle disponible pour un code absent du snapshot ; un code sans `-` n'est pas préuniversitaire.
- Les opérandes `Raw` (examen, plage de numéros) restent présumées et remontées : ce ne sont pas des cours, rien ne peut les satisfaire dans le modèle.
- La référence Python n'est pas touchée : elle ne présume rien (erreur sur verdict indécidable) et aucune fixture n'exerce le cas.

## Alternatives rejetées

- **Présomption générale (l'ADR d'origine)** : un obligatoire troué comme GCI-1011 passait silencieusement en « présumé » ; le cheminement produit était faux sans que rien ne bloque.
- **Bloquer aussi les `Raw`** : une opérande textuelle (« Examen Test français ») n'est jamais satisfiable par un placement ; bloquer rendrait le cours définitivement implaçable pour un motif qui n'est pas un cours.
