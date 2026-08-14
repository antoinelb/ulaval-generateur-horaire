# US-13 — Exigence linguistique, personne francophone

**Persona** : Louis, francophone, au B-GEX, qui doit réussir `ANL-2020` ou obtenir 53 au test VEPT pour diplômer.
**Intention** : savoir si l'exigence est satisfaite et, si oui, ne pas gaspiller une case de sa grille.

## Préconditions

- Programme « B-GEX » : `language_requirement.francophone` vaut `{course: "ANL-2020", tests: [{name: "VEPT", score: 53}]}`.

## Scénario

1. Louis cherche « anglais » dans la barre de recherche du panneau.
2. Il place `ANL-2020` en E27.
3. Il constate ensuite qu'il a déjà 60 au VEPT et retire le cours de la grille.

## Résultats attendus

- `ANL-2020` est trouvable par sigle et par titre dans le panneau.
- Placé, il compte dans la règle qui le contient et dans le total.
- Retiré, ses crédits disparaissent du bilan.

## Repères pour le test e2e

- `#cours-search` avec la valeur `ANL` ne laisse visible qu'un sous-ensemble de `.course-line`.
- `.dropped-tile[data-code="ANL-2020"]` apparaît puis disparaît.

## Variantes et cas limites

- **Manque fonctionnel connu** : l'exigence linguistique est un champ dédié du programme (`language_requirement`), mais l'interface ne l'affiche nulle part et ne permet pas de déclarer un score de test dispensant du cours. Un étudiant dispensé n'a aucun moyen de le dire à l'application; sa règle restera en dessous de son minimum.
- Les préalables d'`ANL-2020` contiennent de la prose non conforme à la grammaire (`Examen TOEFL - version IBT avec résultat de 57 à 63`). L'évaluation doit rester tolérante et ne pas signaler faussement le cours (US-42).
- L'exigence est une porte de diplômation, pas un préalable : elle ne doit bloquer aucun autre cours.
