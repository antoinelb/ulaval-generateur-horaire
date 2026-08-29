# Rapport de contre-vérification UX — Élodie, finissante au cégep

**Date :** 2026-08-29
**Persona :** future étudiante en sciences de la nature, aucun cours universitaire réussi, hésite entre génie civil, génie mécanique et génie physique à l'Université Laval.
**Session de navigateur isolée :** `agent-browser --session etudiante-cegep`, `localStorage` vidé avant l'exploration.
**Contexte :** contre-vérification ciblée des correctifs apportés suite à mon rapport du 2026-08-27 (`docs/ux/rapport-etudiante-cegep-2026-08-27.md`), suivie d'une ronde de régression plus libre.

**Note environnementale (hors app) :** en cours d'exploration, le quota disque de mon répertoire scratchpad temporaire (`/tmp/claude-1000/.../scratchpad`) a été atteint à deux reprises (erreurs `EDQUOT` sur `agent-browser screenshot` et sur l'écriture de fichiers), à cause de captures accumulées de mes propres explorations précédentes sur cette même session. Le nettoyage du dossier a réglé le problème chaque fois ; ceci n'a rien à voir avec l'application testée et n'apparaît nulle part dans mes constats ci-dessous.

---

## Les 7 points à recontrôler

### 1. Bogue majeur du « Début » (cheminement daté, crédits fantômes, ✓) — CORRIGÉ
Choisir un programme frais (génie civil, sans historique) affiche maintenant :
- « Début » pré-rempli à **A26** (prochaine session admissible), plus jamais A24 ;
- aucune session marquée d'un « ✓ » de complétion avant toute action de ma part ;
- le premier écran vierge (avant même de choisir un programme) affiche bien « 0 cr cette session », sessions « à planifier », sans aucune trace de crédits déjà comptés.

Le total « 97/120 cr au bac » qui s'affiche dès le choix du programme n'est plus accompagné de « ✓ » : en le relisant plus attentivement cette fois, je comprends qu'il représente les crédits que l'auto-placement vient de répartir sur les 8 sessions du cheminement complet généré (pas des crédits déjà réussis) — ce qui est cohérent avec une nouvelle admise. Bogue confirmé corrigé.

### 2. Résidu de concentration (cours qui reste, recompté ailleurs) — CORRIGÉ
Reproduit deux fois, sur deux programmes différents :
- **Génie civil, Eau et environnement** : j'ai placé FOR-2020 (Évaluation environnementale) via « automatique » dans la Règle 1 de la concentration (100/120 cr). En remettant « Cheminement sans concentration », un message apparaît immédiatement : **« ⚠ Cours retirés avec l'ancien bloc : FOR-2020 — « Annuler » les restaure. »**, et le total redescend à 97/120. Cliquer le bouton « ↶ Annuler » du bandeau restaure bien la concentration « Eau et environnement » avec FOR-2020 replacé (100/120 cr).
- **Génie mécanique, Génie du développement durable** : même chose avec un cours auto-placé par le solveur lui-même (GMC-3351, jamais choisi à la main par moi) — en revenant à « sans concentration », le même message apparaît (« Cours retirés avec l'ancien bloc : GMC-3351 ») et « Annuler » restaure la concentration précédente. Le correctif couvre donc aussi bien mes choix manuels que les placements automatiques de la concentration.

Bogue confirmé corrigé, avec un message clair et une échappatoire (Annuler) qui fonctionne.

### 3. Sigle caché sous le bloc suivant (génie physique, Génie médical et biophotonique, mercredi fin de journée) — CORRIGÉ
Sur A1 — Automne 2026, mercredi 16h30, le bloc « Mécanique et relativité restreinte » affiche maintenant clairement « PHY-1003 - A » sur sa propre ligne, entièrement visible, sans chevauchement avec le bloc suivant (« Introduction à la programmation avec Python », 17h30). Vérifié à la fois dans l'arbre d'accessibilité (`Mécanique et relativité restreinte PHY-1003 - A`) et visuellement par capture d'écran.

### 4. Doublons dans les « présumés acquis » — CORRIGÉ
- Génie civil : « MAT-0130, MAT-0150, MAT-0260 » — plus de doublon.
- Génie physique (Génie médical et biophotonique) : « MAT-0130, MAT-0150, MAT-0260, PHY-0150 » — plus de doublon.

### 5. Vocabulaire non expliqué (concomitance, créditer vs automatique, entente) — AMÉLIORÉ
- **Concomitance** : une explication apparaît maintenant à côté de la case « Permettre un préalable en concomitance » : *« Concomitance : le préalable peut être suivi en même temps (même session) que le cours qui l'exige, plutôt qu'avant. »* — clair.
- **Créditer vs automatique** : le bouton « créditer » a maintenant une info-bulle (`title`) au survol : *« Créditer [cours] : compté sans occuper de session — contrairement à « automatique », qui le place dans une session. »* — répond exactement à ma confusion d'origine.
- **Entente** : le menu déroulant « Rattacher [cours] à une règle » a maintenant une info-bulle : *« Entente : compter [cours] dans une règle »*. C'est un ajout utile, mais le terme « entente avec la direction » lui-même (l'option affichée dans la liste) n'est toujours pas défini — je comprends *qu'il faut une entente*, mais pas *ce qu'est* une entente avec la direction dans ce contexte. Amélioration partielle.

### 6. Filtre pour la longue liste de cours d'anglais — CORRIGÉ
La règle « Autres exigences – Règle 1 » de génie civil (Intermediate English II, Advanced English I/II, Workplace English, etc.) affiche maintenant une boîte **« Filtrer cette règle (sigle ou titre)… »** juste au-dessus de la liste. Testé en tapant « Workplace » : la liste se réduit immédiatement à « Workplace English » seul. Fonctionne très bien et répond exactement à la friction rapportée.

### 7. Choix par défaut de concentration en génie physique, maintenant annoncé — CORRIGÉ
Au premier choix de B-GPH, un message apparaît clairement : **« ⚠ Concentration « Aéronautique et aérospatiale » sélectionnée par défaut — changez-la au besoin dans le panneau de gauche. »** Le même type de message existe pour génie civil/mécanique quand « Cheminement sans concentration » est retenu par défaut. Je ne suis plus prise par surprise.

---

## Constats additionnels (régression / nouveaux, du plus grave au plus bénin)

### Les boutons d'accordéon (Règles, « changer », « Choisir [programme] ») ne réagissent pas de façon fiable à un premier clic automatisé
- **Gravité** : mineur
- **Type** : friction (possiblement un artefact de mon outil d'automatisation plutôt que l'application elle-même — je le rapporte quand même par prudence)
- **Reproduction** :
  1. Cliquer sur un bouton de règle repliée (ex. « Règle 1 0/12 cr ▸ ») via un clic direct sur l'élément.
  2. Reprendre un instantané de la page.
- **Attendu** : le premier clic déplie la règle.
- **Observé** : à plusieurs reprises, un clic « normal » sur ces boutons (accordéons de règles, bouton « changer », bouton « Choisir B-GCI ») n'a produit aucun changement visible à l'écran ni dans l'arbre d'accessibilité, alors qu'un déclenchement direct de l'événement de clic sur le même élément fonctionnait instantanément. Je n'ai pas pu établir avec certitude si c'est un défaut de synchronisation de mon outil de test ou une vraie fragilité de la zone cliquable (ex. cible trop petite, ou clic intercepté par un élément superposé comme les bandeaux de messages). Je ne l'élève pas au rang de bogue confirmé, mais je note que si une utilisatrice humaine cliquait précisément sur le bord du bouton plutôt que son centre, un clic « manqué » sans aucun retour visuel serait déroutant (aucun curseur d'attente, aucun feedback).
- **Impact pour Élodie** : incertain — pourrait n'affecter que l'automatisation. À surveiller si des utilisatrices rapportent des clics « qui ne font rien ».

### Nouveau message utile non demandé, mais bienvenu : avertissement de dépendance entre un cours à option et un cours obligatoire
- **Gravité** : n/a (amélioration constatée)
- **Type** : amélioration
- **Reproduction** : choisir génie physique, concentration Génie médical et biophotonique.
- **Observé** : un message apparaît : « ⚠ GMC-2005 ajouté aux cours à option : un cours obligatoire l'exige comme préalable. » — utile et clair, dans la même veine que les autres avertissements bien rédigés du système.

### Le bouton « Réinitialiser » semble n'agir que sur le dernier programme actif, pas sur tous les programmes explorés
- **Gravité** : mineur
- **Type** : friction (comportement pas vérifié à fond, sous toutes réserves)
- **Reproduction** :
  1. Choisir génie physique, changer de concentration (Photonique, Aucune).
  2. Cliquer « changer » (retour à l'écran de choix de programme, sans avoir choisi de nouveau programme).
  3. Cliquer « Réinitialiser ». Un message confirme : « ✓ Ce programme a été réinitialisé — « Annuler » restaure votre organigramme. »
  4. Rechoisir génie civil.
- **Attendu** : je ne savais pas trop ce que « Réinitialiser » sans aucun programme actif à l'écran allait faire — le mot « Ce programme » dans le message suivant suggérait qu'un programme précis venait d'être visé.
- **Observé** : génie civil réapparaît intact (Eau et environnement, FOR-2020 toujours placé, 100/120 cr) — donc « Réinitialiser » n'a pas touché génie civil, seulement (je présume) génie physique qui était le dernier actif avant le retour à l'écran de choix. Le libellé du bouton ne précise pas sur quel programme il agit quand aucun n'est visiblement sélectionné à l'écran. Je n'ai pas creusé plus loin faute de budget d'actions ; ce n'était pas un des 7 points à revérifier.
- **Impact pour Élodie** : confusion possible sur la portée exacte de « Réinitialiser » — mais le comportement observé (ne pas tout effacer) est plutôt rassurant pour mon usage de comparaison entre programmes.

---

## Reste de la procédure habituelle (régression rapide)

- **Persistance inter-programmes** : confirmé à nouveau que choisir un autre programme puis revenir au premier restaure exactement son état (concentration, cours placés, crédits) — testé génie civil → génie mécanique → génie civil, et génie physique → génie civil → génie physique.
- **Rechargement de page en cours de comparaison** : après être passée par génie civil, génie mécanique, génie physique et être revenue sur génie mécanique (Génie du développement durable, 117/120 cr), un `reload` complet retombe exactement sur ce même programme/concentration/total — aucune perte.
- **Liste des 7 concentrations de génie physique** : c'est un simple menu déroulant natif (`<select>`), pas une longue liste qui déborde à l'écran — parfaitement navigable, aucune friction là-dessus.
- **Répéter un changement de concentration une deuxième fois** (étape 8 de ma procédure) : refait le cycle Robotique → Génie du développement durable → sans concentration sur génie mécanique, et Génie médical et biophotonique → Photonique → Aucune sur génie physique. Le comportement du message d'avertissement + Annuler est identique au premier essai — pas de dégradation au deuxième passage.
- **Console/erreurs JS** : aucune erreur applicative détectée pendant toute l'exploration. Seules 3 occurrences de la même erreur, sans lien avec l'app testée, concernant l'enregistrement du service worker (`SecurityError: ... sw.js ... does not have a MIME type`) — probablement un artefact du serveur de développement (`dx serve`), pas quelque chose qu'une utilisatrice verrait en production.
- Faute de budget d'actions restant après la contre-vérification ciblée, je n'ai pas retesté : Partager par URL, Exporter, Charger depuis Capsule, ajout de cours manuel, profils, cases « Ouvrir les étés »/« concomitance ». Rien n'indique que ces zones aient changé depuis mon dernier rapport.

---

## Impression générale

Les correctifs répondent directement aux problèmes que j'avais soulevés, et je les ai vérifiés un par un avec des reproductions concrètes : plus de cheminement daté dans le passé avec des crédits fantômes, un vrai message (et un vrai « Annuler » qui fonctionne) quand je change de concentration et perds un cours, un sigle de cours enfin lisible dans la grille, une liste d'acquis présumés sans doublons, un filtre qui marche pour la longue liste de cours d'anglais, et une annonce claire quand une concentration est choisie à ma place. Le vocabulaire s'est aussi amélioré (concomitance, créditer/automatique) même si « entente avec la direction » reste à moitié mystérieux.

Mon jugement d'ensemble du 2026-08-27 reste globalement valable pour ce qui n'a pas changé : l'outil décrit très bien chaque programme individuellement, mais rien ne m'aide encore à les **comparer** directement (pas de vue côte-à-côte, pas de mise en évidence des cours communs). Ceci dit, la confiance que j'ai dans les chiffres affichés est nettement meilleure maintenant : avant, je n'étais jamais sûre que le total de crédits reflétait vraiment « ce cheminement-là » après quelques essais de concentrations ; maintenant, chaque changement est annoncé et réversible, ce qui est exactement ce qu'il fallait pour que je puisse comparer sereinement sans avoir à tout recommencer à zéro à chaque doute.
