# La borne « 1000 à 4999 » d'une exigence de crédits est retirée du texte

Date : 2026-07-20

**Remplacé le 2026-07-20 par `2026-07-bornes-de-credits-toutes-retirees`** : toute borne est retirée, pas seulement celle du premier cycle. Le contexte ci-dessous reste valide, la décision non.

## Contexte

Le premier scrape de GMC a produit trois anomalies « Malformed prerequisites course code. », toutes sur la même forme :

```
 ET GMC-1024 ET GMC-2001 ET  1000 à 4999 Crédits exigés : 15
```

— GMC-1590, GMC-2580 et GMC-3034.
Le tokenizer lisait « 1000 » comme un sigle de cours, échouait, et **toute** l'expression basculait en `Prerequisites::Raw` : dans le cas de GMC-3034, dix sigles parfaitement lisibles étaient perdus pour le solveur avec la borne.
C'est le mode d'échec déjà corrigé une fois par l'ADR `2026-07-credits-exiges-sans-programme`.

La même borne existe collée à une matière — ACT-4114 :

```
ACT-2002 ET ACT-2003 ET ACT-1000 à 4999, Crédits exigés : 39
```

Ici l'exigence porte bien sur ACT ; seule la borne empêchait de la lire comme le `MATIÈRE, Crédits exigés : N` que la grammaire connaît déjà.

Dans les deux cas la borne restreint le décompte aux cours numérotés de 1000 à 4999, c'est-à-dire au premier cycle.

## Décision

`tokenize_prereq_raw` retire du texte les deux écritures de la borne avant de découper en mots — `-1000 à 4999` puis `1000 à 4999` — comme il retire déjà les `*`.
Il reste alors les deux formes que la grammaire lit sans changement : `Crédits exigés : N` (programme `None`) et `ACT, Crédits exigés : N`.
Aucune branche nouvelle dans le tokenizer, aucun champ nouveau dans `ProgramCredits`.

La borne peut disparaître parce que le périmètre du planificateur coïncide avec elle :
les crédits préuniversitaires (numéros sous 1000) ne comptent nulle part dans un cheminement de baccalauréat, et le cas d'un étudiant de cycle supérieur revenant suivre des cours de premier cycle est hors périmètre, comme le troisième cycle l'est déjà (ADR `2026-07-troisieme-cycle-hors-perimetre`).
Sur l'ensemble des crédits que le planificateur sait compter, « 1000 à 4999 » désigne le tout : la borne n'ajoute aucune contrainte.

Toute autre borne — « 6000 à 9999 », « 1000 à 2999 » — reste hors grammaire, donc signalée : celle-là restreindrait réellement le décompte, et l'élargir en silence violerait « jamais ignoré en silence ».
Le texte source reste dans `raw` de toute façon, la borne n'est donc pas perdue du snapshot.

GMC-1590 (forme nue) et ACT-4114 (forme collée à la matière) rejoignent les fixtures gelées.

## Alternatives rejetées

- **Reconnaître la séquence exacte dans le tokenizer** (une branche `["1000", "à", "4999", "Crédits", …]`) : première version écrite, remplacée — sept mots à apparier pour le seul effet de ne rien produire, et la forme collée à la matière aurait demandé une seconde branche.
- **Accepter n'importe quelle borne `N à M` et l'ignorer** : une exigence de deuxième cycle deviendrait silencieusement une exigence sur tous les crédits — un préalable satisfait à tort, la pire des sorties pour un solveur.
- **Porter la borne dans `ProgramCredits`** (un champ `levels`) : fidèle à la source, mais aucun consommateur ne saurait quoi en faire, puisque le planificateur ne compte que du premier cycle ; un champ mort dans toutes les fixtures.
- **Laisser l'expression en brut** : c'est le comportement qu'on corrige — les dix cours corrects de GMC-3034 partaient avec la borne.

## Effet mesuré

Sur GMC, les trois anomalies ont disparu (run du 2026-07-20) ; GMC-1590 sérialise `{"all": ["GMC-1003", "GMC-1024", "GMC-2001", {"program_credits": {"program": null, "credits": 15}}]}`.
