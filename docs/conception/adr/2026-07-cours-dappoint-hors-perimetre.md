# Les cours d'appoint sont hors périmètre

> **Remplacé par `2026-07-cours-dappoint-reintegres` (2026-07-21)** : les cours `0xxx` sont réintégrés au catalogue, à la demande du directeur, pour porter les préalables préuniversitaires.

Date : 2026-07-20

## Contexte

Le catalogue compte 28 sigles dont le numéro commence par `0` : `MAT-0150 Calcul différentiel`, `FRN-0100 Français écrit`, `CHM-0150 Chimie générale`, `PSY-0100 Introduction à la psychologie`…

Ce sont des **cours d'appoint** (ou de mise à niveau) : ils comblent un préalable du collégial manquant à l'admission.
Ils ne comptent pour aucun crédit du programme et n'apparaissent dans aucun cheminement — un étudiant les suit *avant* ou *à côté* de son bac, jamais comme une activité de sa grille de crédits.

Le filtre `8xxx` d'`2026-07-troisieme-cycle-hors-perimetre` retire déjà, à l'autre extrémité de la numérotation, les activités qui occupent une session sans entrer dans un horaire.
Les `0xxx` posent le même problème par le bas : le générateur les proposerait comme des cours ordinaires, gonflant la recherche du solveur et la liste offerte à l'étudiant avec des activités qu'il ne peut pas choisir.

## Décision

**Les cours `0xxx` sont exclus du catalogue** à la construction (`Catalogue::from_entries`), au même endroit et par la même lecture du sigle que les `8xxx`.
Le prédicat devient « le premier chiffre du numéro est `0` ou `8` ».

Comme pour les `8xxx`, l'exclusion a lieu **avant toute requête HTTP** : le sigle suffit à décider, la page du cours n'est jamais visitée.

## Conséquences

Le catalogue perd 28 entrées sur 8 854 (0,3 %).
L'écart entre le total annoncé par le site et le catalogue produit s'élargit d'autant ; il reste voulu et distinct du bogue de comptage de `2026-07-le-catalogue-est-lunion-des-facettes`.

Un sigle sans tiret reste conservé, et seul le **premier** chiffre compte : `MAT-1050` survit.

## Alternatives rejetées

- **Filtrer sur l'absence de crédits plutôt que sur le numéro** : exigerait de visiter la page pour lire la carte des crédits, alors que le sigle suffit et est déjà dans le catalogue. Le cas « page sans carte de crédits » a par ailleurs son propre traitement (`2026-07-cours-sans-carte-de-credits`).
- **Conserver les `0xxx` et les masquer dans l'interface** : déplacerait une règle de domaine dans la vue, ce qu'interdit la contrainte « toute la logique métier dans `core` ».
