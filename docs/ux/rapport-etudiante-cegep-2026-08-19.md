# Rapport d'exploration — Élodie, finissante au cégep, hésite entre génie civil / mécanique / physique

Date : 2026-08-19
Testé sur : http://localhost:8000 (session de navigateur isolée `elodie`)

## Contexte de la séance

Je pars de zéro (aucun cours réussi) et je compare les baccalauréats en génie civil (B-GCI), génie mécanique (B-GMC) et génie physique (B-GPH) : premier contact, cheminement par défaut, essais de concentrations, allers-retours entre programmes, rechargement de page, et répétition de certaines actions pour voir si le comportement est stable.

---

### L'application affiche un cheminement d'un autre programme sous le nom du programme qu'on vient de choisir

- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** :
  1. Choisir « Baccalauréat en génie civil » (B-GCI) — le cheminement se génère (76/120 cr, cours GCI-xxxx dans chaque session).
  2. Cliquer « changer » (en-tête).
  3. Cliquer « Choisir » sur « Baccalauréat en génie mécanique » (B-GMC).
- **Attendu** : un nouveau cheminement généré pour B-GMC (cours GMC-xxxx, GPH/MAT/etc. propres à ce programme), ou au minimum un avertissement disant que mes anciens choix ne s'appliquent pas.
- **Observé** : l'en-tête affiche bien « Baccalauréat en génie mécanique (B-GMC version H27) — Cheminement sans concentration » et « 85/120 cr au bac », mais le ruban de sessions garde les cours de génie civil (GCI-1000, GCI-1001, GCI-1010, GCI-1011, GLG-1000 en A1, etc.) — jamais un seul cours GMC-xxxx. Le panneau « Obligatoires » affiche « 7/35 » pour B-GMC en comptant les cours GCI qui portent par coïncidence le même sigle qu'un cours GMC requis. Les messages d'avertissement sous l'horaire citent encore des cours GCI-2003, GCI-2006, GCI-2011… comme non plaçables — alors que ces cours n'existent pas dans le programme actuellement affiché.
  Reproduit une seconde fois (après réinitialisation, GCI → changer → GMC) avec le même résultat exact (85/120 cr, mêmes cours GCI dans le ruban) ; reproduit une troisième fois avec la paire GPH → GCI (113/120 cr, ruban rempli de cours GPH sous l'en-tête « génie civil »). Le bogue est général, pas propre à une paire de programmes.
  Capture : `06-changer-residu.png` (en-tête « aucun programme choisi » mais ruban encore rempli de GCI), `07-gmc-residu-gci.png` (en-tête « génie mécanique » mais ruban et messages encore en GCI).
  Erreur console : aucune à ce stade (le rendu ne plante pas, il affiche juste les mauvaises données).

C'est le pire scénario possible pour Élodie : c'est précisément en comparant plusieurs programmes qu'on tombe dessus, et rien à l'écran ne dit que les chiffres et les cours affichés ne correspondent pas au programme annoncé.

---

### Changer de concentration alors que l'état est déjà incohérent fait planter l'application

- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** :
  1. Reproduire le bogue ci-dessus (GCI → changer → Choisir B-GMC, ruban resté en GCI).
  2. Dans le panneau, changer « Concentration » de « Cheminement sans concentration » à « Robotique ».
- **Attendu** : le cheminement se recalcule pour la concentration Robotique (ou, à défaut, un message d'erreur clair sans casser l'écran).
- **Observé** : panique Rust/Dioxus affichée en plein écran : « App panicked! See console for details. » — `assertion left == right failed: keyed siblings must each have a unique key (left: 56, right: 55)`. Le reste de l'interface devient inutilisable derrière le bandeau d'erreur.
  Capture : `08-crash-panic.png`.
  Erreur console : `panicked at .../dioxus-core-0.7.10/src/diff/iterator.rs:107:17: assertion left == right failed: keyed siblings must each have a unique key — left: 56, right: 55` puis `wasm-bindgen: imported JS function that was not marked as catch threw an error: unreachable`.

---

### L'état incohérent (mauvais programme + mauvais cours) survit au rechargement de la page

- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** : après le plantage ci-dessus, `reload` la page.
- **Attendu** : soit l'état corrompu est nettoyé au chargement, soit on retombe au moins sur un état cohérent (même incomplet).
- **Observé** : la page se recharge sans planter cette fois, mais l'en-tête affiche « Baccalauréat en génie mécanique (B-GMC version H27) — Robotique » alors que le ruban de sessions montre toujours les cours GCI-1000/1001/1010/1011/GLG-1000 de génie civil, avec le même message « 1 cours hors grille » et le même avertissement de recherche infructueuse. Impossible de revenir à un état propre sans passer par le bouton « Réinitialiser » (qui, lui, fonctionne et efface tout — mais rien n'indique à l'utilisatrice que c'est la seule porte de sortie).
  Capture : `09-apres-reload-etat-corrompu.png`.

---

### Le cheminement par défaut de génie mécanique ne place aucun cours (0/120 crédits), sans que rien ne l'explique clairement

- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Cliquer « Réinitialiser » pour repartir propre.
  2. Choisir directement « Baccalauréat en génie mécanique » (B-GMC), version H27 — sans passer par un autre programme.
- **Attendu** : comme pour génie civil (76/120 cr placés automatiquement) et génie physique (104/120 cr, 36/36 cours obligatoires), un cheminement de départ raisonnable avec au moins les cours de première session.
- **Observé** : 0/120 cr au bac, 0/35 obligatoires, les onze sessions affichent « à planifier », l'horaire hebdomadaire dit « Aucun cours avec horaire publié pour cette session ». Le seul message est « 37 cours sans session : le solveur a rempli au mieux et n'a pas pu les placer » et « La recherche s'est arrêtée avant d'avoir tout exploré, sans rien trouver pour l'instant » — un texte qui laisse croire qu'un effort partiel a été fait, alors qu'aucun cours, pas même GMC-1001 (« rentrerait dans la session affichée » d'après son propre descriptif), n'a été retenu. Reproduit à l'identique avec la version A26 de B-GMC (pas un problème de millésime précis).
  En revanche, le placement manuel fonctionne : cliquer sur le bouton de session « A1-A26 » à côté d'un cours (ex. GMC-1002) l'ajoute bien et met les crédits à jour — donc le moteur de génération automatique est spécifiquement en cause, pas le placement en général.
  Captures : `10-gmc-vide.png`, `11-gmc-a26-vide.png`.

Pour Élodie qui compare trois programmes côte à côte, ça donne l'impression que génie mécanique « ne marche pas » dans l'outil, ce qui biaise la comparaison sans rapport avec le programme réel.

---

### Choisir une concentration ne change (presque) jamais ce qu'on voit dans le cheminement généré

- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Choisir génie civil, cheminement « sans concentration » (76/120 cr, mêmes cours dans chaque session).
  2. Changer la concentration pour « Eau et environnement », puis pour « Structures et matériaux ».
  3. Refaire l'exercice en génie physique avec « Aéronautique et aérospatiale » puis « Photonique ».
- **Attendu** : en choisissant une concentration, je m'attends à voir de nouveaux cours apparaître dans les sessions concernées (surtout la session H6-H29, complètement vide dans le cheminement de base), pour pouvoir comparer la charge et le contenu réel entre concentrations.
- **Observé** : dans les deux programmes, le ruban de sessions reste identique au cours pixel près (mêmes sigles, mêmes crédits par session, 76/120 pour B-GCI et 104/120 pour B-GPH peu importe la concentration choisie). Seul le panneau de règles en dessous change : une nouvelle « Règle » apparaît (« Règle 1 - 12 cr 0/12 cr » pour Eau et environnement vs la même étiquette « Règle 1 - 12 cr » mais un contenu différent — Charpentes en bois, Sécurité incendie, etc. — pour Structures et matériaux), mais avec 0 crédit satisfait : aucun cours de la concentration n'est pré-sélectionné ni placé automatiquement. Il faut cliquer soi-même sur chaque cours listé et choisir une session (bouton « automatique » ou une session précise) pour voir la concentration se refléter dans l'horaire. Rien à l'écran n'indique qu'il faut faire ça pour « voir » la concentration.
  Capture : `03-gci-eau-environnement.png` (comparer avec `02-gci-sans-concentration.png` — ruban identique), `04-panel-rules.png`/`05-panel-rules2.png` (le contenu de la règle diffère bien une fois déplié).

Pour une utilisatrice qui compare des concentrations précisément pour voir « qu'est-ce que ça change dans mon horaire », c'est le pire endroit pour ne rien montrer automatiquement.

---

### « Aucune » et « Cheminement sans concentration » se ressemblent mais ne couvrent pas les mêmes exigences

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Choisir génie civil, concentration « Cheminement sans concentration » : une règle « Règle 1 - 15 cr 0/15 cr » apparaît dans le panneau, en plus des Stages et de la Scolarité préparatoire.
  2. Changer la concentration pour « Aucune ».
- **Attendu** : soit les deux options sont vraiment équivalentes, soit une explication (infobulle, texte) dit pourquoi il y a deux choix qui se ressemblent tant.
- **Observé** : sous « Aucune », l'en-tête perd le sous-titre de concentration et la règle « Règle 1 - 15 cr » disparaît complètement du panneau — comme si ces 15 crédits n'étaient plus exigés. Rien n'explique à l'écran la différence entre les deux options ni laquelle correspond au cheminement réellement reconnu par la faculté. Une étudiante pressée pourrait choisir « Aucune » en pensant que c'est le choix neutre, et sous-estimer de 15 crédits ce qu'il lui reste à faire.

---

### Le cheminement par défaut de génie civil laisse 6 cours obligatoires non placés et une session complètement vide, avec des messages en jargon

- **Gravité** : majeur
- **Type** : friction
- **Reproduction** : choisir « Baccalauréat en génie civil », cheminement sans concentration, à partir de zéro.
- **Attendu** : pour une première admise sans cours réussi, je m'attends à un cheminement complet ou, s'il ne l'est pas, une explication simple de ce qui manque et pourquoi.
- **Observé** : 76/120 cr placés, 26/32 cours obligatoires ; la session H6-H29 (milieu du parcours) reste entièrement « à planifier », sans aucun cours. Le message d'avertissement énumère GCI-2003, GCI-2006, GCI-2011, GCI-2012, GCI-3000, GCI-3333 avec des explications comme « ses préalables sont insatisfiables avec les cours fournis » ou « aucune place ne restait — les autres cours et le plafond remplissent déjà chaque session où il est offert ». Ces phrases supposent qu'on comprenne déjà le fonctionnement interne du solveur (plafond de crédits, satisfiabilité de préalables) — un vocabulaire que je ne maîtrise pas en tant que future étudiante, et qui ne me dit pas concrètement quoi faire (augmenter le plafond ? ajouter une session ? attendre une session future ?).
  Capture : `02-gci-sans-concentration.png`.

---

### Les messages « le solveur a rempli au mieux » ne distinguent pas un échec partiel d'un échec total

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : comparer le message affiché pour génie civil (26/32 obligatoires placés, 6 cours en échec) et pour génie mécanique (0/35 placés, 37 cours en échec).
- **Attendu** : un message différent selon la gravité — au minimum, un ton qui reflète qu'aucun cours n'a pu être placé dans le cas de génie mécanique.
- **Observé** : le même texte générique « X cours sans session : le solveur a rempli au mieux et n'a pas pu les placer » s'affiche dans les deux cas, qu'il reste 6 cours ou 37. Pour génie mécanique, où littéralement rien n'a été placé, ce message minimise ce qui ressemble plutôt à un échec complet.

---

### Pas d'outil de comparaison : je dois tout retenir de tête

- **Gravité** : mineur
- **Type** : pas encore construit
- **Reproduction** : essayer de comparer les trois programmes côte à côte (nombre de crédits placés, charge par session, cours communs, horaires types de première session).
- **Attendu** : un tableau récapitulatif, ou au moins la possibilité d'ouvrir deux programmes en parallèle, pour comparer sans naviguer dans trois onglets mentaux.
- **Observé** : l'outil ne montre qu'un programme à la fois ; changer de programme remplace tout (et, comme vu plus haut, le fait mal). Aucune vue de comparaison, aucun résumé de « points communs entre B-GCI/B-GMC/B-GPH ». Je n'ai pas trouvé de fonctionnalité dédiée à la comparaison, et je ne l'ai vue nulle part dans `docs/project_plan.md` — ce n'est prévu à aucun jalon recensé (les jalons restants concernent le cron CI, jalon 5, déjà en place, et les préférences/partage d'URL/cours manuel du jalon 10). Je considère donc ceci comme un souhait hors-plan plutôt qu'un report de jalon précis.

---

### Non testé — à noter explicitement

- Le bouton « Partager » (partage d'URL) : présent dans l'en-tête à chaque étape, mais je ne l'ai pas testé (copier le lien, l'ouvrir ailleurs) faute de budget d'actions restant après les bogues plus graves.
- Le sélecteur « Profil » (ex. Profil développement durable, Profil international) : vu dans le panneau mais jamais essayé.
- Les concentrations « Génie du développement durable » (génie mécanique), et 5 des 7 concentrations de génie physique (Électricité/électronique et puissance, Environnement, Génie médical et biophotonique, Génie des matériaux, Signaux et communications) : non essayées, par manque de temps après avoir documenté les bogues bloquants.
- Le glisser-déposer d'un cours entre sessions (mentionné dans l'historique Git du projet) : non testé.
- Le crédit rétroactif d'un cours déjà réussi (bouton « créditer ») : vu dans l'interface mais pas testé de bout en bout.

---

## Impression générale

Non, dans l'état actuel, cet outil ne m'aiderait pas à choisir entre génie civil, génie mécanique et génie physique — et il pourrait même m'induire en erreur. Le problème le plus grave n'est pas visuel ni ergonomique : c'est que **changer de programme ne régénère pas le cheminement**, il garde silencieusement les cours de l'ancien programme sous le nom du nouveau, avec des crédits et des avertissements qui n'ont plus rien à voir avec la réalité. Comme comparer plusieurs programmes est justement le geste que je répète sans arrêt dans une vraie séance de choix, je tombe sur ce bogue presque à chaque fois — et une fois, il a carrément fait planter l'application. Que cet état corrompu survive même à un rechargement de page aggrave tout : je ne peux m'en sortir qu'en cliquant « Réinitialiser », ce qui efface aussi tout mon travail légitime, sans que rien ne me dise que c'est la solution.

Même sans ce bogue, deux choses m'empêcheraient de trancher : génie mécanique ne me montre aucun cheminement de départ (0 cours placés) alors que génie civil et génie physique en proposent un, ce qui rend la comparaison injuste envers ce programme ; et choisir une concentration — le cœur de ma décision — ne change presque jamais ce que je vois dans mon horaire tant que je n'ai pas moi-même cliqué sur chaque cours de la liste. Pour une utilisatrice qui, comme moi, veut savoir « à quoi ressemblerait vraiment ma session si je prends telle concentration », l'outil demande déjà le travail que je venais lui demander de faire à ma place.
