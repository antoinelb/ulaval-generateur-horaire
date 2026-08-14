# US-68 — B-GCI, « Profil international »

**Persona** : Sarah-Maude, en génie civil, qui part une session à l'étranger.
**Intention** : que son échange apparaisse dans sa grille.

## Préconditions

Mêmes préconditions de déploiement qu'en US-63.

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1GCI`.
- Aucune règle, aucun `credits_required`.

## Scénario

1. Sarah-Maude choisit « Profil international ».
2. Elle glisse `EHE-1GCI` dans la session de son échange.
3. Elle complète la session avec des pseudo-cours à l'étranger.

## Résultats attendus

- Le panneau n'affiche qu'une carte, « Cours obligatoires (Profil international) ».
- Le bilan affiche une section `Profil international` sans total déclaré.

## Repères pour le test e2e

- `#cheminement-select` a « Profil international » en dernier.
- `.course-line[data-code="EHE-1GCI"]` existe dans le panneau.

## Variantes et cas limites

- **Écart connu** : `EHE-1GCI` n'est ni dans `data/cours.json` ni dans un `cours-hors-catalogue.csv`. Titre vide, `0` crédit, avertissement de console. Il faut créer `b-gci/cours/cours-hors-catalogue.csv` avec ce sigle, sur le modèle de `b-gex`.
- Le B-GCI n'a aucun pseudo-cours `OPT-ETR*` : sans fichier hors catalogue, Sarah-Maude n'a rien à placer pour les cours suivis à l'étranger (US-03).
- Le profil international et une concentration sont exclusifs dans l'interface : c'est une limite du menu à choix unique, pas une règle du programme.
