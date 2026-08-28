# Rapport d'exploration — directeur du B-GCI (Bernard)

Date : 2026-08-27.
Session agent-browser : `directeur-gci`.
Programme testé : Baccalauréat en génie civil (B-GCI, version A26), les quatre concentrations, départs automne et hiver.
Captures dans `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/07141d69-919b-4129-8679-333a9b9566ad/scratchpad/shots/`.

---

## Constats

### Le solveur ne suit pas « le cheminement actuel du plus près », contrairement à ce qu'il affiche
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Charger B-GCI A26, sans concentration, départ A26, rien de réussi (cheminement de référence : A1=15cr, H2=15cr, É27=9cr, A3=15cr, H4=16cr, A5=15cr, H6=6cr, A7=9cr, H8=6cr).
  2. Dans « Cours obligatoires », créditer GCI-1000, GCI-1010, GCI-1011, GLG-1000 (façon de simuler « déjà réussis » — voir constat sur l'absence de marquage dédié).
  3. Geler GCI-1001 (Statique, seul préalable réel de GCI-2001 : `GCI-2000 OU GMC-2001`, donc **aucun lien** avec GCI-1001) en A3-A27, pour simuler un échec de Statique en A1 et sa reprise l'automne suivant.
  4. Comparer le cheminement recalculé à la référence.
- **Attendu** : le bandeau d'information affiché en permanence dit explicitement « D'autres agencements équivalents existent ; celui proposé suit votre cheminement actuel du plus près. » Je m'attendais donc à un déplacement localisé : GCI-1001 en A3, et seulement les cours qui en dépendent réellement décalés en conséquence.
- **Observé** : la quasi-totalité du cheminement est redistribuée. GCI-2001 (dont le préalable est confirmé, dans son propre panneau, être `GCI-2000 OU GMC-2001` — sans aucune mention de GCI-1001) est déplacé de A3-A27 vers A5-A28 sans nécessité de préalable ni de plafond de crédits (A3 n'était chargée qu'à 12/17cr après le déplacement de GCI-1001, il restait de la place). GCI-1007 (dont le seul préalable, `GGL-2600 OU GLG-1900 OU GLG-1000`, est satisfait dès le départ par le crédit de GLG-1000) est rapatrié en A1, alors qu'il était en A3 dans la référence. PHI-2910, GCI-2004, GCI-2006, GCI-2007 changent aussi de session. Le total de crédits par session passe de 15/15/9/15/16/15/6/9/6 à 3/12/9/12/13/15/12/12/6 — une redistribution large pour un seul cours retardé.
  J'ai reproduit exactement la même séquence une seconde fois (mêmes clics, dans le même ordre) et obtenu un résultat identique au chiffre près : le comportement est déterministe, pas un aléa isolé.
  Un second essai avec **deux** cours retardés simultanément (GCI-1000 et GCI-1001, tous deux automne seulement, reportés en A3) aggrave encore l'effet : A1 devient entièrement vide, et des cours sans aucun rapport (PHI-3900, GCI-2002, GCI-2011, GCI-2012, GCI-3000, GCI-3008…) changent tous de session jusqu'à H8.
  Capture : `shots/21-fail-1001-to-a3.png`, `shots/27-double-fail.png`. Aucune erreur console.
- **Pourquoi c'est grave pour moi** : mon travail consiste justement à dire à un étudiant « voici précisément ce qui change si vous coulez ce cours ». Avec ce comportement, je ne peux pas répondre : je dois comparer session par session à la main, et une bonne partie de ce que je vois changer n'a aucune justification pédagogique (ni préalable, ni plafond) — impossible à défendre devant un comité de programme ou face à un étudiant qui demande pourquoi un cours sans lien avec son échec a bougé.
- **Contre-exemple qui confirme le diagnostic** : la même manipulation sur un cours *terminal* (GCI-3333, le projet intégrateur de H8, dont rien ne dépend) donne un résultat parfaitement localisé : en le gelant à une session H10 ajoutée à l'horizon, **toutes les autres sessions (A1 à A7) restent identiques au chiffre près** à la référence ; seules H8 (vidée) et H10 (nouvelle) changent. Voir `shots/25-terminal-fail.png`. Cela montre que le solveur *peut* produire un résultat minimal quand aucune collision de plafond ne force un choix global — le problème apparaît spécifiquement quand un cours retardé doit être « recasé » ailleurs dans une session déjà chargée.

### Aucun marquage dédié « cours réussi » — seul un contournement indirect existe
- **Gravité** : mineur
- **Type** : pas encore construit / friction
- **Reproduction** : chercher dans l'interface un moyen de dire « j'ai réussi ce cours ». Aucune case à cocher par cours, aucun mot « réussi » nulle part dans l'interface.
- **Attendu** : un geste direct pour marquer un cours comme déjà réussi en session 1, avant de simuler un échec sur un autre cours de la même session.
- **Observé** : le mécanisme disponible est « créditer » (bouton par cours dans le panneau), dont l'info-bulle dit « Créditer X : compté sans occuper de session » — sémantiquement plus proche d'une reconnaissance des acquis (RAC) que d'un « réussi en A1 ». Il produit néanmoins l'effet recherché (le cours sort de la grille, ses crédits restent comptés), donc j'ai pu m'en servir comme contournement. Pour simuler un échec, il faut ensuite geler manuellement le cours coulé sur une session future — l'outil ne propose aucune action « ce cours a été coulé » qui déclencherait ce replacement à ma place.
  Après lecture de `docs/conception/project_plan.md`, ce choix est documenté et volontaire (ADR `2026-08-retrait-de-la-notion-de-cours-reussi` : « l'interface n'a pas de marquage réussi distinct ; seul le mécanisme `passed` de `core` subsiste »). Le vrai chemin prévu pour l'historique académique est l'import Capsule (relevé de notes), que je n'ai pas testé ici faute de relevé disponible dans ce contexte d'exploration.
- Je le classe en friction et non en bogue puisque le choix est assumé, mais il reste réel pour mon usage manuel (sans relevé Capsule sous la main, par exemple pour un cas hypothétique en comité de programme) : je dois deviner qu'il faut combiner « créditer » + « geler dans une session future » pour simuler un échec, ce qui n'est pas du tout évident pour quelqu'un de moins familier avec l'outil que moi.

### Ajouter un seul cours électif redistribue une bonne partie du cheminement
- **Gravité** : majeur
- **Type** : bogue (même cause que le premier constat)
- **Reproduction** :
  1. B-GCI A26, concentration Structures et matériaux, départ A26, rien de réussi (référence identique à « sans concentration » : 15/15/9/15/16/15/6/9/6).
  2. Dans la règle de concentration (12 cr à choisir), cliquer « automatique » pour le premier cours de la liste (FOR-2020, 3 cr, offert à l'automne).
- **Attendu** : FOR-2020 s'ajoute quelque part avec un déplacement minimal (par exemple, un seul cours poussé d'une session pour respecter le plafond de 17 cr).
- **Observé** : A3-A27 (qui recevait FOR-2020) a dû perdre GCI-2001 pour respecter le plafond — logique — mais GCI-2001 a ensuite délogé GCI-3000 de A5-A28, qui a lui-même atterri en A7-A29 ; en parallèle, GCI-2004 et GCI-2007 ont migré de H4-H28 vers H6-H29 sans lien apparent avec FOR-2020. Cinq sessions sur huit changent pour l'ajout d'un seul cours de 3 crédits. Capture : `shots/11-after-pick.png` (comparer à `shots/10-before-pick.png`).
- Je regroupe ce constat sous le même bogue que le premier (non-minimalité du replacement), avec une preuve indépendante : ici, aucun échec n'est en cause, seulement l'ajout d'un cours au choix.

### Les règles à crédits libres (« Autres exigences », concentration) ne proposent aucun choix par défaut pour un programme type
- **Gravité** : mineur
- **Type** : friction (comportement volontaire et bien expliqué, mais gênant pour mon usage)
- **Reproduction** : charger B-GCI sans concentration, départ A26 — observer le bandeau « mais 4 sections de règles restent à combler ci-dessous — le bac n'est pas complet. »
- **Attendu** : rien de précis — mais pour produire un « programme type » officiel à distribuer aux étudiants, j'aurais besoin qu'un choix représentatif soit déjà fait pour chaque règle à crédits libres (langue, complémentaires, concentration), sans quoi le document que je publierais serait toujours « incomplet » selon l'outil.
- **Observé** : chaque liste affiche clairement « Choisissez X crédits de cours dans cette liste — rien n'est pris automatiquement », ce qui est honnête et bien expliqué (pas un bogue), mais cela signifie que je dois faire moi-même un choix arbitraire pour chacune des quatre concentrations avant de pouvoir comparer des cheminements complets côte à côte — et chaque choix risque de redéclencher le bogue de redistribution ci-dessus.

### Département hiver : réordonnancement correct, mais dernière session très légère
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : B-GCI sans concentration, Début = H27, Sessions = 8.
- **Attendu** : un cheminement hiver plausible en 8 sessions, avec une charge raisonnablement répartie.
- **Observé** : le réordonnancement est correct et intelligent (les cours automne-seulement sont bien reportés à leur premier automne disponible, pas un simple décalage d'étiquettes — vérifié par comparaison de contenu des sessions, voir `shots/14-hiver.png`), et la durée totale (8 sessions, 106 crédits hors stage) est identique à la cohorte d'automne — bon point. Mais la dernière session (A8-A30) ne contient qu'un seul cours de 3 crédits (GCI-2012), une charge très déséquilibrée pour finir le bac. Cela peut être une contrainte réelle de préalables plutôt qu'un artefact, mais rien dans l'interface ne me permet de le vérifier facilement.

### Délai de rendu perceptible sur les accordéons du panneau de règles
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : cliquer un en-tête d'accordéon (« Cours obligatoires », « Autres exigences – Règle 1 », etc.) puis lire immédiatement l'état.
- **Attendu** : ouverture instantanée et visible au clic.
- **Observé** : à plusieurs reprises, l'état `aria-expanded` du bouton restait à `false` juste après le clic puis passait à `true` un instant après, sans nouvelle action. Je n'ai pas de preuve que cela cause un vrai clignotement visible à l'œil humain (mes mesures viennent de lectures DOM automatisées, pas d'un chronométrage visuel), donc je le note avec prudence plutôt que comme un bogue confirmé.

### Le plafond de crédits (17) n'est expliqué nulle part dans l'interface
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : observer le champ « Plafond (cr) » dans le panneau de gauche, valeur par défaut 17.
- **Attendu** : une indication si 17 est une contrainte dure de l'université ou une simple valeur cible ajustable.
- **Observé** : aucune info-bulle ni texte n'explique la provenance de cette valeur. Le comportement observé est celui d'une contrainte dure (jamais dépassée dans tous mes essais). Ceci correspond à une question ouverte du plan de projet (« Plafond de crédits par session : dur ou cible molle — à confirmer avec le directeur ») : en tant que directeur, c'est précisément la question qu'on me pose et à laquelle l'outil ne me permet pas de répondre depuis l'écran lui-même.

### Non testé — à mentionner explicitement
- Le partage par URL (bouton « Partager ») n'a pas été vérifié de bout en bout (ouverture du lien dans une session tierce).
- Le tiroir « Charger depuis Capsule » n'a pas été ouvert (aucun relevé de notes disponible dans ce contexte).
- Les boutons « Annuler »/« Rétablir » (undo/redo) n'ont pas été testés explicitement, au-delà de vérifier leur présence.
- Je n'ai pas testé l'allongement du cheminement (spinbutton « Sessions ») pour un scénario d'échec *situé au milieu* du cheminement (seulement pour le cours terminal GCI-3333) ; le comportement pour un échec plus précoce nécessitant un rallongement n'a pas été vérifié séparément du bogue de redistribution déjà documenté.

---

## Points positifs observés (pour équilibrer le rapport)

- Le cheminement de référence (sans concentration, A26) place correctement les 32 cours obligatoires, respecte les préalables visiblement (chaînes MAT-1900→MAT-1910, etc.), et le total de crédits (`97/120 cr au bac` + `9 cr de stage en sus`) est cohérent avec la règle « Stages » explicitée (`crédits en sus`).
- Changer de concentration sans avoir rien choisi ne laisse aucun résidu : les sessions restent identiques ; un choix devenu invalide après changement de concentration (ex. GGL-2600 propre à Eau et environnement) est proprement retiré de la grille, sans apparaître comme orphelin dans « Hors programme ».
- Le départ hiver recalcule un cheminement réellement différent (pas un décalage naïf d'étiquettes de session), respectant les saisons d'offre.
- La persistance `localStorage` fonctionne : au rechargement, programme, concentration, début, sessions et cours crédités/gelés sont tous restaurés à l'identique.
- La zone « Stages » explique clairement en prose la règle (1 stage obligatoire + 3 optionnels en sus des crédits du programme), avec un badge cohérent.

---

## Impression générale

Je ne confierais pas encore à cet outil la production de mes programmes types officiels ni les réponses que je donne à un étudiant en échec, pour une raison précise : le solveur ne respecte pas sa propre promesse affichée à l'écran (« suit votre cheminement actuel du plus près »).
Dans trois scénarios indépendants et reproductibles — un cours retardé, deux cours retardés simultanément, un simple ajout d'électif — j'ai vu des cours sans aucun lien de préalable avec la cause du changement se déplacer d'une session à l'autre, parfois sur la moitié du cheminement.
Or c'est exactement le scénario que je rejoue en comité de programme ou face à un étudiant : « si je coule ce cours, qu'est-ce que ça change ? ».
Aujourd'hui, la réponse honnête est « je ne sais pas sans comparer les huit sessions une par une », ce qui annule une bonne partie de la valeur de l'outil pour cet usage précis.

À l'inverse, le comportement est solide sur plusieurs autres points : cheminement de référence défendable, réordonnancement hiver correct, absence de résidu au changement de concentration, persistance fiable, et — preuve à l'appui (le cas du cours terminal GCI-3333) — le solveur *sait* produire un résultat minimal quand rien ne l'oblige à recomposer une session entière. Le problème est donc ciblé : la logique de « recasage » quand un cours retardé entre en collision avec le plafond d'une session choisit une redistribution globale plutôt que le déplacement le plus proche parmi les solutions équivalentes qu'elle affirme pourtant considérer.

Si ce point de stabilité était corrigé (ou si l'écart entre le message affiché et le comportement réel était au moins retiré du texte), je recommanderais l'outil sans hésiter pour la production des programmes types ; en l'état, je continuerai à produire mes cheminements de référence à la main et à n'utiliser l'outil que pour des vérifications ponctuelles où je peux me permettre de tout relire session par session.
