# Un cheminement par fichier, sous `data/cheminements/`

Date : 2026-08-29

> Dépassé en partie le 2026-08-30 par l'ADR `2026-08-retrait-de-l-aller-retour-json-du-cheminement` : l'interface ne lit ni n'écrit plus ce fichier.
> Le format un-cheminement-par-fichier reste celui de `data/cheminements/`, écrit et vérifié à la main.

## Contexte

Le format `{code}-{semester}.manuel.json` (ADR `2026-08-fichier-manuel-de-programme-millesime`) portait un tableau `cheminements_types`, chaque entrée étiquetée par son `label` : « Sciences de la nature », « Technique de génie mécanique - Scolarité préparatoire complétée ».
Un fichier tenait de un à cinq cheminements.

L'interface a besoin de lire un cheminement depuis un fichier que l'étudiant choisit, et d'en écrire un depuis son organigramme.
Le format en tableau imposait alors une question sans bonne réponse : un fichier à cinq cheminements demande un choix, donc un sélecteur, donc un état intermédiaire entre le clic et le résultat — pour un cas que l'export ne produit jamais, puisqu'un organigramme est un cheminement et un seul.

## Décisions

- **Le document est le cheminement**, sans enveloppe ni étiquette :

  ```json
  { "completed": ["GMC-1024"],
    "sessions": [{ "semester": "A26", "courses": ["GMC-1001"] }] }
  ```

  `core::program_manual::{ProgramManual, CheminementType}` devient `core::cheminement::Cheminement`, sans `label` ni `cheminements_types`.

- **Le nom du fichier porte l'identité** : `data/cheminements/{code}-{semester}[-{concentration}].json`.
  La concentration est le `label` en snake case, accents repliés (`sciences_de_la_nature_profil_international`) — le nom voyage par URL, par interpréteur de commandes et par sélecteur de fichiers sur trois systèmes.
  Un millésime à variante unique garde le nom nu : `B-GEX-A26.json`.

- **`data/cheminements/` plutôt que `data/programmes/`** : ces fichiers ne sont plus le compagnon d'un snapshot scrapé — ils ne suivent plus son nom, et un même millésime en produit maintenant plusieurs.

- **Conversion** : 19 fichiers → 26, un par cheminement.
  Chacun revérifié entrée par entrée contre les CSV d'origine du dépôt JS (`cac693a^`) : `completed`, ordre des sessions, liste de cours. Aucun écart, aucun cheminement créé ni perdu.

- **Le bloc `provenance` de l'export voyage sans être lu.**
  `Cheminement` n'a pas de `deny_unknown_fields` : l'export écrit date, programme, millésime, concentration et empreintes de code et de données (EXP-1), et la lecture les ignore.
  Le fichier exporté est donc rechargeable tel quel, et déposable tel quel dans `data/cheminements/`.

- **Le chargement remplace le document** — il ne fusionne pas — en un seul acte étiqueté « Cheminement chargé » (ACT-2).
  Tout ce qui nomme un cours ou une session repart de zéro : épingles, placement affiché, cours manuels, sections forcées, crédités, électifs, origines d'électifs, ententes de règle, libellés de session.
  Survivent les seuls réglages dont le fichier ne parle pas : programme, plafond de crédits, concomitance, scolarité préparatoire, corrections de préalables.
  Le plan est reconstruit depuis `Plan::default()` plutôt que vidé champ par champ — un champ ajouté plus tard est alors vidé par défaut, ce qui est le sens sûr ; l'inverse en oublie un et le laisse fuir dans la nouvelle grille.

  Garder les électifs, comme la première version le faisait, suffisait à ce que `auto_propose` les rasseye par-dessus la grille fraîchement chargée : à l'usage, les cours s'additionnaient.

- **Rien n'est laissé tomber en silence.**
  Les grilles officielles emploient des jetons qui ne sont pas des sigles (`OPT-ION1`, `AUC-HOIX`, `LAN-GUES`) : la porte du catalogue les refuse et le bilan les nomme, un par ligne.
  Deux occurrences d'un même sigle — `B-GEX-A26` place GMN-2902 en H28 *et* en H29 — gardent la première session et nomment la seconde.

## Ce que cela change en aval

Le dépôt JS `grille-de-cheminement-interactive` lit `data/programmes/{code}-{millesime}.manuel.json` par URL (`js/sauvegarde.js`).
Ces fichiers n'existent plus : son « Charger un cheminement type » (US-23) casse dès la fusion vers `main`, d'où Pages déploie.
À porter à son `CORRECTIFS-AMONT.md` — chemin, nom et forme changent tous les trois.

## Alternatives rejetées

- **Garder le tableau et choisir dans l'interface** : ramène le sélecteur, donc l'état intermédiaire, donc le tiroir — pour un cas que l'export ne crée jamais.
- **Garder le tableau et prendre la première entrée** : silencieux sur quatre cheminements pour le B-GMC A26, ou bavard d'un avertissement que personne ne peut suivre d'action.
- **Garder les accents dans le nom du fichier** : lisible, mais `B-GMC-A26-technique_de_génie_mécanique.json` traverse mal une URL et un dépôt partagé entre systèmes.
