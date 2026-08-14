# US-03 — Session d'échange à l'étranger

**Persona** : Malik, en troisième année du B-GEX, profil international, parti une session à l'INSA Lyon.
**Intention** : réserver une session complète à l'étranger dans sa grille sans que l'application la déclare invalide.

Les cours suivis là-bas n'existent pas au répertoire de l'ULaval et n'ont aucun horaire publié.
Ils entrent dans la grille par les pseudo-cours de `b-gex/cours/cours-hors-catalogue.csv` : `OPT-ETR1` à `OPT-ETR4` et `EHE-1GEX`.

## Préconditions

- Programme « B-GEX », spécialisation « Profil international ».
- Le cheminement type est chargé.

## Scénario

1. Malik retire de la session H29 les cours qu'il ne suivra pas à Québec, en les glissant vers le panneau de droite.
2. Il glisse `OPT-ETR1`, `OPT-ETR2`, `OPT-ETR3` et `OPT-ETR4` dans la colonne H29.
3. Il glisse `EHE-1GEX` dans la même colonne.
4. Il ouvre « Grille horaire de session » et choisit H29.

## Résultats attendus

- Les quatre pseudo-cours occupent la session et comptent 3 crédits chacun au bilan; `EHE-1GEX` compte 0 crédit.
- Aucun conflit d'horaire n'est signalé : ces cours n'ont aucune plage horaire.
- La fenêtre de grille horaire affiche « Aucun cours avec horaire publié pour cette session. » si H29 ne contient que des cours à l'étranger.
- Les cours des sessions suivantes qui exigeaient un cours réellement remplacé à l'étranger restent signalés : l'équivalence n'est pas connue de l'application.

## Repères pour le test e2e

- `.dropped-tile[data-code="OPT-ETR1"]` existe dans la colonne dont l'en-tête vaut `H29`.
- Aucun `select.section-select` sur ces pastilles.
- Dans la fenêtre ouverte par `#btn-grille-horaire`, `.empty-msg` est visible pour la session H29.

## Variantes et cas limites

- **Comportement observé à trancher** : un cours hors catalogue n'a aucune saison enregistrée, donc `estOffert` répond faux et la pastille est marquée `cours-non-offert` avec l'erreur « ne sera pas offert à la session … ». Pour un pseudo-cours d'échange, cette alerte est un faux positif.
- Si Malik place un vrai cours de l'ULaval en H29 en même temps qu'un cours à l'étranger, seul le vrai cours peut entrer en conflit d'horaire.
- Le profil international exige `EHE-1GEX` : l'oublier doit laisser la règle correspondante en dessous de son minimum dans le bilan.
