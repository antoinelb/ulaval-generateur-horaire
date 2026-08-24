# User stories — grille de cheminement interactive

Ce dossier décrit en langage naturel les situations que la grille doit gérer.
Chaque fichier est une histoire autonome, lisible sans le code, qui sert à deux choses :

- **tests manuels** pendant le développement — le scénario se joue tel quel dans un navigateur;
- **tests e2e Playwright** — la section « Repères pour le test e2e » nomme les sélecteurs et les états observables.

## Conventions

- Une histoire par fichier; le numéro `US-NN` ne change jamais, même si l'histoire est réécrite.
- Toutes les histoires suivent le même gabarit : persona, intention, préconditions, scénario, résultats attendus, repères e2e, variantes et cas limites.
- Le genre des personas alterne d'une histoire à l'autre.
  Daniel, directeur de programme, est la seule persona réelle du projet : il incarne les histoires de qualité des données.
- Les sigles cités (`MAT-1900`, `GEX-1580`, `CHM-1903`…) viennent des données réelles de `data/` : un test peut s'y fier.
- Une histoire qui décrit une fonctionnalité non encore livrée le dit explicitement par **À venir**.
- Un écart entre le comportement observé et le comportement souhaitable est écrit comme tel, avec la mention du choix à trancher — ces histoires ne décrivent pas un système parfait.

## Prérequis pour jouer les scénarios

L'application charge ses données par `fetch` de chemins relatifs : elle exige un serveur HTTP.
`python -m http.server` à la racine du dépôt suffit.

## Index

### Parcours d'étudiants

| # | Histoire |
| --- | --- |
| 01 | [Parcours nominal, tous les préalables préuniversitaires](01-parcours-nominal-tous-les-prealables.md) |
| 02 | [Préalables préuniversitaires manquants (sciences de la santé)](02-prealables-preuniversitaires-manquants.md) |
| 03 | [Session d'échange à l'étranger](03-session-d-echange-a-l-etranger.md) |
| 04 | [Équivalences obtenues dans une autre université](04-equivalences-d-une-autre-universite.md) |
| 05 | [Terminer un bac de quatre ans en trois ans](05-bac-de-quatre-ans-en-trois-ans.md) |
| 06 | [Maximum de douze crédits par session](06-maximum-de-douze-credits-par-session.md) |
| 07 | [Échec d'un cours charnière](07-echec-d-un-cours-charniere.md) |
| 08 | [Admission à la session d'hiver](08-admission-a-la-session-d-hiver.md) |
| 09 | [Changement de concentration](09-changement-de-concentration.md) |
| 10 | [Changement de programme](10-changement-de-programme.md) |
| 11 | [Millésime antérieur et cours retiré du répertoire](11-millesime-anterieur-et-cours-retire.md) |
| 12 | [Stages obligatoires, crédits en sus](12-stage-obligatoire-credits-en-sus.md) |
| 13 | [Exigence linguistique, personne francophone](13-exigence-linguistique-francophone.md) |
| 14 | [Exigence linguistique, personne non francophone](14-exigence-linguistique-non-francophone.md) |
| 15 | [Étalement sur cinq ans](15-etalement-sur-cinq-ans.md) |
| 16 | [Cours au choix et options à déterminer](16-cours-au-choix-et-options-a-determiner.md) |

### Chargement, sauvegarde et sélection

| # | Histoire |
| --- | --- |
| 20 | [Choisir le programme](20-choix-du-programme.md) |
| 21 | [Choisir la session d'admission](21-choix-de-la-session-d-admission.md) |
| 22 | [Choisir la spécialisation](22-choix-de-la-specialisation.md) |
| 23 | [Charger un cheminement type](23-chargement-d-un-cheminement-type.md) |
| 24 | [Charger un cheminement depuis un fichier local](24-chargement-d-un-cheminement-local.md) |
| 25 | [Charger les cours complétés depuis Capsule](25-chargement-des-cours-completes-depuis-capsule.md) |
| 26 | [Sauvegarder le cheminement en CSV](26-sauvegarde-du-cheminement-en-csv.md) |
| 27 | [Imprimer le cheminement](27-impression-du-cheminement.md) |

### Manipulation de la grille

| # | Histoire |
| --- | --- |
| 28 | [Placer un cours par glisser-déposer](28-placer-un-cours-par-glisser-deposer.md) |
| 29 | [Déplacer une pastille et cascade](29-deplacer-une-pastille-et-cascade.md) |
| 30 | [Retirer un cours de la grille](30-retirer-une-pastille-de-la-grille.md) |
| 31 | [Ajouter et retirer des sessions](31-ajouter-et-retirer-des-sessions.md) |
| 32 | [Ajuster le nombre de rangées visibles](32-ajuster-les-rangees-visibles.md) |
| 33 | [Rechercher un cours dans le panneau](33-rechercher-un-cours.md) |
| 34 | [Choisir la section d'un cours](34-choisir-la-section-d-un-cours.md) |
| 35 | [Ouvrir la grille horaire d'une session](35-ouvrir-la-grille-horaire-de-session.md) |

### Vérifications

| # | Histoire |
| --- | --- |
| 36 | [Conflits d'horaire](36-conflits-d-horaire.md) |
| 37 | [Bilan des crédits et couverture des règles](37-bilan-des-credits-et-couverture-des-regles.md) |
| 38 | [Case « Scolarité préparatoire complétée »](38-scolarite-preparatoire-completee.md) |
| 39 | [Cours non offert à la session choisie](39-cours-non-offert-a-la-session.md) |
| 40 | [Préalable simultané](40-prealable-simultane.md) |
| 41 | [Préalable exprimé en crédits accumulés](41-prealable-en-credits-accumules.md) |
| 42 | [Expressions de préalables ET / OU](42-expressions-de-prealables-et-ou.md) |

### Robustesse et horizon

| # | Histoire |
| --- | --- |
| 50 | [Données indisponibles ou corrompues](50-donnees-indisponibles-ou-corrompues.md) |
| 51 | [Fenêtres contextuelles bloquées](51-fenetres-contextuelles-bloquees.md) |
| 52 | [Sigles inconnus ou mal formés](52-sigles-inconnus-ou-malformes.md) |
| 53 | [Couleurs des pastilles et redimensionnement](53-couleurs-et-redimensionnement.md) |
| 54 | [Reprise après rechargement](54-reprise-apres-rechargement.md) |
| 55 | [Fonctionnalités à venir apportées par le solveur](55-fonctionnalites-a-venir-du-solveur.md) |

### Concentrations et profils

Une histoire par concentration et par profil des programmes présents dans `../generateur_horaire/data/programmes/`.
**B-ANT et M-GEX ne sont pas encore servis par ce dépôt** : leurs histoires portent en tête les fichiers à déposer pour les jouer.

| # | Programme | Spécialisation |
| --- | --- | --- |
| 60 | B-ANT | [Concentration « Environnement »](60-b-ant-concentration-environnement.md) |
| 61 | B-ANT | [Concentration « Études autochtones »](61-b-ant-concentration-etudes-autochtones.md) |
| 62 | B-ANT | [Profil international](62-b-ant-profil-international.md) |
| 63 | B-GCI | [Cheminement sans concentration](63-b-gci-cheminement-sans-concentration.md) |
| 64 | B-GCI | [Concentration « Eau et environnement »](64-b-gci-concentration-eau-et-environnement.md) |
| 65 | B-GCI | [Concentration « Géotechnique »](65-b-gci-concentration-geotechnique.md) |
| 66 | B-GCI | [Concentration « Structures et matériaux »](66-b-gci-concentration-structures-et-materiaux.md) |
| 67 | B-GCI | [Profil développement durable](67-b-gci-profil-developpement-durable.md) |
| 68 | B-GCI | [Profil international](68-b-gci-profil-international.md) |
| 69 | B-GEX | [Profil international](69-b-gex-profil-international.md) |
| 70 | B-GIN | [Concentration « Approche généraliste »](70-b-gin-concentration-approche-generaliste.md) |
| 71 | B-GIN | [Concentration « Chaîne logistique »](71-b-gin-concentration-chaine-logistique.md) |
| 72 | B-GIN | [Concentration « Intelligence numérique des systèmes »](72-b-gin-concentration-intelligence-numerique.md) |
| 73 | B-GIN | [Concentration « Informatisation des systèmes d'entreprise »](73-b-gin-concentration-informatisation-des-systemes.md) |
| 74 | B-GIN | [Concentration « Systèmes productiques et distributiques »](74-b-gin-concentration-systemes-productiques.md) |
| 75 | B-GIN | [Profil entrepreneurial](75-b-gin-profil-entrepreneurial.md) |
| 76 | B-GIN | [Profil international](76-b-gin-profil-international.md) |
| 77 | B-GMC | [Cheminement sans concentration](77-b-gmc-cheminement-sans-concentration.md) |
| 78 | B-GMC | [Concentration « Robotique »](78-b-gmc-concentration-robotique.md) |
| 79 | B-GMC | [Concentration « Génie du bâtiment durable »](79-b-gmc-concentration-genie-du-batiment-durable.md) |
| 80 | B-GMC | [Passage intégré au deuxième cycle](80-b-gmc-profil-passage-integre-deuxieme-cycle.md) |
| 81 | B-GPH | [Concentration « Aéronautique et aérospatiale »](81-b-gph-concentration-aeronautique-et-aerospatiale.md) |
| 82 | B-GPH | [Concentration « Électricité, électronique et puissance »](82-b-gph-concentration-electricite-electronique-puissance.md) |
| 83 | B-GPH | [Concentration « Environnement »](83-b-gph-concentration-environnement.md) |
| 84 | B-GPH | [Concentration « Génie médical et biophotonique »](84-b-gph-concentration-genie-medical-et-biophotonique.md) |
| 85 | B-GPH | [Concentration « Photonique »](85-b-gph-concentration-photonique.md) |
| 86 | B-GPH | [Concentration « Génie des matériaux »](86-b-gph-concentration-genie-des-materiaux.md) |
| 87 | B-GPH | [Concentration « Signaux et communications »](87-b-gph-concentration-signaux-et-communications.md) |
| 88 | B-GPH | [Profil distinction](88-b-gph-profil-distinction.md) |
| 89 | B-GPH | [Profil international](89-b-gph-profil-international.md) |
| 90 | M-GEX | [Programme sans spécialisation](90-m-gex-programme-sans-specialisation.md) |

## Écarts relevés à l'écriture

Ces points sont documentés dans les histoires concernées et attendent une décision :

- Un cours hors catalogue (`OPT-ETR1`, `OPT-ION1`, `AUC-HOIX`, `EHE-1GEX`) n'a aucune saison enregistrée et est donc marqué « non offert » dans toute session — un faux positif systématique (US-03, US-16, US-39).
- `MAT-0260` est préalable de `MAT-1900` mais absent de la règle « Scolarité préparatoire » du B-GEX A26 : cocher la case ne suffit pas (US-02, US-38).
- Changer de programme ou de millésime vide la grille sans confirmation (US-08, US-10).
- Le choix de section, l'état de la case de scolarité préparatoire, le programme et le millésime ne sont ni sauvegardés dans le CSV ni restaurés (US-26, US-34, US-54).
- Le journal est effacé à chaque vérification : les avertissements de chargement disparaissent au premier déplacement (US-52).
- La boîte « Résumé du programme chargé » est construite mais masquée en dur dans le HTML (US-53).
- Le titre de la fenêtre de grille horaire est figé sur « génie mécanique » (US-35).
- Retirer un cours n'est possible que par glisser-déposer, sans équivalent au clavier (US-30).

Écarts relevés en couvrant les concentrations et les profils :

- **Une règle négociée avec contrainte est impossible à combler.** `courses: "negotiated"` n'est pas un tableau, donc la règle se charge sans aucun cours; le « Profil distinction » du B-GPH exige alors 12 crédits que rien ne peut satisfaire, et gonfle le total exigé du programme (US-88). Le « Passage intégré » du B-GMC, sans contrainte, tombe à 0 crédit et reste inoffensif (US-80).
- **Les sigles d'échange manquent pour tous les programmes sauf le B-GEX.** `EHE-1ANT`, `EHE-1GCI`, `EHE-1GIN` et `EHE-1GPH` sont absents du catalogue **et** des fichiers hors catalogue de leur programme : titre vide, 0 crédit, avertissement de console (US-62, US-68, US-76, US-89).
- **Les notes en prose ne sont affichées nulle part.** Le champ `notes` d'un programme ou d'une spécialisation, et le `raw` d'une règle négociée, ne sont lus par aucun module — contraire à l'invariant « ne jamais rien perdre en silence » (US-70, US-80, US-88, US-90).
- **Des `credits_required` de section dépassent ce que leurs règles permettent** : 18 crédits déclarés pour 15 atteignables au B-GMC (US-77, US-78). À confronter aux pages du répertoire.
- **B-ANT et M-GEX ne sont pas servis par ce dépôt** : leurs fichiers existent dans `../generateur_horaire/data/programmes/` mais ne sont ni dans `data/programmes/`, ni dans `index-programmes.csv`, ni pourvus d'un dossier de fichiers manuels (US-60, US-90).
- **Le champ `cycle` n'est lu par aucun module** : rien ne distingue une maîtrise d'un baccalauréat dans l'interface (US-90).
