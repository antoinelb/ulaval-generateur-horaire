# Une seule trame de partage tant que rien n'est déployé

Date : 2026-08-17

## Contexte

Le lien de partage empilait deux formats : `ShareV1` gelée, et `ShareV2` qui l'imbriquait entière pour lui ajouter `prereq_overrides` (ADR `2026-08-partage-de-lorganigramme-complet-en-fragment`, ADR `2026-08-correction-des-prealables-par-millesime`).
L'ajout d'un champ « crédité » aurait mécaniquement produit une `ShareV3` imbriquant `ShareV2`.
Or l'application n'a jamais été déployée : aucun lien n'a jamais circulé. La compatibilité protégeait des liens qui n'existent pas, au prix d'un bras de `match` par version, d'une struct imbriquée par ajout et d'un verrou de test par format.

## Décisions

- **`ShareV1` et `ShareV2` fusionnent en une struct `Share` plate**, qui porte tous les champs, `prereq_overrides` et `credited` compris. `share_into` redevient la seule traduction vers le `Plan` — plus de post-traitement hors d'elle.
- L'octet d'en-tête `version|flag` **reste**, et `SHARE_VERSION` repart à **1** : tout autre octet est refusé par `OrganigrammeShareError::UnknownVersion`, message inchangé.
- **Le gel commence au premier déploiement**, pas avant : jusque-là, mettre à jour la chaîne du test `the_frozen_string_still_encodes_byte_for_byte` *est* la migration. Après, un ajout devient une version 2 avec la sienne.

## Alternatives rejetées

- **Empiler `ShareV3`** : trois formats, trois verrous, pour des destinataires qui n'existent pas.
- **Retirer l'octet de version aussi** : il ne coûte rien et il est ce qui rendra le premier gel possible ; sans lui, un vieux lien se décoderait en charabia plutôt qu'en erreur nommée.
