# US-89 — B-GPH, « Profil international »

**Persona** : Adam, en génie physique, qui part une session à l'étranger.
**Intention** : réserver sa session d'échange.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1GPH`.
- Aucune règle, aucun `credits_required`.

## Scénario

1. Adam choisit « Profil international », la dernière des neuf options.
2. Il cherche `EHE-1GPH` dans le panneau.
3. Il cherche de quoi représenter les cours suivis à l'étranger.

## Résultats attendus

- Le panneau n'affiche qu'une carte, « Cours obligatoires (Profil international) ».
- Le bilan affiche une section `Profil international` sans total déclaré.
- Choisir ce profil masque la concentration : Adam ne voit plus les 15 crédits de concentration qu'il doit faire aussi.

## Repères pour le test e2e

- `#cheminement-select option` compte neuf entrées, « Profil international » en dernier.
- Une seule `.rule-card` est affichée.

## Variantes et cas limites

- **Écart connu** : `EHE-1GPH` est absent du catalogue **et** de `b-gph/cours/cours-hors-catalogue.csv`, qui ne déclare que `LAN-GUES`. Titre vide, `0` crédit, avertissement de console.
- Le B-GPH n'a aucun pseudo-cours `OPT-ETR*` : Adam n'a rien à placer pour ses cours à l'étranger, contrairement au B-GEX, au B-GIN et au B-GMC.
- Le cas cumulé concentration + profil international n'est représentable dans aucun programme : c'est la limite du menu à choix unique, à trancher si des étudiants font les deux.
