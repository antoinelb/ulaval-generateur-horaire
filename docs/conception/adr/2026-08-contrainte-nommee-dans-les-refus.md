# Les refus du solveur nomment la contrainte exacte

## Contexte

Un cours refusé à sa session épinglée recevait « (plafond, horaire ou préalables) — dépinglez-le ou montez le plafond de crédits » : trois causes possibles, aucune nommée, rien d'actionnable (retour d'Antoine, 2026-08-26).
Le pré-écran du solveur savait pourtant *pourquoi* un arbre de préalables est insatisfiable, mais jetait la preuve.

## Décision

`Blocked` gagne `missing: Vec<Vec<String>>` : chaque entrée est une exigence qu'aucun agencement ne peut satisfaire, ses alternatives interchangeables ensemble (`[["ECN-2901","ECN-4901"]]` se lit « ECN-2901 ou ECN-4901 »).
Le blâme est extrait par une marche bornée sur les feuilles fausses de l'arbre (`blame_false_leaves`), partagée entre le pré-écran optimiste et la nouvelle fonction statique `unmet_prerequisites`.
Vide quand la cause n'a pas de cours à nommer (seuil de crédits) — et absent du JSON dans ce cas, pour ne rien changer aux fixtures.
Côté UI, `blocked_note` rend la preuve (« il faudrait GCI-1011, ni acquis ni prévu au cheminement »), et le toast d'épinglage refusé diagnostique localement les trois causes vérifiables — saison non offerte, plafond avec les chiffres, préalables manquants par sigle via `unmet_prerequisites` — « l'horaire » restant l'explication résiduelle honnête quand aucune ne se vérifie.

## Alternatives rejetées

- **Nommer la famille seulement** (préalables / plafond / saison sans le sigle) : l'étudiant sait quoi lire mais toujours pas quoi corriger.
- **Un blâme exact par branche dans les `any` imbriqués** : les alternatives s'aplatissent aux feuilles du sous-arbre — exact pour les grammaires réelles (profondeur ≤ 2), et le raffinement attendra qu'un cas le réclame (`ponytail` en commentaire).
