# US-69 — B-GEX, « Profil international »

**Persona** : Youssef, au baccalauréat en génie des eaux, qui fait sa quatrième année à l'étranger.
**Intention** : que son échange soit reconnu dans sa grille.

C'est le seul programme dont le profil international est **entièrement utilisable aujourd'hui** : `EHE-1GEX` est déclaré dans `b-gex/cours/cours-hors-catalogue.csv`, avec `OPT-ETR1` à `OPT-ETR4` pour les cours suivis là-bas.

## Préconditions

- Programme « B-GEX », session d'admission « A26 ».

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1GEX`, déclaré à 0 crédit dans le fichier hors catalogue.
- Aucune règle, aucun `credits_required`.
- Le B-GEX n'a **aucune** concentration : le menu « Spécialisation » ne contient que ce profil.

## Scénario

1. Youssef choisit « Profil international ».
2. Il glisse `EHE-1GEX` dans la session de son échange, avec `OPT-ETR1` à `OPT-ETR4`.
3. Il lit le bilan.

## Résultats attendus

- La pastille `EHE-1GEX` porte son titre « Échange étudiant hors établissement » et 0 crédit.
- Les quatre `OPT-ETR*` comptent 3 crédits chacun, mais seulement dans les règles qui les listent.
- La section `Profil international` du bilan n'affiche pas de total déclaré.

## Repères pour le test e2e

- `#cheminement-select` contient exactement une option.
- `.dropped-tile[data-code="EHE-1GEX"][data-credits="0"]` existe.

## Variantes et cas limites

- Un menu « Spécialisation » à une seule option ne peut pas être changé : le scénario de US-09 (changement de concentration) n'est pas jouable sur le B-GEX.
- `EHE-1GEX` et les `OPT-ETR*` sont marqués « non offerts » dans toute session, faute de saison enregistrée — le faux positif de US-03.
- Ce programme est le modèle à reproduire pour les autres : sigle d'échange déclaré hors catalogue, plus des pseudo-cours d'options à l'étranger.
