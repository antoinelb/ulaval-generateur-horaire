# US-54 — Reprise après rechargement de la page

**Persona** : William, qui ferme son portable en plein milieu de sa planification.
**Intention** : retrouver sa grille en rouvrant l'application.

## Préconditions

- Une grille remplie, un programme et une spécialisation choisis.

## Scénario

1. William recharge la page.

## Résultats attendus — comportement actuel

- **Tout est perdu** : la grille repart vide, sur le premier programme de la liste, au millésime le plus récent.
- Aucune donnée n'est conservée entre deux visites; la seule persistance est la sauvegarde CSV manuelle (US-26).

## Résultats attendus — à venir

Le plan du dépôt `generateur_horaire` prévoit une reprise côté client :

- L'état courant (programme, millésime, spécialisation, grille, sections choisies, filtres) est conservé dans `localStorage` et restauré au chargement.
- Un cheminement est partageable par URL, un horaire n'étant qu'un ensemble de sections encodable dans l'adresse.

## Repères pour le test e2e

- Aujourd'hui : après `page.reload()`, `.dropped-tile` est absent — c'est le test de non-régression du comportement actuel, à inverser le jour où la persistance arrive.
- Demain : après `page.reload()`, la grille est identique, et `localStorage` contient l'état.

## Variantes et cas limites

- Deux onglets ouverts sur la même application ne partagent rien aujourd'hui; avec `localStorage`, il faudra décider lequel gagne.
- Un état persisté qui référence un millésime disparu du dépôt doit se dégrader proprement, jamais planter.
- Une URL partagée doit rester lisible par quelqu'un qui n'a jamais ouvert l'application.
