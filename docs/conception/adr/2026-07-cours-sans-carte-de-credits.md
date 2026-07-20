# Un cours sans carte « Crédits » vaut zéro crédit

Date : 2026-07-20

## Contexte

`parse_credits` cherche dans `ul.fe--faits-rapides` la carte dont le libellé commence par « Crédit », puis lit sa valeur.
GCI-2510, un séminaire de stage, n'a pas cette carte du tout : ses faits rapides ne portent que « Cycle du cours » et « Modes d'enseignement ».
L'absence était une `MissingElement`, donc une erreur franche : la page échouait en entier et le cours disparaissait du snapshot, horaire compris.

L'absence n'est pourtant pas une dérive du balisage : la page est complète et cohérente, elle dit simplement que l'activité ne porte aucun crédit.

## Décision

Aucune carte « Crédits » rend `Ok(0)` — le cours est conservé, crédité de zéro.
Zéro est la valeur juste et non un bouche-trou : un séminaire sans crédits contribue effectivement zéro au total affiché à l'étudiant.

Les deux autres chemins restent des erreurs, parce qu'eux décrivent bien un balisage inattendu :

- carte présente mais sans `span.promo-entete--titre` → `MissingElement` ;
- valeur présente mais non numérique (« trois ») → `MalformedEntry`.

GCI-2510 rejoint les fixtures gelées, mais hors du tableau `parses_every_course_fixture_without_anomalies` : son préalable est en prose (« Examen Formation obligatoire stage avec résultat de P »), donc hors grammaire, donc légitimement une anomalie.
C'est la première page gelée à parser *avec* une anomalie ; un test à part épingle les deux faits ensemble — zéro crédit et le préalable conservé en brut et signalé.

Le tableau `a_page_missing_a_field_fails_rather_than_yielding_a_partial_course` suit : sa ligne « credits » porte désormais une carte illisible plutôt qu'une carte absente, sans quoi le `?` de `parse_credits` dans `parse` ne serait plus exercé (il l'était par accident, la page tombant ensuite sur le cycle manquant).

## Alternatives rejetées

- **`credits: Option<u32>`** : distinguerait « aucune carte » de « zéro crédit », mais rien dans le domaine ne fait cette différence — un cours sans carte compte pour zéro dans un total comme dans une règle de programme — et chaque consommateur devrait dérouler l'option pour retomber sur 0.
- **Signaler une anomalie en plus de rendre 0** : le journal se remplirait d'un cas parfaitement connu et voulu, alors que la contrainte « jamais ignoré en silence » vise le texte non reconnu, pas une carte absente.
- **Rendre 0 pour tous les chemins d'échec** : une carte « Crédits » vide ou portant « trois » est une vraie dérive de balisage ; la ramener à 0 la rendrait invisible et fausserait le total de l'étudiant.
