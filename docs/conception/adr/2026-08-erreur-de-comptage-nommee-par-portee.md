# L'erreur de dépassement nomme la portée de la règle

Date : 2026-08-23

Amende `2026-07-somme-au-dessus-du-max-en-erreur-typee`.

## Contexte

`CoverageError::CreditsOverMax` et `CountOverMax` ne portaient que le titre de la règle en défaut (« Règle 1 »).
Or une concentration et un profil peuvent chacun déclarer leur propre « Règle 1 » : la bannière d'erreur du panneau ne disait pas laquelle avait dépassé son maximum, et l'étudiante ne savait pas où retirer un cours.

## Décision

Les deux variantes portent désormais un champ `scope: Scope` en plus de `rule` et `total`/`max`; le `Display` anglais l'inclut entre parenthèses (`"{rule} ({scope} scope) : …"`).
Côté panneau, `coverage_error_message` traduit la portée avec `scope_origin`, la même fonction déjà partagée avec l'en-tête d'une règle (`rule_lead`) — la bannière dit « Règle 2 de la concentration » ou « Règle 1 du profil », exactement la formulation que porte le `lead` d'une règle de cette portée, pour que le vocabulaire ne diverge jamais entre les deux messages.

## Alternatives rejetées

- **Ne garder que le titre de la règle** — deux règles homonymes dans deux portées différentes (une « Règle 1 » de concentration, une « Règle 1 » de profil) rendaient la bannière ambiguë sur laquelle des deux avait débordé.
