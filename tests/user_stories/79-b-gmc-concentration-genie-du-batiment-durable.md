# US-79 — B-GMC, concentration « Génie du bâtiment durable »

**Persona** : Alexis, en génie mécanique, orienté vers les bâtiments performants.
**Intention** : combler sa concentration et comprendre pourquoi son nom a changé.

## Préconditions

- Programme « B-GMC », session d'admission « A26 ».

## Ce que la concentration ajoute

- Deux cours obligatoires : `GMC-3012` et `GMC-3015`.
- Règle 1 : 9 crédits parmi 13 cours (`ARC-3103`, `GBO-2040`, `GBO-4070`, `GCI-2004`, `GCI-2007`…).
- `credits_required` vaut 18.

## Scénario

1. Alexis charge le millésime A25, où la concentration s'appelle « Génie du développement durable ».
2. Il passe au millésime A26, où elle s'appelle « Génie du bâtiment durable ».
3. Il observe ce que devient sa sélection.

## Résultats attendus

- Le changement de millésime vide la grille et repeuple le menu des spécialisations.
- La spécialisation demandée n'existant pas sous le nouveau nom, la **première** de la liste est chargée à sa place — pas la plus proche par le nom.
- Le panneau et le bilan reflètent la concentration réellement chargée, pas celle demandée.

## Repères pour le test e2e

- Après changement de millésime, `#cheminement-select` vaut la première option, quelle qu'ait été la sélection précédente.
- `#programme-subtitle` nomme la spécialisation effectivement chargée.

## Variantes et cas limites

- Le renommage vient du parseur de pages programme : l'ancien fichier A26 fait à la main disait « Génie du développement durable ». L'ADR `2026-08-conversion-des-millesimes-anciens` documente l'écart, dont la correction appartient au dépôt `generateur_horaire`.
- La concentration mêle `ARC-`, `GBO-`, `GCI-` et `GMC-` : quatre matières, donc quatre teintes, dans une seule carte.
- Un test qui vérifie la sélection après changement de millésime doit accepter la retombée sur la première option comme comportement voulu, ou demander qu'elle change.
