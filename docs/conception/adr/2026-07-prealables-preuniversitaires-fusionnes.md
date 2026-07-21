# Les préalables préuniversitaires sont fusionnés aux préalables

Date : 2026-07-21

## Contexte

La page de certains cours (`GML-1001`) porte, dans une boîte `.fe--message` répétée à chaque session, la phrase **« Préalables préuniversitaires nécessaires s'il y a lieu : CHM-0150 et PHY-0250. »**.
Le sélecteur des préalables (`div.fe--prealables p.etiquette-container`) ne la lit pas : ces préalables disparaissaient en silence, ce qu'interdit la contrainte « ne jamais perdre une entrée non reconnue ».

## Décision

Les sigles du champ préuniversitaire sont **fusionnés à l'arbre des préalables** comme des opérandes `ET`, en une seule expression (choix de l'utilisateur, comme le fait le dépôt de référence `pcardou/grille-de-cheminement-interactive`).

Le champ est lu **nœud de texte par nœud de texte** — le marqueur et ses sigles partagent un nœud, une prose voisine en occupe un autre — puis le `raw` combiné est reconstruit et confié au parseur de préalables existant, pour un arbre plié comme celui de tout autre cours :

- champ seul : `raw = "CHM-0150 ET PHY-0250"` ;
- préalable régulier *et* préuniversitaire : `raw = "(<régulier>) ET (<préuniv>)"` ;
- aucun sigle extrait alors que le marqueur est présent : une anomalie `MalformedEntry`, jamais un abandon silencieux.

Un cours sans champ préuniversitaire garde son `raw` et son arbre au bit près.

## Conséquences

`GML-1001`, sans préalable régulier, obtient `{ raw: "CHM-0150 ET PHY-0250", tree: { all: ["CHM-0150", "PHY-0250"] } }`.
Les sigles pointent vers des cours d'appoint désormais présents au catalogue (ADR `2026-07-cours-dappoint-reintegres`).

## Alternatives rejetées

- **Un champ séparé `preuniversitaire_prerequisites`** : préserverait la nuance « s'il y a lieu » (conditionnel selon le dossier collégial de l'étudiant), mais l'utilisateur a tranché pour une expression de préalables unique.
- **Lire la boîte `.fe--message` d'un seul tenant** (`text().collect::<String>()`) : collerait « PHY-0250.Cette » au nœud de prose suivant et perdrait le second sigle.
