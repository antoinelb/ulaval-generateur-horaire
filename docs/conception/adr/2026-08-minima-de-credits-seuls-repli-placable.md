# Seuls les minima de crédits peuvent être placés en faute diagnostiquée

Date : 2026-08-23

## Contexte

ADR `2026-08-seuil-de-credits-elague-au-domaine` interdit tout placement qui viole un seuil `program_credits`.
Certains cheminements réels ne peuvent pourtant être proposés sous leur horizon et leur plafond publiés, alors que décaler le cours au plus tard fournit une correction utile.

## Décision

Cette décision révise cette interdiction uniquement pour les passes internes de repli et de diagnostic.
Le mode strict reste le défaut de `place`, `admissible_sessions` et de la vérification.
Le mode souple ne relâche que les feuilles `ProgramCredits`, place le cours concerné le plus tard possible et sérialise chaque `CreditShortfall`.
Les préalables par sigle, saisons, horaires, plafonds, stages et épinglages restent durs.
La génération essaie le seuil souple avant d'ouvrir les étés, puis peut le combiner au repli `left_out` en dernier recours.
La vérification stricte reste invalide et une seconde passe épinglée ne fournit que le diagnostic persistant.

## Alternatives rejetées

- **Augmenter automatiquement le plafond** — cela modifierait une contrainte distincte sans nommer le cours responsable.
- **Afficher un toast** — l'écart critique disparaîtrait alors que le placement resterait inchangé.
- **Assouplir les préalables par sigle** — le solveur inventerait un ordre académiquement faux.
