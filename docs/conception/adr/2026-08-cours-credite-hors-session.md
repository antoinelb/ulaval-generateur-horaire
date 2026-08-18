# Cours crédité par entente : compté, jamais placé

Date : 2026-08-17

## Contexte

Une entente avec la direction peut **créditer** un cours : l'étudiant ne le suit pas, mais le cours compte dans ses crédits et dans ses acquis.
L'interface ne savait dire que *dans quelle règle* un cours compte (`Plan.rule_grants`, ADR `2026-08-entente-cours-regle-et-scolarite-preparatoire`), jamais qu'il est acquis d'avance.
Sans marqueur, le seul moyen de ne pas lui donner de session était de l'omettre — et alors ses crédits disparaissaient du compteur du bac et sa règle restait incomplète.

La sémantique visée existe déjà dans `core` sous le nom `passed`, que l'ADR `2026-08-retrait-de-la-notion-de-cours-reussi` avait conservée exactement pour ça :
« c'est le seul mécanisme qui retire du placement des cours acquis sans leur faire occuper une session ni des crédits de l'horizon ».
Son unique alimentateur était `Plan.preparatory_done` ; « crédité » devient le second.

## Décisions

- **`Plan.credited: BTreeSet<String>`**, et `solve::passed_codes` réunit les deux familles (préparatoire cochée, crédités) pour les deux constructeurs de requête — `request_json` et `unplaced_codes`. `core` ne change pas d'une ligne : `passed` exclut déjà des candidats, satisfait les préalables et cumule ses crédits.
- **Le comptage passe par la sélection** : `panel::selection` chaîne `plan.credited`, ce qui met les crédités d'un seul geste dans `coverage_report` *et* dans `credit_summary` (« X/120 cr au bac »). Un crédité préuniversitaire ou « en sus » reste ventilé à part par `credit_summary`, comme n'importe quel autre cours — le crédit ne ment pas sur le diplôme.
- **La bascule est séparée du select d'entente, et cumulable** : créditer dit que le cours est acquis, le select dit dans quelle règle il compte. Un cours manuel fait ailleurs peut donc être crédité *et* rattaché à la Règle 3.
- **Un crédité apparaît dans la règle où il compte** (Obligatoires, Règle 2, …) — pas de section « Crédités » dédiée. Celui qu'aucune section ne liste (règle « tous les cours », cours hors programme sans entente) est **nommé** dans les avertissements du panneau : jamais de crédits venus de nulle part.
- **Même invariant que la case préparatoire** : un crédité n'occupe aucune structure de placement. `state::purge_codes` s'applique dans la *même* transaction annulable que la bascule, `solve::acquired_leftovers` (ex-`preparatory_leftovers`) surveille les deux familles pour l'effet de guérison `heal_acquired`, et `validate_new_code` refuse un code crédité en nommant l'issue. La rangée `RowState::Credited` n'offre ni « + » ni puces — le select d'entente, lui, reste.

## Alternatives rejetées

- **« Crédité » comme option du select d'entente** : un seul contrôle, mais alors exclusif du rattachement — un cours crédité hors programme ne pourrait plus être compté dans une règle.
- **Une section « Crédités » dans le panneau** : une liste de plus à lire alors que la règle concernée est justement l'endroit où l'étudiant cherche le cours.
- **Ressusciter `Plan.passed` général** : l'ADR `2026-08-retrait-de-la-notion-de-cours-reussi` l'a retiré parce qu'il doublonnait un placement en session passée. Un crédité ne doublonne rien : il n'a *pas* de session, c'est tout son propos.
