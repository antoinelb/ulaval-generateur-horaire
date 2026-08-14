# US-62 — B-ANT, « Profil international »

**Persona** : Naomi, en anthropologie, qui part une session à l'étranger.
**Intention** : voir ce que le profil ajoute à son cheminement.

## Préconditions

Mêmes préconditions de déploiement qu'en US-60.

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1ANT` (échange étudiant hors établissement).
- Aucune règle, aucun `credits_required`.

## Scénario

1. Naomi choisit le « Profil international ».
2. Elle lit le panneau et le bilan.

## Résultats attendus

- Le panneau affiche une seule carte, « Cours obligatoires (Profil international) ».
- Faute de `credits_required`, l'en-tête de section du bilan n'affiche que le nom du profil, sans crédits.
- Le total exigé de la section est la somme des maximums de ses règles, soit les crédits d'`EHE-1ANT`.

## Repères pour le test e2e

- `#cheminement-select` a « Profil international » en dernière position : les concentrations viennent d'abord, les profils ensuite.
- `#log-content` contient une ligne de section `Profil international` sans mention de crédits.

## Variantes et cas limites

- **Écart connu** : `EHE-1ANT` n'existe ni dans `data/cours.json` ni dans aucun `cours-hors-catalogue.csv`. La ligne du panneau affiche un titre vide et `0` crédit, et la console journalise `Sigle introuvable dans le catalogue de cours : EHE-1ANT`. Le correctif est d'ajouter le sigle à `b-ant/cours/cours-hors-catalogue.csv`, comme le B-GEX le fait pour `EHE-1GEX`.
- Choisir le profil international **remplace** l'affichage de la concentration : l'interface ne permet pas de cumuler une concentration et un profil, alors que le répertoire le permet (US-89 pose la même question pour le B-GPH).
