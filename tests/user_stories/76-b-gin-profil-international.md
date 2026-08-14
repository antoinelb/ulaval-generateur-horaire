# US-76 — B-GIN, « Profil international »

**Persona** : Kim, en génie industriel, qui part une session à l'étranger.
**Intention** : réserver sa session d'échange dans la grille.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».

## Ce que le profil ajoute

- Un seul cours obligatoire : `EHE-1GIN`.
- Aucune règle, aucun `credits_required`.

## Scénario

1. Kim choisit « Profil international ».
2. Elle cherche `EHE-1GIN` dans le panneau.
3. Elle place `OPT-ETR1` à `OPT-ETR5` dans la session de son échange.

## Résultats attendus

- Les pseudo-cours `OPT-ETR1` à `OPT-ETR5` du B-GIN sont disponibles; `OPT-ETR5` porte le titre « Cours à option à l'étranger ou à l'Université Laval ».
- Ils comptent 3 crédits chacun, mais seulement dans les règles qui les listent.
- Le bilan affiche une section `Profil international` sans total déclaré.

## Repères pour le test e2e

- `.dropped-tile[data-code="OPT-ETR5"][data-credits="3"]` existe.
- `.course-line[data-code="EHE-1GIN"]` existe dans le panneau.

## Variantes et cas limites

- **Écart connu** : `EHE-1GIN` est absent de `data/cours.json` **et** de `b-gin/cours/cours-hors-catalogue.csv`. Le panneau affiche un titre vide et `0` crédit, et la console journalise `Sigle introuvable dans le catalogue de cours : EHE-1GIN`. Le B-GEX déclare son `EHE-1GEX` : il suffit d'ajouter la ligne équivalente au fichier du B-GIN.
- Le B-GIN a des pseudo-cours particuliers à tester : `LAN-GUES` (cours de langue selon le résultat VEPT), `SAN-SÉCU` (dont le sigle contient un accent, contrairement au motif `[A-Z]{2,4}-\d{4}`) et `SCI-NATU`.
- `SAN-SÉCU` et les autres pseudo-sigles non conformes ne sont jamais reconnus par l'analyse des préalables : ils ne peuvent ni satisfaire ni bloquer un cours (US-52).
