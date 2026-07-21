# Les cours d'appoint sont réintégrés au catalogue

Date : 2026-07-21

Remplace `2026-07-cours-dappoint-hors-perimetre`.

## Contexte

L'ADR `2026-07-cours-dappoint-hors-perimetre` retirait du catalogue les 28 sigles `0xxx` — les **cours d'appoint** (`CHM-0150`, `PHY-0250`, `MAT-0150`…) —, jugés hors périmètre parce qu'ils ne comptent pour aucun crédit de programme.

Deux besoins, portés par le directeur du baccalauréat, renversent cette décision :

- La page de certains cours porte le champ **« Préalables préuniversitaires nécessaires s'il y a lieu : … »**, qui nomme précisément ces `0xxx` (ADR `2026-07-prealables-preuniversitaires-fusionnes`).
  Pour qu'un préalable pointe vers une entrée réelle, le cours d'appoint doit exister au catalogue.
- L'étudiant qui n'a pas le préalable collégial doit pouvoir placer le cours d'appoint dans son horaire : c'est une activité qui occupe une plage, pas une simple mention.

## Décision

**Les cours `0xxx` sont réintégrés au catalogue.**
Le prédicat de `Catalogue::from_entries` ne retire plus que les `8xxx` (« le premier chiffre du numéro est `8` »).

Le cycle lu sur la page reste l'autorité du périmètre : un `0xxx` se déclare « Préuniversitaire », un cycle désormais dans le périmètre pour un cours (ADR `2026-07-cycle-preuniversitaire-cours-seulement`).

## Conséquences

Le catalogue regagne ses 28 entrées `0xxx`, chacune désormais visitée et analysée.
L'empreinte de périmètre du cache (`scope_tag`) passe de « 1,2 » à « 0,1,2 », ce qui périme d'office les verdicts « hors périmètre » écrits sous l'ancienne règle — sans purge manuelle.

Le `8xxx` reste exclu par le sigle, avant toute requête HTTP.

## Alternatives rejetées

- **Ne réintégrer que les `0xxx` cités en préalable** : exigerait une seconde passe pour collecter les sigles référencés et compliquerait le filtre, alors que le directeur veut l'ensemble des cours d'appoint de retour.
