# Un préalable invérifiable ne bloque jamais le placement : présumé satisfait et remonté

**Date :** 2026-07-30
**Statut :** accepté (décision Antoine), **restreint le 2026-07-31** par `2026-07-presomption-limitee-au-preuniversitaire` (seuls les codes 0xxx restent présumés ; les autres codes inconnus bloquent) ; étend `2026-07-operande-non-verifiable-gardee-en-texte` aux codes de cours inconnus et ferme la question « préalable vers un cours hors liste et non réussi » de `docs/next_steps.md`.

## Contexte

Un arbre de préalables peut nommer un cours que B ne peut pas vérifier : une opérande `Raw` (examen, plage de numéros), ou un **code hors de la liste fournie et non réussi** — un cours collégial, ou un électif absent d'une liste partielle (tronc seul), cas d'usage de premier rang.
La référence Python s'arrêtait en erreur dès que le verdict final en dépendait ; les fixtures évitent le cas, donc les deux politiques les reproduisent — mais l'implémentation Rust doit choisir pour les entrées réelles.

## Décision

- Évaluation **trois-valuée** de `PrereqTree` : une feuille invérifiable (opérande `Raw`, ou code ni dans la liste ni réussi) ne rejette **jamais** un placement — cohérent avec « remontée, jamais imposée ».
- Quand le verdict final d'un cours a **reposé** sur de telles feuilles, elles sont remontées dans la solution (`assumed` : l'opérande textuelle ou le code, par solution) — l'étudiant juge, `core` n'invente ni un blocage ni un silence.
- Une branche invérifiable qui ne change pas l'issue (un `any` dont une autre branche est satisfaite) n'est pas remontée : rien n'a été présumé.

## Alternatives rejetées

- **Erreur typée (la politique de la référence)** : jamais fausse, mais une liste partielle réelle qui touche le cas n'obtient aucun placement — l'API produit refuse son cas d'usage principal.
- **Bloquant (non satisfait)** : impose un verdict que les données ne portent pas ; « aucun cheminement faisable » deviendrait un faux prouvé.
