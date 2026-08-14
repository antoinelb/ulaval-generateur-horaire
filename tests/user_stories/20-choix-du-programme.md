# US-20 — Choisir le programme

**Persona** : Nicolas, qui hésite entre le génie des eaux et le génie industriel.
**Intention** : comparer les deux programmes dans l'application.

## Préconditions

- `index-programmes.csv` liste `b-gex`, `b-gin`, `b-gmc` et `b-gph`.

## Scénario

1. Nicolas ouvre l'application : le premier programme de la liste est chargé.
2. Il passe à « B-GIN ».
3. Il revient à « B-GEX ».

## Résultats attendus

- Le menu affiche les acronymes en majuscules; leur valeur est l'acronyme minuscule, qui est aussi le nom du dossier des fichiers manuels.
- Chaque changement recharge, dans l'ordre : le catalogue, l'index des millésimes, le menu des sessions d'admission, le menu des spécialisations, le panneau de règles, puis relance la vérification.
- Le sous-titre du panneau affiche le nom complet du programme, tiré de `index-programmes.csv`, et non le titre du fichier de programme.
- La grille est vidée à chaque changement.

## Repères pour le test e2e

- `#programme-select option` contient quatre entrées, valeurs `b-gex`, `b-gin`, `b-gmc`, `b-gph`.
- `#programme-subtitle` a la forme `<nom> — <millésime> — <spécialisation>`.
- Aucune `.dropped-tile` après un changement.

## Variantes et cas limites

- Si `index-programmes.csv` est absent, le menu reste vide et l'application retombe sur le programme par défaut `b-gmc`.
- Un programme sans fichier dans `data/programmes/index.json` donne un menu de sessions d'admission vide et un panneau sans règles.
- Le B-GPH n'a qu'un millésime : le cas d'un menu à une seule option doit fonctionner.
