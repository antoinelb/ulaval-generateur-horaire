# Le défaut est « sans concentration », pour tous les programmes

Date : 2026-08-30

## Contexte

Élodie, finissante au cégep, compare trois programmes (constat du 2026-08-29) :

> Choisir génie civil ou génie mécanique → concentration par défaut = « sans concentration ».
> Choisir génie physique → concentration par défaut = « Aéronautique et aérospatiale » (la première de la liste), alors qu'une option « Aucune » existe aussi.
> En arrivant sur génie physique, le cheminement montré est déjà celui d'une concentration spécifique, sans que rien n'attire l'œil sur ce choix.

`panel::default_concentration` prenait `concentrations.first()`.
Ce que dit la donnée :

- `data/programmes/B-GCI-A26.json` et les onze `B-GMC-*.json` ouvrent leurs `concentrations` par un bloc **« Cheminement sans concentration »**;
- les huit `B-GIN-*.json` par **« Approche généraliste »**;
- `data/programmes/B-GPH-A26.json` porte sept concentrations, toutes nommées — *aucun* cheminement sans concentration : `['Aéronautique et aérospatiale', 'Électricité, électronique et puissance', 'Environnement', 'Génie médical et biophotonique', 'Photonique', 'Génie des matériaux', 'Signaux et communications']`.

Le `first()` tombait donc sur le bloc neutre pour B-GCI, B-GMC et B-GIN, et sur une concentration spécifique pour B-GPH : un défaut identique dans le code, incohérent à l'écran.

La donnée dit aussi que « sans concentration » est un cheminement réel en B-GPH : ses règles de programme suffisent à remplir les 120 crédits exigés (`Règle 2` : 3 à 15 crédits, `Règle 3` : 0 à 12), aucune note n'exige de concentration, et l'interface offre déjà « Aucune » dans son menu — `cheminement_choices.offers_none` est vrai précisément quand la page ne porte pas de bloc neutre (ADR `2026-08-aucune-retiree-quand-un-bloc-neutre-existe`).
Le correctif « dire qu'une concentration est exigée » ne s'applique donc pas : rien ne l'exige.

## Décision

`panel::default_concentration` ne renvoie plus la première concentration mais **le bloc neutre du millésime quand la page en porte un** (`neutral_concentration` : « Cheminement sans concentration », « Approche généraliste »), et **`None` sinon**.

Le défaut est ainsi le même partout — sans concentration — quelle que soit la façon dont le répertoire l'exprime : par un bloc qui porte ce nom, ou par l'absence de bloc.
C'est le défaut expert-sûr de LAY-3 : le cheminement de base du programme, jamais une spécialisation choisie à la place de l'étudiante.

Conséquence sur l'avis existant : `AlertCause::DefaultConcentration` ne se lève que lorsqu'un bloc a réellement été sélectionné à sa place, et il le nomme — « Concentration « Cheminement sans concentration » sélectionnée par défaut ». En B-GPH, plus rien n'est présélectionné, donc plus d'avis : il ne peut plus nommer une concentration que l'en-tête ne montre pas.

## Alternatives rejetées

- **Garder `first()` et ajouter un avis plus visible en B-GPH** : l'avis existe déjà et disait vrai; le problème n'était pas qu'il se taise, mais que le défaut lui-même diffère d'un programme à l'autre. LAY-3 demande un défaut sûr, pas une explication qui excuse un défaut surprenant.
- **Retirer le bloc neutre du menu et toujours ouvrir sur « Aucune »** : un bloc « Cheminement sans concentration » scrapé porte ses propres règles; le sélectionner et sélectionner « Aucune » ne comptent pas la même chose (ADR `2026-08-aucune-retiree-quand-un-bloc-neutre-existe`). Le bloc neutre *est* « sans concentration » pour ces programmes-là.
- **Reconnaître le bloc neutre par une heuristique de titre plus large** (tout titre contenant « sans concentration » ou « généraliste ») : les six pages connues sont couvertes par les deux libellés exacts; élargir invente une règle que la donnée ne demande pas et risque d'attraper une vraie concentration.
