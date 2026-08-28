# Rapport d'exploration UX — persona étudiante (génie des eaux)

Explorée par « Camille », étudiante de 2e année au B-GEX, avec l'application servie sur `http://localhost:8000` (session `agent-browser --session etudiante-gex` isolée). Environ 90 actions navigateur.

## Constats

### L'annuler/rétablir ne redonne pas l'horaire vu précédemment — l'horizon se rebrasse de façon imprévisible
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Choisir B-GEX (placement automatique de tout l'horizon, stage d'été GEX-1580 en É27).
  2. Chercher « GLO- », épingler GLO-1901 (3 cr) à A1-A26 → A1 dépasse le plafond (19/17 cr), le reste de l'horizon ne bouge pas.
  3. Dépingler GLO-1901 (bouton « ✕ ») → l'horizon **entier** se replace différemment : le stage GEX-1580 saute de É27 à É30, PHI-2910/PHI-3900 changent de session, H4-H28 passe de 12 à 9 cr, etc.
  4. Cliquer une fois « ↶ Annuler » : A1 revient à 19 cr (GLO-1901 réépinglé) mais le **reste de l'horizon garde le brassage de l'étape 3** au lieu de revenir à celui de l'étape 2.
  5. Cliquer « ↶ Annuler » une seconde fois : là seulement l'horizon retrouve l'arrangement d'origine.
- **Attendu** : un clic sur « Annuler » redonne exactement l'écran vu juste avant la dernière action (comme un vrai undo/redo), y compris l'agencement complet de l'horizon.
- **Observé** : l'historique ne semble pas mémoriser l'horaire résolu à chaque étape ; il rejoue les épinglages et laisse le solveur recalculer, qui retombe sur un agencement différent (mais valide) à chaque recalcul. Un « Rétablir » ne redonne pas non plus l'état exact vu avant l'« Annuler ». Capture : `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/07141d69-919b-4129-8679-333a9b9566ad/scratchpad/30-undo-inconsistent.png`. Aucune erreur console.
- Pour une étudiante réelle, ça veut dire : annuler un ajout de cours électif peut faire bouger son stage d'un été à l'autre sans avertissement, et « annuler » ne garantit pas de retrouver le plan qu'on vient de voir.

### Le compteur de crédits total au bandeau devient faux après un retrait de cours, corrigé seulement par un rechargement
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Avec B-GEX chargé (102/120 cr au bac affiché), chercher « GLO-2005 », cliquer « automatique » (ajouté malgré préalables non remplis, avertissement affiché) → 105/120 cr au bac.
  2. Retirer GLO-2005 avec le bouton « ✕ ».
  3. Le bandeau affiche **99/120 cr au bac** alors que la somme des crédits affichés session par session ne correspond pas à une perte de 6 cr.
  4. Recharger la page (`reload`) : le bandeau affiche alors **102/120 cr au bac**, la valeur correcte, sans qu'aucune session n'ait changé de contenu entre-temps.
- **Attendu** : le total affiché reflète toujours l'état réellement affiché, sans avoir besoin de recharger la page.
- **Observé** : après un retrait de cours, le total « cr au bac » reste incorrect (99 au lieu de 102) jusqu'au rechargement. Capture avant reload : `.../scratchpad/33-before-reload.png` (99/120 visible en haut à droite avant le `reload`, comparée au 102/120 après). Aucune erreur console.

### Les horaires alternatifs (⇄N) s'affichent illisibles quand on clique pour les voir
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Dans la grille d'A1-A26, cliquer sur le badge « ⇄ 4 » de Mathématiques pour l'ingénierie I (MAT-1900), le mercredi 8h30.
  2. Les 4 sections possibles (A, B, C, Z3) s'affichent comme des colonnes très étroites, superposées dans le même créneau.
- **Attendu** : pouvoir distinguer les sections proposées (numéro, local ou modalité) pour choisir en connaissance de cause.
- **Observé** : le texte est tronqué à une ou deux lettres par ligne (ex. « M / p / l / I »), illisible visuellement ; seul l'arbre d'accessibilité révèle les vrais libellés (« MAT-1900 - B », « - C », « - Z3 - à distance »). Capture : `.../scratchpad/13-crop.png`. Aucune erreur console — les données sont là, seul le rendu casse.

### Les bandeaux d'avertissement flottants cachent la grille horaire et reviennent après chaque action
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Choisir B-GEX : deux bandeaux jaunes (« D'autres agencements équivalents existent… » et « Le cheminement présume ces acquis… ») apparaissent superposés au coin supérieur droit de la grille horaire, cachant plusieurs cases (ex. mardi/mercredi 8h30-11h30).
  2. Les fermer avec leur « ✕ ».
  3. Ajouter un cours ou changer de session : les mêmes bandeaux réapparaissent, à nouveau par-dessus la grille.
- **Attendu** : soit les bandeaux ne recouvrent pas les cases de cours, soit une fois fermés ils ne reviennent pas pour la même information.
- **Observé** : chevauchement visuel confirmé sur capture `.../scratchpad/01-apres-choix-bgex.png` et `.../scratchpad/15-conflit-detail.png` ; réapparition après dismissal confirmée à plusieurs reprises pendant l'exploration.

### Pas trouvé de façon de marquer un cours « réussi » sans passer par l'import Capsule
- **Gravité** : mineur
- **Type** : friction (peut-être « pas encore construit », voir plus bas)
- **Reproduction** : cherché un bouton/case à cocher « réussi » sur les cartes de cours du panneau « Choix des cours » (obligatoires, règles, recherche libre) : chaque cours n'offre que « créditer » (rattacher à une règle par entente), « automatique » et des boutons de session à épingler. Le seul mécanisme documenté dans l'app pour marquer des cours comme passés/en cours est le tiroir « Charger depuis Capsule », qui demande de coller un relevé de notes réel.
- **Attendu** : pouvoir dire rapidement « ce cours-là, je l'ai déjà réussi » sans avoir de relevé Capsule sous la main (par exemple en rédigeant ce rapport sans compte réel).
- **Observé** : aucun contrôle de ce type visible ; capture du tiroir Capsule : `.../scratchpad/23-capsule-drawer.png`. Je n'ai pas pu tester le marquage réussi/annulé demandé à l'étape 5 faute de relevé Capsule réel — test non complété plutôt que jugé absent avec certitude.

### Petit accroc visuel : les boutons d'export chevauchent le texte après avoir coché « Ouvrir les étés »
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : cocher « Ouvrir les étés aux cours réguliers », regarder l'entête de la grille horaire.
- **Attendu** : mise en page stable.
- **Observé** : le bouton « Exporter l'horaire » se replace sur une ligne à part, chevauchant temporairement le texte « combinaison automatique... ». Capture : `.../scratchpad/31-ete-ouvert.png`.

### Le panneau « Choix des cours » est court et tout se passe hors-champ
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : dérouler une règle du programme (ex. Règle 1) ou une fiche de cours trouvé par recherche.
- **Attendu** : voir le contenu déplié apparaître à l'écran, ou au moins un indice qu'il faut défiler.
- **Observé** : le panneau ne fait qu'environ 300 px de haut ; dérouler une règle n'amène aucun défilement automatique, le nouveau contenu (liste de cours admissibles, boutons de session) reste invisible sous la ligne de flottaison tant qu'on ne défile pas soi-même le panneau (distinct du défilement de la page). Plusieurs fois pendant l'exploration, cliquer un bouton de placement resté hors-champ n'a eu aucun effet apparent tant que je ne l'ai pas fait défiler dans la vue avant de cliquer.

### Le total de crédits affiché ne correspond pas visiblement à la somme des sessions, sans explication
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : additionner à la main les crédits par session affichés dans la barre de navigation (ex. 16+17+15+9+9+9+12+15+9 = 111) et comparer au « 102/120 cr au bac » affiché en haut.
- **Attendu** : soit les chiffres concordent, soit une info-bulle explique l'écart (ex. crédits de stage « en sus », cours hors-programme non comptés).
- **Observé** : aucune explication visible à l'écran ; l'écart semble cohérent avec la règle documentée « crédits de stage en sus du total », mais rien à l'écran ne le dit à l'étudiante.

### Classement des horaires par préférences (journées compactes, matins libres, pause dîner)
- **Gravité** : —
- **Type** : pas encore construit (jalon 10)
- Cherché une option de ce type (à côté de « Profil » ou dans les paramètres) : rien trouvé, seul un choix « Aucun / Profil international » (concentration, pas une préférence horaire) est présent. Cohérent avec le jalon 10 du plan.

## Constats positifs observés (pour contexte)

- Le placement automatique de tout l'horizon (A1 à H8 + étés) se fait dès la sélection du programme, avec crédits par session raisonnables et cours obligatoires correctement enchaînés.
- Les messages de préalables non remplis, de dépassement de plafond et de conflit d'horaire sont en français clair et actionnable (« Dépinglez-le ou corrigez ce qui bloque »).
- Le conflit d'horaire créé volontairement (forcer MAT-1900 section B, qui chevauche Chimie des eaux le vendredi) a été signalé clairement (bandeau rouge, jour surligné, blocs hachurés) et résolu proprement en resélectionnant la section A.
- Le partage par URL (bouton « Partager ») fonctionne : lien copié et confirmé à l'écran.
- L'ajout manuel d'un cours absent du catalogue (formulaire code/titre/crédits/NRC/horaire) existe déjà et est accessible depuis le panneau, contrairement à ce que je croyais avant de le tester.
- La persistance après rechargement fonctionne pour la structure du plan (sessions, cours placés, case « Ouvrir les étés ») — seul le total de crédits affiché était temporairement faux avant le rechargement (voir plus haut).

## Impression générale

L'outil comprend déjà bien ce qu'une étudiante en génie des eaux doit faire : il place tout le cheminement automatiquement, explique ses choix, et signale clairement les problèmes (préalables manquants, plafond dépassé, conflits d'horaire) avec des messages en français que je comprends sans formation. Je m'en servirais pour un premier survol de mon cheminement.

Ce qui m'empêcherait de m'y fier pour planifier ma session réelle, aujourd'hui : le bouton Annuler ne redonne pas ce que je viens de voir (mon stage d'été a changé de place tout seul en dépingant un cours électif, et il a fallu deux « Annuler » pour vraiment revenir en arrière), et le total de crédits affiché peut rester faux tant que je n'ai pas rechargé la page — deux choses qui minent la confiance dans les chiffres que l'outil me montre. Une fois ces deux points réglés, et l'affichage des horaires alternatifs rendu lisible, je m'y fierais davantage.
