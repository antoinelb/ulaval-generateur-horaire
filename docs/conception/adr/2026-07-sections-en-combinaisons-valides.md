# Une saison porte des combinaisons d'inscription valides, pas des groupes de choix

Date : 2026-07-20

Remplace les points 2 et 5 de `2026-07-sections-en-groupes-de-choix`.
Les points 1, 3, 4 et 6 de cet ADR restent en vigueur.

## Contexte

`2026-07-sections-en-groupes-de-choix` §5 posait une hypothèse **explicitement falsifiable** :

> L'hypothèse « les sections liées ne dépendent pas de la section de premier niveau choisie » est assumée et vérifiable […] Le passage sur le catalogue complet (~10 000 cours) tranchera empiriquement ; aucune fixture connue n'exhibe ce cas.

Le passage a eu lieu.
`data/cours_errors.log` porte 33 enregistrements « linked sections under N top-level sections », sur 22 cours.
**L'hypothèse est fausse.**

Les 22 pages ont été récupérées et leur DOM cartographié, en nombre de sections liées par section de premier niveau :

```
arc-6501  hiver   : [23,23,23,23,23,23,23,23]
pse-3501  hiver   : [1,1,1,1,1,1,1,1,1,1,1]
ift-1004  automne : [2, 0]
cso/eti/gie/med/sin/svs : [1,1] ou [1,1,1]   (forme dominante)
```

IFT-1004, automne 2026, est le cas le plus net :

| | NRC | section | mode | liées |
|---|---|---|---|---|
| niveau 1 | 85469 | *(vide)* | En classe | 85470 (A), 85471 (B) |
| niveau 1 | 85472 | Z3 | À distance | — |

Le modèle plat produit `groups = [[85469, 85472], [85470, 85471]]`, lu « un par groupe ».
Son produit cartésien admet **85472 + 85470** — la section à distance appariée au laboratoire de la section en classe — et ne sait pas exprimer « 85472 seul ».
PSE-3501 est pire : 11 × 11 = 121 combinaisons, dont 11 valides.

Une seconde affirmation du même ADR tombe avec.
Son §Contexte déclarait qu'« un NRC dupliqué dans deux groupes de choix est structurellement impossible, le NRC *étant* l'identité de l'inscription ».
Or CSO-6702 accroche ses deux sections A et B au **même** NRC lié 13449, et PSE-3501 accroche H1, H2 et H3 au même 17896.
Le modèle plat met donc le même NRC deux fois *dans un seul groupe*.
La section liée n'est pas un choix indépendant : c'est une **composante commune partagée** — un séminaire, un cours magistral commun — que la section de premier niveau entraîne avec elle.

Vérification faite sur les 22 pages : l'imbrication ne dépasse **jamais** deux niveaux.

## Décision

`SeasonOffering { groups: Vec<Vec<Section>> }` devient `SeasonOffering { options: Vec<Vec<Section>> }`, et l'invariant s'inverse :

- **avant** — un `Vec<Section>` est un groupe de choix; on retient une section *par groupe*, et l'horaire est l'union des sections retenues;
- **après** — un `Vec<Section>` est une inscription **complète**; on retient une `options[i]` **en entier**, et l'horaire est l'union des plages de ses sections.

Le parseur développe les combinaisons au moment de lire la page :

```
pour chaque section de premier niveau T :
    si T n'a aucune section liée  → une option [T]
    sinon                         → une option [T, L] par section liée L
```

IFT-1004 automne devient `[[85469, 85470], [85469, 85471], [85472]]`.
ARC-6501 hiver devient 8 × 23 = 184 options.

Le champ **doit** être renommé : garder `groups` avec la même forme JSON et un sens inversé serait un piège pour tout consommateur écrit contre l'ancienne lecture.

Un NRC peut désormais réapparaître **entre** options — c'est le cas de CSO-6702 — sans contradiction : les options sont des alternatives, jamais tenues à la fois.
Seule une répétition **à l'intérieur** d'une option resterait absurde.

L'anomalie « linked sections under N top-level sections » est **supprimée** : elle gardait l'hypothèse qu'on abandonne, et elle n'a plus rien à signaler.
La réconciliation arithmétique du §6 (« N sections offertes » contre le nombre de sections de premier niveau trouvées) est **conservée** : c'est elle qui a débusqué le bogue de `2026-07-sections-de-premier-niveau-par-ascendance`.

## Conséquences

Les quinze fixtures existantes renomment la clé.
Deux seulement changent de contenu, et exactement comme le modèle le prédit :

| fixture | avant | après |
|---|---|---|
| `gci-1007` | `[[84664], [84665 A, 84666 B]]` | `[[84664, 84665 A], [84664, 84666 B]]` |
| `gci-2010` | `[[14733 A, 14734 B]]` | `[[14733 A], [14734 B]]` |

Les treize autres sont `[[X]]` et ne bougent pas — le nouveau modèle est une généralisation stricte de l'ancien sur tout ce qu'il représentait correctement.

Deux fixtures rejoignent le corpus gelé : **`ift-1004`** (deux sections de premier niveau, l'une avec laboratoires, l'autre sans — le falsificateur du §5) et **`cso-6702`** (deux sections partageant un NRC lié).

La section de premier niveau est recopiée dans chaque option qu'elle engendre : ARC-6501 la répète 23 fois.
Le coût en volume est réel mais marginal, et il achète l'impossibilité de construire une inscription invalide.

## Alternatives rejetées

- **Imbriquer les sections liées dans la section** (`Section { linked: Vec<LinkedSection> }`) : fidèle au DOM, sans recopie, et la profondeur 2 devient structurelle si `LinkedSection` n'a pas de champ `linked`. Rejeté parce que le solveur doit alors traiter deux niveaux et refaire le développement à chaque usage, alors que le produit est calculable une fois pour toutes à l'écriture du snapshot — et que la donnée resterait un modèle à interpréter plutôt qu'une liste de choix.
- **Garder `groups` et ajouter une contrainte à côté** (« ces liées appartiennent à cette section ») : réintroduit la logique métier hors du type, ce que la forme des données porte déjà pour l'arbre des préalables.
- **Renommer le sens sans renommer le champ** : les snapshots déjà commités se reliraient sans erreur avec une sémantique inversée — la pire des ruptures, silencieuse.
- **Traiter les sections liées comme obligatoires plutôt que comme un choix** : vrai pour CSO-6702 (une seule liée), faux pour IFT-1004 (deux laboratoires dont un seul est à choisir). Une option par liée couvre les deux cas sans les distinguer.
