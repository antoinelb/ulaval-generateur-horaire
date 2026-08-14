# Rapport d'utilisation — Camille, 2e année B-GEX (version A26)

Session du 2026-08-13, interface refondue, build `d3ad104`.
Environ 75 actions navigateur, fenêtre 1600×1000.
Captures dans le répertoire scratchpad de la session (`.../scratchpad/shots/`).

---

### Le panneau des règles disparaît complètement, remplacé par un message moitié anglais

- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** :
  1. Choisir B-GEX version A26, cliquer « Proposer un organigramme ».
  2. Ouvrir « Règle 1 - 1 parmi », rattacher GMN-2901 à la Règle 2 par le menu « entente… », puis cliquer son « + » (il se place en H2-H27, la Règle 1 affiche ✓ GMN-2901).
  3. Toujours dans la Règle 1, cliquer le « + » de GMN-2902 (il s'ajoute dans la session affichée).
- **Attendu** : un avertissement du genre « la Règle 1 n'accepte qu'un cours, vous en avez deux », les règles restant affichées.
- **Observé** : tout le contenu du panneau s'efface — Obligatoires, Règle 1 à 5, Stages, Scolarité préparatoire, exigence linguistique, tout disparaît. Il ne reste qu'un encadré jaune :
  « ⚠ Les règles ne peuvent pas être comptées : Règle 1 : the selection counts 2 courses, above the max 1 — semantics await the director's ruling. Le reste du panneau reste utilisable. »
  Une phrase moitié française moitié anglaise, avec du vocabulaire que je ne comprends pas (« semantics await the director's ruling »), et la phrase « le reste du panneau reste utilisable » est fausse : il ne reste rien à utiliser. Je perds aussi l'accès à la case « Scolarité préparatoire déjà faite » et au choix de règle du formulaire de cours manuel. Seul « Annuler » m'a sortie de là.
  (captures : `shots/19-panneau-effondre.png`, `shots/19b-panneau-bas.png` ; erreur console : aucune)

---

### « Où le placer ? » casse la vérification et crache un message anglais du solveur

- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Proposer un organigramme (le panneau affiche « Placement vérifié ✓ (préalables, plafond, horaires) » puis « ⚠ mais 4 sections de règles restent à combler »).
  2. Ouvrir la Règle 1, cliquer « où le placer ? » sur MED-1100, puis la puce « + A5-A28 ».
- **Attendu** : le cours se place et la vérification se refait avec un nouveau verdict.
- **Observé** : le cours se place bien (le ruban montre MED-1100 en A5-A28, le compteur passe de 100 à 103 cr), mais :
  - le verdict de vérification **disparaît complètement** du panneau — plus une seule ligne entre « Proposer un organigramme » et « Obligatoires » ; j'ai attendu, rien n'est revenu ;
  - la bande d'état affiche « ⚠ Solveur : MED-1100 is passed or pinned but has no Course in the request ». Je ne comprends rien à cette phrase et elle n'est même pas en français.
  Le verdict n'est revenu qu'après avoir **retiré** MED-1100. Le message anglais, lui, est resté affiché longtemps après le retrait du cours (message zombie qu'il faut fermer soi-même).
  (capture : `shots/08-erreur-solveur-anglais.png` ; erreur console : aucune)

---

### Le verdict dit « Placement vérifié ✓ (… horaires) » alors qu'un conflit d'horaire est affiché à côté

- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Ouvrir la session H2-H27, cliquer le bloc « Mathématiques pour l'ingénierie I » (MAT-1900 - A).
  2. Cliquer la plage pointillée « MAT-1900 - B » pour la forcer → le lundi 8:30 devient hachuré, « ⚠ conflit d'horaire — plages en cause hachurées ».
  3. Regarder le verdict dans le panneau de gauche.
- **Attendu** : le verdict global signale le conflit.
- **Observé** : le panneau affiche « Placement vérifié ✓ (préalables, plafond, horaires) » **sur le même écran** que la grille hachurée en rouge. Le mot « horaires » est explicitement dans la parenthèse du ✓. Je ne sais plus à quoi me fier.
  Pire : si je change de session, **plus rien** ne me rappelle qu'il y a un conflit en H2 — la vignette H2-H27 du ruban n'a aucun marqueur (alors que le dépassement de crédits, lui, met bien un « ⚠ » sur la vignette). Je peux laisser un conflit non résolu sans le savoir.
  (capture : `shots/12-verdict-ok-malgre-conflit.png` ; erreur console : aucune)

---

### Un même cours compte pour deux règles à la fois

- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Ouvrir la Règle 1, choisir « Règle 2 » dans le menu « entente… » de GMN-2901.
  2. Cliquer le « + » de GMN-2901 pour le placer.
  3. Regarder les badges des règles.
- **Attendu** : GMN-2901 compte soit pour la Règle 1, soit pour la Règle 2 (c'est le sens d'une entente : le faire compter ailleurs).
- **Observé** : la Règle 1 affiche « ✓ GMN-2901 » **et** la Règle 2 passe à « 2/3 cr ». Les deux règles sont créditées du même cours de 2 crédits. Si je me fie à ça pour savoir ce qu'il me reste à faire, je vais manquer des cours à la diplomation.
  (capture : `shots/14-surcharge-double-comptage.png` ; erreur console : aucune)

---

### Un cours offert seulement à l'hiver s'ajoute à une session d'automne sans avertissement immédiat

- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Ouvrir la session A3-A27 (automne).
  2. Dans la Règle 1, cliquer le « + » de GMN-2902, dont la ligne dit pourtant « offert H ».
- **Attendu** : refus, ou au minimum un message immédiat et visible.
- **Observé** : le cours est ajouté sans broncher. La vignette A3-A27 passe à « 18 ⚠ », mais le ⚠ concerne le plafond de crédits, pas la saison. L'en-tête de la grille affiche même « combinaison automatique - **sans conflit ✓** ».
  L'explication existe (« ⚠ GMN-2902 — pas offert à cette session : gardé dans la liste, rien n'est dessiné »), mais elle est **tout en bas de la grille**, sous le pli : il faut défiler jusqu'à 22:30 pour la trouver. Le panneau finit par dire « ⚠ GMN-2902 : aucune session de l'horizon ne peut l'accueillir », ce qui est clair — mais arrive après coup.
  Deuxième anomalie du même geste : la ligne de GMN-2902 affichait **déjà** « aucune session ne peut l'accueillir » avant mon clic, et le bouton « + » restait quand même actif. Si aucune session ne peut l'accueillir, pourquoi le bouton me laisse-t-il l'ajouter ?
  (capture : `shots/18-cours-hiver-en-automne.png` ; erreur console : aucune)

---

### Impossible de savoir quelle version du programme j'ai choisie, ni d'en changer

- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Sur l'écran d'accueil, deux entrées « Baccalauréat en génie des eaux » : « B-GEX - version A24 - 120 cr » et « B-GEX - version A26 - 120 cr ».
  2. Cliquer la version A26.
- **Attendu** : voir quelque part « B-GEX version A26 » après le choix, et pouvoir revenir en arrière si je me trompe.
- **Observé** : l'en-tête affiche seulement « Baccalauréat en génie des eaux » — sans code, sans version. Le nom n'est pas cliquable (curseur normal, pas d'infobulle) et il n'y a **aucun bouton pour changer de programme** : la liste des programmes disparaît définitivement. Si j'avais cliqué A24 par erreur, mon seul recours serait de marteler « Annuler ».
  Deuxième problème, en amont : rien n'explique ce qu'est une « version ». Je ne sais pas si je dois prendre celle de mon admission, la plus récente, ou autre chose. Il faudrait une phrase du genre « choisissez la version en vigueur à votre admission ».
  (captures : `shots/01-accueil.png`, `shots/02-programme-choisi.png`)

---

### « Le cheminement affiché brise une contrainte » — laquelle ? où ?

- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Placer un cours qui fait dépasser le plafond (par exemple le « + » de GMN-2901 dans H2-H27, qui passe à 17 cr).
  2. Lire le verdict.
- **Attendu** : « la session H2-H27 dépasse le plafond de 15 cr ».
- **Observé** : « ⚠ Le cheminement affiché brise une contrainte (préalable, plafond, été fermé ou conflit d'horaire). » Le message énumère les quatre causes possibles sans dire laquelle, ni dans quelle session. Ici je devinais parce que je venais de faire le geste ; sur un organigramme complet de huit sessions, je serais incapable de trouver le problème. (À son crédit, l'en-tête affiche bien « ⚠ plafond de 15 cr dépassé » — mais le verdict et l'en-tête ne se parlent pas.)
  (capture : `shots/14-surcharge-double-comptage.png`)

---

### « Où le placer ? » ne dit pas pourquoi la plupart des sessions sont exclues, et la puce de la session actuelle porte un « + »

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Cliquer « où le placer ? » sur CHM-1903 (obligatoire, déjà placé en A1-A26).
  2. Faire la même chose sur GEX-1000 (placé en H6-H29), puis sur PHI-3900 et GSC-1000.
- **Attendu** : comprendre pourquoi certaines sessions sont possibles et d'autres non.
- **Observé** : pour CHM-1903 et GEX-1000, une **seule** puce : « + A1-A26 » et « + H6-H29 » — exactement la session où le cours se trouve déjà. Donc « où le placer ? » me propose de le placer là où il est. Le « + » suggère un ajout, et rien ne dit « c'est là qu'il est ».
  Pour PHI-3900 et GSC-1000, plusieurs puces apparaissent — donc le mécanisme fonctionne, mais je n'ai aucun moyen de savoir si l'absence d'une session vient d'un préalable, du plafond ou d'un conflit d'horaire.
  Le bouton « où le placer ? » **disparaît** quand les puces s'affichent : si je change d'idée, je n'ai aucun moyen de refermer la liste.
  Enfin, deux étés (« + É27 », « + É28 ») sont proposés pour PHI-3900 alors que la case « Ouvrir les étés aux cours réguliers » est décochée. J'ai dit non aux étés et on me les propose quand même.
  (capture : `shots/07-ou-le-placer.png`)

---

### Le lien de partage est déversé en entier à l'écran

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Cliquer « Partager » en haut à droite.
- **Attendu** : « lien copié » et c'est tout.
- **Observé** : la bande d'état affiche le message **plus l'URL complète**, environ 640 caractères qui prennent six lignes pleine largeur et repoussent l'horaire vers le bas. Ce n'est pas lisible, ce n'est pas utile (le lien est déjà dans le presse-papier), et il faut penser à fermer le message pour retrouver de la place.
  À noter aussi : la barre d'adresse du navigateur ne change pas (elle reste `http://localhost:8000/`), donc si je ferme le message sans avoir collé le lien quelque part, il est perdu.
  **Le partage lui-même fonctionne parfaitement** : ouvert dans un autre navigateur, le lien restitue les huit sessions au cours près, les crédits, la surcharge « 20 ⚠ » de H2-H27, mon cours manuel GEX-9999 avec son horaire, mes ententes de règles et les badges (Règle 1 ✓ GMN-2901, Règle 2 2/3 cr, Règle 3 ✓ GEX-9999). Le message d'accueil « ✓ Organigramme partagé importé — « Annuler » restaure le vôtre » est le meilleur message de toute l'application.
  (capture : `shots/17-partager.png`)

---

### « Chercher plus longtemps » ne semble rien faire, et son message est du jargon

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Cocher « Ouvrir les étés aux cours réguliers », cliquer « Proposer un organigramme ».
  2. Un bouton « Chercher plus longtemps » apparaît à côté ; le cliquer.
- **Attendu** : une nouvelle proposition, ou au moins une indication d'attente.
- **Observé** : aucun indicateur pendant le calcul, aucun changement dans le ruban après, et le même message qu'avant : « ⚠ Recherche interrompue (budget de calcul atteint) : résultat partiel — l'absence de solution n'est pas prouvée. »
  « Budget de calcul », « l'absence de solution n'est pas prouvée » — ce sont des mots d'informaticien. Comme étudiante, ce que je veux savoir c'est : est-ce que ça vaut la peine de réessayer, ou est-ce que mon cheminement est impossible ?
  (capture : `shots/20-chercher-plus-longtemps.png`)

---

### La grille perd ses en-têtes de jours quand on défile, et la légende est cachée sous le pli

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Ouvrir une session, faire défiler la grille horaire vers le bas.
- **Attendu** : la ligne « Lundi / Mardi / … » reste visible.
- **Observé** : elle disparaît. Comme l'axe va toujours de 8:30 à 22:30 alors que mes cours finissent vers 20:30, je passe mon temps à défiler dans du vide et je ne sais plus quelle colonne est quel jour.
  Même problème pour la légende « Plein = plage retenue - pointillé = autre plage possible (cliquer pour la forcer) - hachuré = conflit » : elle est **sous** la grille, donc invisible tant qu'on n'a pas défilé jusqu'en bas. Or c'est elle qui m'apprend qu'un cours est cliquable — je ne l'ai découverte qu'en cherchant dans le code de la page, pas à l'écran.
  (capture : `shots/04-horaire-bas.png`)

---

### Textes tronqués dans les blocs courts et dans le panneau

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Ouvrir A3-A27 : mercredi 17:30, le bloc « Analyse numérique pour l'ingénierie ».
  2. Ouvrir le formulaire « Cours absent du catalogue ? ».
- **Observé** : dans les blocs d'une heure, le code du cours est coupé en deux par le bord (« MAT-2910 - S » à moitié visible). Quand deux blocs partagent la même heure (H2-H27 mercredi 18:30 avec une plage pointillée), le texte devient carrément illisible.
  Dans le formulaire de cours manuel, les champs sont trop étroits pour leurs libellés : « Code (GEX-12: », « NRC (optionn( », « 11:2( », « Compte dans « Règle▾ ». Je remplis à l'aveugle.
  (captures : `shots/18-cours-hiver-en-automne.png`, `shots/16-cours-manuel-cree.png`)

---

### Le cours manuel se place tout seul quelque part, et « Copier le JSON » / « Proposer au catalogue » ne me parlent pas

- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Ouvrir « Cours absent du catalogue ? », saisir GEX-9999 / « Projet spécial en eaux » / 3 cr / vendredi 13:30-16:20, choisir « Compte dans « Règle 3 » (entente) », cliquer « Créer le cours ».
- **Attendu** : savoir où le cours atterrit.
- **Observé** : ça marche très bien — le cours apparaît vendredi dans la grille, la Règle 3 passe à « ✓ GEX-9999 », le pied de page passe de 8834 à 8835 cours. Mais :
  - le formulaire n'annonce nulle part que le cours sera **placé dans la session actuellement affichée** ; ma session H2-H27 est passée de 17 à 20 crédits sans que je l'aie demandé ;
  - dans la grille, le cours est étiqueté « GEX-9999 - **M** », alors que le texte d'aide promettait « marqué « manuel » ». Un « M » tout seul, je ne devine pas ;
  - deux boutons apparaissent ensuite : « Copier le JSON » (je ne sais pas ce qu'est un JSON ni quoi en faire) et « Proposer au catalogue », qui mène à `github.com/…/issues/new` sans prévenir. Il faudrait au moins dire « cela ouvre une page GitHub, un compte est requis ».
  (capture : `shots/16-cours-manuel-cree.png`)

---

### Petites choses relevées en passant

- **Gravité** : mineur — **Type** : friction
- Le pied de page dit « données : **date de récolte inconnue** ». Comme je planifie une vraie session, je veux savoir si l'horaire vient de cette année ou d'il y a deux ans.
- Les règles s'appellent « Règle 1 », « Règle 2 »… sans intitulé. En ouvrant la Règle 1 je découvre trois cours de santé et sécurité — j'aurais aimé le savoir avant de cliquer.
- Les badges des règles ne sont pas du même format : « 0/3 cr », puis « ✓ GEX-9999 » (le compteur de crédits disparaît quand c'est comblé), puis « — » pour la Règle 5 et la Scolarité préparatoire. Le verdict dit « 4 sections de règles restent à combler » alors que j'en compte cinq incomplètes à l'écran (la Règle 5 affiche « — » et n'apparaît dans aucun menu « entente… » — je ne sais pas si elle me concerne).
- Le badge de la Scolarité préparatoire reste « — » même quand je décoche « déjà faite » et que 8 cours deviennent requis ; le message « 8 cours sans session » ne dit pas lesquels et n'est pas cliquable. (Le mécanisme lui-même fonctionne bien : décocher ajoute 8 cours, recocher les retire exactement.)
- Les sections d'un cours qui ne rentre pas affichent « conflit d'horaire » dans la ligne du panneau — j'ai d'abord cru qu'un conflit existait déjà, alors que ça veut dire « il en créerait un si vous l'ajoutiez ici ».
- Les cours dont le préalable manque sont grisés avec « préalables non remplis », sans dire **lequel** manque, et sans aucun bouton (ni « + », ni « où le placer ? »). Je ne peux ni comprendre ni forcer.
- Forcer une section (MAT-1900 en B puis en A) laisse l'en-tête afficher « combinaison **automatique** », alors que j'ai choisi manuellement. Rien n'indique quelles sections sont figées ni comment revenir à l'automatique.
- Les sections du panneau se referment toutes seules quand j'en ouvre une autre (accordéon) : j'ai perdu la Règle 1 ouverte en dépliant « Obligatoires ».
- La session A1-A26 contenait 5 cours (dont GSC-1000) mais la grille n'en dessinait que 4, sans le dire ; l'en-tête affichait quand même « sans conflit ✓ ».
- Plusieurs clics sur les entêtes de sections n'ont pris effet qu'au deuxième essai. Je ne saurais dire si c'est l'application ou l'automatisation que j'utilisais — je le signale sans le compter comme bogue.

---

### Ce qui marche bien (pour être juste)

- Le premier écran est clair et instantané ; la liste des programmes est immédiatement compréhensible.
- « Proposer un organigramme » répond en moins d'une seconde et produit un cheminement plausible : 13/15/9/15/15/12/6/12/12 crédits, préalables dans le bon ordre, le stage GEX-1580 placé à l'été É27, 34/34 obligatoires couverts.
- La vérification automatique (sans bouton) est un vrai gain : elle se relance seule et le message « … la vérification se relancera d'elle-même » me rassure.
- L'axe va bien de 8:30 à 22:30 en tout temps, la page elle-même ne défile jamais (seuls le panneau et la grille), les libellés du ruban sont bien « A1-A26 » et le titre bien « Horaire — A1 — Automne 2026 ».
- **Le signalement du conflit d'horaire dans la grille est excellent** : bandeau rouge, « Lundi ⚠ » dans l'en-tête de colonne, et les deux blocs hachurés avec « ⚠ conflit » écrit dessus. Et il disparaît dès que je remets la bonne section.
- Forcer une plage pointillée est fiable : **aucun autre cours n'a bougé** (j'ai comparé les positions de tous les blocs avant et après ; IFT-1903, GGL-2600 et STT-1900 gardent exactement le même créneau et la même section, seul MAT-1900 change).
- La persistance est parfaite : après rechargement, tout est là — cours, session affichée, cours manuel, ententes, section forcée, surcharge.
- Le partage rouvre l'organigramme entier tel quel dans un autre navigateur.
- Fermer un message de la bande d'état en cliquant n'importe où dessus fonctionne (testé, le message disparaît).

---

## Tests prévus que je n'ai pas pu faire

- **Marquer un cours comme réussi puis annuler** : je n'ai trouvé aucune commande « réussi » dans l'interface — ni sur les lignes du panneau (qui n'ont que « entente… », « + » / « ✕ » et « où le placer ? »), ni sur les blocs de la grille, ni ailleurs. Soit ça n'existe pas, soit c'est trop bien caché pour que je le trouve.
- **Épingler / dépingler explicitement un cours** : même chose, je n'ai vu aucun bouton « épingler ». Les messages parlent pourtant d'« épinglages » (« offre et épinglages ne laissent rien ») — un mot que je ne comprends pas et dont je ne trouve pas la commande correspondante.
- **Préférences de classement** (journées compactes, matins libres) : absentes de l'interface. D'après le plan, c'est le jalon 10 — *pas encore construit*, je ne le compte pas comme un défaut.
- Je n'ai pas testé le cron CI ni la fraîcheur réelle des données (jalon 5, hors de ma portée d'utilisatrice).

---

## Impression générale

L'ossature est là et elle est bonne : en trois clics j'obtiens un cheminement complet sur huit sessions, avec les préalables dans le bon ordre et le stage à l'été, et la grille hebdomadaire est belle, lisible et honnête sur les conflits. Le partage par lien et la persistance sont irréprochables. Pour **regarder** un cheminement, je m'en servirais dès aujourd'hui.

Pour **le modifier**, non — pas encore, et c'est un vrai « non ». Dès que je touche à quelque chose, l'application se met à me mentir ou à se taire. Elle m'a affiché « Placement vérifié ✓ (préalables, plafond, horaires) » pendant qu'un conflit d'horaire hachuré était affiché juste à côté. Elle a fait disparaître son verdict sans un mot parce que j'avais placé un cours avec son propre bouton. Elle a crédité le même cours à deux règles. Et elle a effacé tout le panneau des règles — ma seule vue de ce qu'il me reste à faire pour diplômer — pour m'annoncer, en anglais, que « semantics await the director's ruling ».

Ce que je retiens : quand l'outil est d'accord avec lui-même, il est excellent ; quand il ne l'est pas, il ne me le dit pas dans ma langue et il me laisse seule. Or je vais consulter ce plan pour choisir mes cours d'automne, avec des dates d'inscription réelles. Trois choses me feraient basculer : (1) que le verdict global tienne compte des conflits d'horaire et me dise **quelle session** pose problème ; (2) qu'aucun message technique en anglais n'atteigne jamais l'écran, et que le panneau des règles ne s'efface jamais, quoi qu'il arrive ; (3) qu'on voie en tout temps quelle version du programme (A24 ou A26) est chargée, avec un moyen d'en changer.
