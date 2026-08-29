# Dans les préalables, la virgule prend le sens du connecteur final

Date : 2026-08-29

## Contexte

La grammaire des préalables ne traitait pas la virgule.
« MAT-0130, MAT-0150 ET MAT-0260 » (MAT-1900) parsait en `all[{raw: "MAT-0130, MAT-0150"}, "MAT-0260"]` : l'énumération restait une feuille brute portant deux codes à la fois.

Deux conséquences.
D'abord des faux doublons à l'écran : le bandeau des acquis présumés joignait ses éléments par « , », la frontière entre une feuille multi-codes et sa voisine disparaissait, et « MAT-0130, MAT-0150, MAT-0150, MAT-0260 » s'affichait (rapport étudiante-cegep du 2026-08-27, ADR `2026-08-acquis-presumes-eclates-a-laffichage`).
Ensuite une perte de vérifiabilité : une feuille brute est *présumée satisfaite*, donc chaque cours de l'énumération sortait du champ du solveur.

L'ADR d'affichage écartait la correction de grammaire faute d'arbitrage : la virgule n'est pas toujours un ET, « CHM-0150, CHM-0160 OU CHM-0170 » (CHM-1003) étant un choix parmi trois.
Antoine a tranché le 2026-08-29, sur ses deux exemples :

- « CHM-0150, CHM-0160 OU CHM-0170 » = CHM-0150 OU CHM-0160 OU CHM-0170 ;
- « MAT-0130, MAT-0150 ET MAT-0260 » = les trois.

## Décision

Une énumération de sigles séparés par des virgules est régie par **le connecteur de son dernier séparateur**, et vaut exactement le groupe parenthésé correspondant.
Elle est donc *un seul opérande* de la précédence ET/OU environnante : « CHM-0150, CHM-0160 OU CHM-0170 ET PHY-0150 » (CHM-1901) donne `all[any[trois], PHY-0150]`, et non un OU dont le dernier terme serait un ET.

C'est littéralement ainsi que le parseur l'implémente : avant qu'aucun opérande ne soit classé, `group_enumerated_sigles` réécrit la suite de mots « MAT-0130, MAT-0150 ET MAT-0260 » en « ( MAT-0130 ET MAT-0150 ET MAT-0260 ) ».
La précédence, le repli en texte brut et les feuilles étoilées restent ceux d'avant — une étoile posée sur un élément d'énumération (« MAT-0260\* », GMC-1001) produit toujours sa feuille `{"concomitant": …}`.

Trois formes restent volontairement brutes, faute d'un sens que la grammaire puisse prouver :

- l'énumération que **ne ferme aucun connecteur** — « BIO-0150, CHM-0150, CHM-0160 » (BCM-1903) ne dit ni ET ni OU, et en choisir un inventerait une exigence que le répertoire n'a pas écrite ;
- la virgule prise dans de la **prose** — « Réussir 2 parmi CTB-6112, CTB-6116, … » (CTB-6113, 13 cours) : le motif « N parmi » n'a pas de gabarit dans la grammaire et n'est pas l'objet de la décision ;
- toute énumération dont un élément n'est pas un sigle (« MAT-0130, Examen de langue »).

L'ADR `2026-08-acquis-presumes-eclates-a-laffichage` reste en vigueur : elle défend l'affichage pour la prose multi-codes qui subsiste.

## Mise à jour des données

`data/cours.json` n'est pas re-scrapé : le `raw` de chaque préalable y est stocké intégralement, et il porte toute l'information.
Une sous-commande `ulaval-scraper reparse` relit le fichier, re-dérive chaque `tree` de son propre `raw` par `parse_prereq_tree`, réécrit atomiquement (tmp + `rename`) et rapporte le nombre d'arbres changés — zéro requête, zéro dérive, et elle resservira à chaque évolution de grammaire (le précédent est l'ADR `2026-08-etoile-de-concomitance-au-parsing`, qui a fait la même re-dérivation à la main).
`meta.json` n'est pas touché : il date le *scrape*, qui n'a pas eu lieu.

Sur les 44 cours dont un opérande brut portait une virgule entre deux sigles, 27 changent d'arbre : 13 sont des « Réussir N parmi », 4 des énumérations sans connecteur final, et le reste passe en `all` (21) ou en `any` (6, dont BCM-1001 et STT-1000).
Toutes les feuilles ainsi rendues vérifiables sont des cours préuniversitaires (`0xxx`) : aucun code universitaire n'était pris dans une énumération à virgules, donc la re-dérivation ne rend aucun préalable soudainement contraignant pour le solveur.
Elle rend en revanche ces `0xxx` nommables un par un — le bandeau des acquis présumés les liste déjà individuellement — et un relevé qui les a réussis les satisfait maintenant feuille par feuille.

## Alternatives rejetées

- **Laisser la virgule brute et ne défendre que l'affichage** (le statu quo) : les faux doublons disparaissaient de l'écran, mais l'énumération restait une feuille présumée, hors de portée du solveur comme des messages de refus.
- **Lire toute virgule comme un ET** : faux sur les 6 énumérations que ferme un OU, où l'étudiant se verrait exiger quatre cours de chimie et de biologie au lieu d'un (BCM-1001).
- **Fusionner l'énumération à plat dans la chaîne environnante** au lieu d'en faire un groupe : CHM-1901 donnerait `any[CHM-0150, CHM-0160, all[CHM-0170, PHY-0150]]`, c'est-à-dire PHY-0150 exigé seulement dans une branche sur trois.
- **Trancher l'énumération sans connecteur final par défaut** (ET, comme la lecture la plus courante) : c'est une exigence inventée ; le texte brut la laisse à l'étudiant, qui voit la phrase.
- **Re-scraper le catalogue** pour régénérer les données : ~10 000 requêtes pour une information que le `raw` déjà stocké porte entièrement.
