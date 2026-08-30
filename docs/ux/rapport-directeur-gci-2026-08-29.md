# Rapport d'exploration — Directeur du programme de baccalauréat en génie civil (B-GCI)

Date : 2026-08-29
Persona : Bernard, professeur et directeur de programme, B-GCI (Université Laval)
Session agent-browser : `directeur-gci`

## Portée de l'exploration

J'ai construit le cheminement type du B-GCI (version A26) sans concentration, à l'automne, puis testé la bascule
entre les trois concentrations (Eau et environnement, Géotechnique, Structures et matériaux), un départ à l'hiver,
et cinq scénarios d'échec de cours (préalable lourd en première session, cours terminal, cours à une seule saison,
deux échecs simultanés, rallongement de l'horizon). J'ai rechargé la page en cours de route et répété certaines
manipulations pour vérifier la stabilité du comportement.

Je n'ai **pas** testé : le partage par URL (« Partager »), les trois destinations d'« Exporter », le chargement
depuis Capsule ou depuis un fichier JSON, ni les autres programmes (B-GEX, B-GIN, B-GMC, B-GPH) — hors du mandat
d'un directeur de B-GCI. Ces zones ne sont donc couvertes par aucun constat ci-dessous.

---

## Constats

### Les avertissements s'empilent indéfiniment et finissent par couvrir l'horaire
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Choisir B-GCI, sans concentration, départ A26.
  2. Changer « Début » à H27 puis revenir à A26 (ou répéter l'aller-retour une seconde fois).
  3. Observer la zone de statut en bas de l'écran.
- **Attendu** : un message explique le changement en cours ; une fois la situation comprise ou dépassée par une
  nouvelle action, les anciens messages n'encombrent plus l'écran (ou au moins se distinguent clairement du
  message pertinent au moment présent).
- **Observé** : après deux ou trois changements de « Début », cinq à six avertissements quasi identiques
  (« … sont retirés du placement… ») s'empilent dans le coin, dont plusieurs redondants au mot près, sans jamais
  se nettoyer automatiquement ; un bouton « +N autres messages » apparaît et le bloc de texte recouvre une partie
  de la grille horaire. Rien ne les distingue comme périmés. J'ai dû les fermer un par un (bouton « ✕ ») pour
  retrouver un écran lisible. Reproduit deux fois de façon identique.
  Capture : `08-messages-empiles.png`, `07-etat-bizarre.png`. Erreur console : aucune.

### Un message d'erreur mêle de l'anglais technique brut dans une interface française
- **Gravité** : mineur
- **Type** : bogue
- **Reproduction** :
  1. Sur la concentration Eau et environnement, ajouter un cours (ex. FOR-2020) qui satisfait à la fois la
     Règle 1 (12 cr) et la Règle 2 (3 cr, « tous les cours de la Règle 1 du cheminement sans concentration ») de
     la concentration, sans lui assigner explicitement une règle via « Rattacher … à une règle ».
  2. Observer le message d'erreur affiché en bas de l'écran.
- **Attendu** : un message entièrement en français, cohérent avec le reste de l'interface (le standard du
  produit, y compris pour ses messages techniques).
- **Observé** : « ⚠ Le solveur n'a pas pu répondre — détail technique : Règle 1 (concentration scope) : the
  selection sums 15 credits, above the max 12 — semantics await the director's ruling ». Le message est
  compréhensible mais bascule sans prévenir en anglais interne (jargon de développeur : « scope », « semantics »,
  « the director's ruling »). Le problème lui-même — un cours éligible à deux règles à la fois nécessite un
  arbitrage — est légitime et bien détecté ; c'est la présentation qui trahit le code sous-jacent.
  Capture : `02-conflit-regle.png`. Erreur console : aucune.

### Aucune façon évidente de « faire échouer » un cours — il faut deviner la combinaison Geler + déplacement manuel
- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Charger B-GCI sans concentration, chercher comment marquer « GCI-1001 réussi en A1, mais pas GCI-1000 ».
  2. Chercher un bouton, une case à cocher ou un menu « réussi »/« échoué » sur un cours placé — aucun n'existe.
  3. Consulter l'aide « Format du fichier de cheminement attendu » : le seul champ apparenté est `completed`,
     documenté comme « les cours crédités à l'admission » (donc pour la reconnaissance des acquis, pas pour un
     échec en cours de programme).
- **Attendu** : une action directe et documentée pour ce cas d'usage extrêmement fréquent en tâche de directeur
  (« un étudiant coule un cours, que se passe-t-il ? »), par exemple un menu contextuel sur le cours dans la grille.
- **Observé** : la seule façon que j'ai trouvée, par essai-erreur, est : geler les sessions déjà « réussies »
  (bouton « Geler », qui empêche le solveur d'y toucher mais reste modifiable à la main), puis chercher le cours
  coulé dans la recherche du catalogue et le réassigner manuellement à une session future via ses boutons de
  session (il n'y a pas de bouton « retirer/échouer » direct pour un cours obligatoire — seulement pour les cours
  de règles à option, via un « ✕ »). Une fois compris, le mécanisme fonctionne bien (voir constats positifs
  ci-dessous), mais rien dans l'interface ne suggère cette procédure ; un directeur pressé ou une étudiante en
  détresse n'aurait aucune chance de la deviner sans expérimenter comme je l'ai fait. Aucune capture unique ne
  documente une « absence » ; voir la démarche dans les captures `03-conflit-resolu.png` à `04-programme-complet.png`.

### Les cours de concentration peuvent être placés avant les cours de base, sans égard à la séquence pédagogique habituelle
- **Gravité** : moyen
- **Type** : friction / limite du modèle
- **Reproduction** :
  1. Choisir B-GCI, concentration Eau et environnement, départ A26.
  2. Ajouter GGL-2600 (Hydrogéologie, cours de concentration) via « automatique ».
  3. Observer sa session de placement.
- **Attendu** : un cours de concentration typiquement enseigné en 3e ou 4e année se retrouve après l'essentiel du
  tronc commun, même en l'absence de préalable formel qui l'imposerait — c'est ainsi que je le présenterais à un
  comité de programme.
- **Observé** : GGL-2600 a été placé en **H2-H27**, la toute deuxième session du programme, avant même Statique
  ou Résistance des matériaux, simplement parce qu'aucun préalable formel ne l'en empêche et qu'il restait de la
  place sous le plafond de crédits. Le placement automatique suit une logique de faisabilité pure, pas de bon
  sens curriculaire implicite. Publier un tel cheminement comme « programme type officiel » sans le revoir à la
  main induirait en erreur un comité de programme. Capture : voir la grille finale `04-programme-complet.png`
  (H2-H27 contient GGL-2600).

### Construire un cheminement complet par concentration exige beaucoup de manipulation manuelle, sans être signalé comme un choix de conception avant de s'y engager
- **Gravité** : mineur
- **Type** : friction (comportement clairement annoncé une fois rencontré, mais coûteux)
- **Reproduction** :
  1. Choisir une concentration.
  2. Constater que « 97/120 cr » ou « 105/120 cr » restent placés (le tronc commun seulement) et que les sections
     de règles (« 0/12 cr », « 0/3 cr », etc.) sont vides.
  3. Dérouler chaque règle et cliquer « automatique » cours par cours (environ 6 à 9 clics par concentration) pour
     atteindre 120/120 cr.
- **Attendu** : je m'attendais à ce qu'un « programme type » propose déjà une combinaison représentative
  d'électifs par défaut, quitte à la modifier — c'est ce qu'un document de programme type publié montre
  normalement (des exemples de cours, pas des cases vides).
- **Observé** : le texte « rien n'est pris automatiquement » est bien présent et honnête, donc ce n'est pas un
  bogue, mais produire les quatre cheminements types (un par concentration) exige de refaire manuellement cette
  sélection à chaque fois — rien n'est mémorisé d'une concentration à l'autre au-delà du tronc commun et des
  choix de niveau « Programme ». Pour un usage répété (comme le mien), c'est un frein réel.

### Chevauchement de règles : un cours éligible à deux règles à la fois est difficile à réconcilier
- **Gravité** : moyen
- **Type** : friction
- **Reproduction** : voir le scénario du bogue « anglais technique » ci-dessus.
- **Attendu** : quand un cours (ex. FOR-2020) apparaît dans deux listes de règles à la fois, je m'attendais à ce
  que l'outil choisisse une règle par défaut (la moins généreuse, ou celle où il manque le plus), ou du moins à
  ce que le retrait du cours d'une des deux listes soit visible et univoque.
- **Observé** : il faut d'abord comprendre qu'un même cours apparaît deux fois dans l'arborescence (une fois sous
  chaque règle qui l'accepte), retirer l'exemplaire fautif via son propre bouton « ✕ » (et non via le menu
  « Rattacher … à une règle », qui déplace le comptage mais ne résout pas le double-compte tant que le cours
  reste listé ailleurs), puis rajouter un cours différent qui n'apparaît que dans une seule règle. La récupération
  a exigé plusieurs allers-retours de ma part avant de comprendre le mécanisme. Une fois résolu, l'état se
  stabilise correctement (`03-conflit-resolu.png`).

### Aucun indicateur de chargement pendant le recalcul du solveur après un changement de Début ou de concentration
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Charger B-GCI sans concentration, départ A26, cheminement complet (120/120 cr).
  2. Changer « Début » à H27.
  3. Prendre une capture d'écran immédiatement après le changement (avant que l'utilisateur ne clique ailleurs).
- **Attendu** : un signe visuel (spinner, texte « calcul en cours… », grisé temporaire) distinguant un état
  transitoire d'un état final.
- **Observé** : l'écran affiche transitoirement un total de crédits erroné (ex. « 30/120 cr » au lieu du 105/120
  final) et plusieurs sessions marquées « à planifier », sans aucun signe que ce n'est pas l'état définitif ; il
  faut attendre quelques centaines de millisecondes et rafraîchir la lecture pour voir l'état stable. Une
  utilisatrice qui regarde l'écran à ce moment précis croirait le cheminement brisé. Capture :
  `06-recalcul-transitoire.png` (état transitoire) à comparer avec `05-hiver-depart.png` (état stable, quelques
  instants plus tard, mêmes entrées).

### Aucune vue comparative entre les quatre concentrations
- **Gravité** : mineur
- **Type** : friction / pas encore construit
- **Reproduction** : basculer entre les concentrations dans le menu déroulant « Concentration ».
- **Attendu** : pouvoir comparer visuellement (côte à côte, ou par onglets conservés) les quatre cheminements
  types que je suis censé publier ensemble dans un même document.
- **Observé** : la bascule change proprement le contenu (aucun résidu de la concentration précédente n'est resté
  dans les sessions lors de mes essais — bon point, voir plus bas), mais un seul cheminement est visible à la
  fois ; pour comparer, il faut noter manuellement ou exporter chacun séparément. Étant donné que « Partager » et
  l'export ne sont pas dans le périmètre que j'ai testé, je ne peux pas confirmer s'il existe un contournement
  raisonnable (par ex. ouvrir plusieurs onglets avec des URL différentes) — je ne l'ai pas essayé.

---

## Constats positifs (à ne pas passer sous silence)

- **La chaîne de préalables est correctement propagée.** En gelant les sessions A1 à H4 et en déplaçant GCI-1001
  (Statique, cours à forte chaîne de dépendances) de A1 vers A3-A27 pour simuler un échec, le stage GCI-2580 —
  qui dépend indirectement de cette chaîne — a automatiquement glissé de É27 à É28, et l'ensemble du reste du
  cheminement s'est réajusté sans qu'aucun cours ne devienne « à planifier » : le programme est resté complet en
  8 sessions grâce à la marge sous le plafond de crédits. C'est exactement le raisonnement que je fais moi-même
  en comité de cas particuliers.
- **Un cours terminal (PHI-3900, dernière session, rien n'en dépend) a un impact minimal** quand on le déplace :
  seul son propre déplacement se produit, rien d'autre ne bouge.
- **Un cours à une seule saison (GCI-2006, offert seulement à l'hiver) coûte bien une année complète** une fois
  retiré de sa session : repoussé de H6-H29 à H8-H30 (aucune session d'automne ne lui est proposée), conforme à
  l'attente.
- **Deux échecs simultanés dans la même session** (GCI-2004 et GCI-2010, tous deux offerts seulement à l'hiver)
  ont été relocalisés ensemble à la session H suivante, dans le respect du plafond de crédits (15 cr sur 17),
  sans rien perdre du reste du programme.
- **Rallonger l'horizon fonctionne et le dit clairement.** Augmenter « Sessions » de 8 à 9 a ajouté une session
  A9-A30 exploitable immédiatement ; en essayant de revenir à 8, l'outil a refusé et expliqué pourquoi : « L'horizon
  reste à 9 sessions : PHI-3900 est épinglé en A9-A30 — dépinglez pour réduire davantage. » C'est précisément le
  type de message que je pourrais montrer telle quelle à une étudiante.
- **Un départ à l'hiver est vraiment recalculé, pas simplement décalé.** Les cours offerts uniquement à l'automne
  ont bien été redistribués dans le bon ordre (H1, A2, H3, É28-stage, A4…), avec un message explicite listant les
  cours « retirés du placement » par le changement de Début.
- **L'exigence linguistique et les préalables non vérifiables sont bien signalés** : « Exigence linguistique -
  ANL-2020 ou VEPT ≥ 53 ✓ » et un avertissement dédié pour le score TOEFL présumé acquis derrière ANL-2020 — utile
  pour rappeler à une étudiante de vérifier elle-même ce que l'outil ne peut pas valider.
- **L'état complet (programme, concentration, Début, sessions gelées, cours déplacés, 105/120 cr) a survécu à un
  rechargement complet de la page**, aussi bien avant qu'après les manipulations d'échec — testé deux fois.
- Aucune erreur JavaScript n'est apparue dans la console pendant toute la session, malgré des manipulations assez
  poussées (double échec, chevauchement de règles, changement répété de Début, rallongement de l'horizon).

---

## Impression générale

Le cœur du solveur m'inspire confiance : la propagation des préalables après un échec, le coût réel d'un cours à
une seule saison, la gestion honnête d'un rallongement d'horizon et la persistance de l'état sont exactement ce
que j'attends d'un outil que je montrerais à un comité de programme ou à une étudiante en échec. Je ferais
confiance au moteur de calcul.

Je ne confierais **pas encore** la production des programmes types officiels à cet outil tel quel, pour deux
raisons concrètes observées à l'écran : (1) rien ne garantit que le placement automatique respecte le bon sens
pédagogique implicite (un cours de concentration de fin de programme peut atterrir en deuxième session, faute de
préalable formel qui l'en empêche) — je devrais réviser chaque cheminement à la main avant publication ; et (2) la
façon de simuler un échec — le scénario que je répète le plus souvent avec les étudiants — n'est documentée nulle
part dans l'interface ; je l'ai reconstituée par essai-erreur (geler + déplacer manuellement), ce qui n'est pas
un exercice que je peux demander à une étudiante en détresse de refaire seule. Les messages qui s'empilent sans
jamais se nettoyer aggravent cette impression : au moment où je voudrais montrer un résultat propre en comité, la
zone de statut est encombrée de mises en garde périmées, certaines encore en partie en anglais technique.

Rien de ce que j'ai vu n'est un bogue qui invalide un résultat (aucune perte silencieuse de crédits, aucune
incohérence de calcul détectée), mais l'outil demande encore, dans son état actuel, un directeur qui sait déjà
la réponse pour vérifier que le cheminement produit est publiable tel quel.
