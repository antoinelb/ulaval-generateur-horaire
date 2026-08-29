# Les acquis présumés sont éclatés en sigles à l'affichage

Date : 2026-08-28

## Contexte

Le bandeau « Le cheminement présume ces acquis… » affichait de faux doublons (« MAT-0130, MAT-0150, MAT-0150, MAT-0260 » — rapport étudiante-cegep du 2026-08-27).
Les éléments d'`assumed` sont un `BTreeSet` et ne peuvent pas se répéter ; le doublon venait d'un élément **multi-codes** : la grammaire des préalables ne traite pas la virgule, si bien que « MAT-0130, MAT-0150 » (de « MAT-0130, MAT-0150 ET MAT-0260 », MAT-1900) reste une seule feuille brute.
Jointe par « , » avec les autres éléments, la frontière disparaît et l'écran fabrique un doublon.

## Décision

`solve::assumed_line` éclate, avant la jointure, tout élément composé uniquement de sigles séparés par des virgules, met les sigles en commun dans un ensemble trié, et laisse tel quel tout élément qui contient autre chose que des sigles.
Le sens est préservé : ces opérandes sont des acquis présumés, les nommer un par un est fidèle.
Un sigle suivi de l'étoile de concomitance (« MAT-0260\* », B-GMC) compte comme sigle et garde son étoile à l'affichage.

## Alternatives rejetées

- Traiter la virgule dans la grammaire des préalables : la virgule n'est pas toujours un ET — « CHM-0150, CHM-0160 OU CHM-0170 » est une énumération dont le connecteur final gouverne (un parmi trois) ; 44 cours sont touchés, avec des cas de précédence ambigus (« …, … OU … ET … »).
  C'est la vraie racine, mais c'est une décision de grammaire à arbitrer séparément, avec re-scrape et régénération de fixtures.
- Joindre par « ; » pour montrer les frontières d'éléments : honnête mais illisible, et le faux doublon resterait à l'écran.
