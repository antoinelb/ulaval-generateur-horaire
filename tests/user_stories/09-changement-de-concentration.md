# US-09 — Changement de concentration en cours de cheminement

**Persona** : Karim, au B-GMC, qui passe du cheminement sans concentration à la concentration « Robotique » après deux ans.
**Intention** : voir ce qui lui reste à faire sous la nouvelle concentration, sans perdre ce qu'il a déjà placé.

## Préconditions

- Programme « B-GMC », millésime offrant au moins deux spécialisations.
- Une grille partiellement remplie.

## Scénario

1. Karim change « Spécialisation » de « Cheminement sans concentration » à « Robotique ».
2. Il compare le panneau de droite avant et après.
3. Il relit le bilan des crédits.

## Résultats attendus

- Le panneau de règles est reconstruit : les règles communes restent, celles de l'ancienne concentration disparaissent, celles de la nouvelle apparaissent.
- **La grille n'est pas vidée** : changer de spécialisation ne réinitialise pas les colonnes, contrairement au changement de programme ou de millésime.
- Les couleurs des pastilles ne changent pas : les teintes sont attribuées sur tous les sigles du fichier de programme, spécialisations comprises.
- Un cours placé qui n'appartient plus à aucune règle de la spécialisation courante reste dans la grille mais ne compte plus dans le bilan.
- L'en-tête `#programme-subtitle` reflète la nouvelle spécialisation.

## Repères pour le test e2e

- `#cheminement-select` est peuplé depuis les concentrations et profils du programme.
- Après changement, le nombre de `.rule-card` change mais le nombre de `.dropped-tile` reste identique.
- La couleur de fond d'une pastille donnée est inchangée avant/après.

## Variantes et cas limites

- Si la spécialisation demandée n'existe pas dans le millésime, la première de la liste est chargée à sa place.
- Un programme sans concentration ni profil laisse le menu vide et le panneau n'affiche que les activités communes.
- Le B-GMC A26 scrapé a perdu ses profils entrepreneurial et international et renomme une concentration : c'est une anomalie connue du parseur, documentée dans l'ADR `2026-08-conversion-des-millesimes-anciens`, et non un défaut de cette interface.
