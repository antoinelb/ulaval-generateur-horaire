# Rapport d'exploration — Élodie, finissante en sciences de la nature

Comparaison des baccalauréats en génie civil (B-GCI), génie mécanique (B-GMC) et génie physique (B-GPH) à l'Université Laval, sans aucun cours réussi au préalable.

Date : 2026-08-29
Captures : `/home/antoine/.claude/jobs/1719c21c/tmp/*.png`

---

## Constats

### Cliquer « changer » donne l'impression trompeuse d'avoir tout perdu, sans aucun avertissement
- **Gravité** : majeur
- **Type** : friction (à la limite du bogue de perception)
- **Reproduction** :
  1. Choisir génie civil, laisser le cheminement se générer (97/120 cr, concentration Géotechnique par exemple).
  2. Cliquer sur « changer » en haut de l'écran.
  3. Observer : l'en-tête revient à « aucun programme choisi », les 8 sessions redeviennent « à planifier », « 0 cr cette session » s'affiche, et le bouton « ↶ Annuler Ctrl+Z » redevient grisé (désactivé).
  4. Re-choisir « Choisir B-GCI » dans la liste : le cheminement et la concentration (Géotechnique) reviennent exactement comme avant.
- **Attendu** : soit une confirmation avant de vider l'écran (« vous quittez génie civil, votre travail sera conservé/perdu »), soit au minimum un signe visuel que le travail est toujours là (le bouton Annuler ne devrait pas être désactivé s'il y a quelque chose à annuler).
- **Observé** : l'écran donne tous les signaux d'une perte totale et irréversible (page vidée, Annuler désactivé) alors que les données sont en fait conservées **par programme** en arrière-plan et réapparaissent si on re-choisit le même programme. Pour une étudiante indécise qui butine entre trois programmes — exactement l'usage visé par cet outil — ce comportement est anxiogène : rien à l'écran ne dit « votre génie civil est toujours là, allez voir génie mécanique tranquillement ». Capture : `11-apres-changer-perte.png`. Erreur console : aucune.

### Le triangle « ▸ » d'une règle ne se déplie pas de façon fiable au premier clic, et j'ai vu une fois la concentration se réinitialiser toute seule
- **Gravité** : majeur
- **Type** : bogue (comportement instable, pas isolé de façon certaine)
- **Reproduction** :
  1. Choisir génie civil, sélectionner la concentration « Eau et environnement ».
  2. Cliquer immédiatement (sans délai) sur le triangle « ▸ » de « Règle 1 » dans la section Concentration.
  3. Observer le sélecteur « Concentration » en haut du panneau.
- **Attendu** : la règle se déplie et montre la liste des cours ; le sélecteur de concentration ne bouge pas.
- **Observé** : une fois, le clic a silencieusement fait revenir le sélecteur de concentration à « Cheminement sans concentration » au lieu de déplier la règle — reproduit deux fois de suite en cliquant tout de suite après avoir changé de concentration. En ajoutant une pause d'environ 1,5 s avant de cliquer, le bug a disparu et la règle s'est dépliée normalement. Le même triangle a aussi souvent nécessité 2 ou 3 clics pour se déplier, sans retour visuel (ni chargement, ni erreur) entre les tentatives ratées. Je n'ai pas pu isoler avec certitude si c'est lié à un recalcul du solveur non signalé par un indicateur de chargement, mais l'absence totale de signe pendant ce délai (pas de spinner, pas de désactivation temporaire des boutons) est en soi trompeuse. Captures : `06-regle1-non-depliee.png`, `07-avant-clic-regle1-conc.png`, `08-apres-clic-regle1-conc.png`. Erreur console : aucune.

### Aucune confirmation visuelle après avoir cliqué « Partager »
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Choisir un programme. 2. Cliquer « Partager ». 3. Regarder l'écran.
- **Attendu** : un message du genre « lien copié ! » ou l'affichage du lien à copier.
- **Observé** : rien ne s'affiche à l'écran ; seule l'URL de la page change en arrière-plan (fragment `#…` encodant tout l'état). Sans ouvrir les outils de développeur, impossible de savoir si l'action a fonctionné. Capture : `14-apres-partager.png`.

### La concentration par défaut n'est pas la même selon le programme
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Choisir génie civil ou génie mécanique → concentration par défaut = « sans concentration ». 2. Choisir génie physique → concentration par défaut = « Aéronautique et aérospatiale » (la première de la liste), alors qu'une option « Aucune » existe aussi.
- **Attendu** : un comportement cohérent — soit toujours « sans concentration » par défaut, soit un signe clair indiquant qu'une concentration a été présélectionnée.
- **Observé** : en arrivant sur génie physique, le cheminement montré est déjà celui d'une concentration spécifique, sans que rien n'attire l'œil sur ce choix ; une étudiante pressée pourrait croire regarder le cheminement « de base » du programme.

### Vocabulaire non expliqué : « millésime »/version, « +9 cr en sus »
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : sur l'écran de choix de programme, chaque programme a un sélecteur de version (A26, H27…) ; une fois un programme choisi, l'en-tête affiche par exemple « 97/120 cr au bac (+9 cr en sus) ».
- **Attendu** : une explication (infobulle ou texte) de ce que représente la version du programme et pourquoi certains ont plusieurs choix de version (B-GIN, B-GMC) et d'autres un seul (B-GCI, B-GPH) — et ce que « en sus » signifie concrètement pour mes crédits totaux.
- **Observé** : aucun texte explicatif visible nulle part sur ces deux points ; il faut deviner ou connaître le jargon universitaire (« millésime », probablement lié à l'année d'admission).

### Comparer les concentrations demande de fouiller loin dans le panneau, le calendrier ne bouge presque jamais
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Choisir génie civil, essayer « sans concentration », « Eau et environnement », « Structures et matériaux » (idem pour les 7 concentrations de génie physique). 2. Regarder le calendrier des 8 sessions à chaque fois.
- **Attendu** : un moyen rapide de voir « en quoi cette concentration change mon parcours ».
- **Observé** : pour génie civil et pour toutes les concentrations de génie physique testées, le contenu des 8 sessions affichées reste identique à l'octet près d'une concentration à l'autre — seuls le nom en haut et les crédits requis tout en bas du panneau (« 0/15 cr », « 0/12 cr »…) changent, et il faut défiler loin dans le panneau de gauche et déplier chaque règle pour voir la liste de cours propre à chaque concentration. Pour génie mécanique, en revanche, les concentrations « Robotique » et « Génie du développement durable » ajoutent bien un ou deux cours obligatoires visibles directement dans le calendrier (ex. GMC-3351 apparaît en H8-H30). Cette différence de comportement entre programmes n'est expliquée nulle part — je ne peux pas savoir, en regardant l'écran, si l'absence de changement pour génie civil/physique signifie « cette concentration n'a pas de cours obligatoire propre » ou « le générateur n'a pas replacé mon cheminement ».

### Rien pour comparer les trois programmes côte à côte
- **Gravité** : mineur
- **Type** : friction / pas encore construit
- **Observé** : pour comparer génie civil, mécanique et physique, je dois changer de programme un par un (avec la friction de l'écran vidé notée plus haut), retenir mentalement les différences, et rouvrir chaque fois le panneau de règles. Rien n'indique les cours communs aux trois programmes (plusieurs sigles se recoupent pourtant : PHI-2910, PHI-3900, ECN-4901, STT-1900 reviennent dans plusieurs programmes), ni un total de crédits par session facile à comparer d'un coup d'œil entre programmes, ni un aperçu horaire type sans devoir tout re-générer. Le plan de projet prévoit au jalon 10 un « classement des combinaisons selon des préférences » (journées compactes, matins libres, pause dîner) qui pourrait aider indirectement, mais rien de spécifique à la comparaison inter-programmes n'y est prévu non plus — je le note ici comme un manque ressenti, pas un jalon annoncé.

### Texte de cours tronqué visuellement dans une case de session bien remplie
- **Gravité** : mineur
- **Type** : bogue (affichage)
- **Reproduction** : 1. Choisir génie physique, session A1 avec 6 cours (13 cr). 2. Regarder la case de la session A1 dans le bandeau du haut.
- **Attendu** : soit toutes les lignes de cours visibles, soit un indicateur de défilement/troncature clair (« +2 »).
- **Observé** : la dernière ligne de cours (« PHY-1003 ») est coupée à mi-hauteur, sans ascenseur ni mention « et plus » — capture `13-gph-medical-biophotonique.png`.

---

## Ce qui fonctionne bien (à mentionner pour équilibrer)

- Le cheminement complet se génère instantanément dès qu'on choisit un programme, avec des sessions de 12 à 15 crédits en général — des charges raisonnables pour une première session, cours de base d'abord (ex. Matériaux de construction, Statique en génie civil ; Statique, Introduction… en génie mécanique).
- Le message « ⚠ GCI-1011 — horaire pas encore publié : gardé dans la liste, rien n'est dessiné » et « GMC-2005 ajouté aux cours à option : un cours obligatoire l'exige comme préalable » sont des explications contextuelles claires, exactement le genre d'info qui aide une étudiante à comprendre pourquoi un cours est là.
- « Réinitialiser » (qui vide le cheminement du programme courant) reste annulable par Ctrl+Z — contrairement à « changer », qui désactive Annuler alors même que les données ne sont pas perdues. Cette incohérence entre deux boutons aux effets superficiellement semblables est déroutante.
- Le partage par URL fonctionne réellement : l'état complet (programme, version, concentration, cheminement) est encodé dans le fragment `#…` et survit à un rechargement de page.
- Au rechargement, on retombe exactement sur le dernier programme et la dernière concentration consultés, sans rien perdre.

## Non testé / hors de portée de cette exploration

- Le classement des combinaisons horaires par préférences (journées compactes, matins libres, pause dîner) n'existe pas dans l'interface — c'est prévu au jalon 10 du plan de projet (« Préférences et partage »), donc attendu comme absent à ce stade, pas un bogue.
- La contribution d'un cours manuel (jalon 10) : j'ai vu un lien « Cours absent du catalogue ? » dans le panneau mais je ne l'ai pas ouvert faute de temps ; à explorer dans une prochaine session.
- Je n'ai pas testé l'import d'un relevé de notes Capsule ni l'import d'un programme par URL/JSON, ces flux n'étant pas pertinents pour une étudiante qui n'a encore réussi aucun cours.

---

## Impression générale

L'outil génère très vite un cheminement complet et plausible pour les trois programmes, avec des explications ponctuelles utiles (cours pas encore publiés, cours ajoutés par un préalable) — c'est un bon point de départ pour se projeter. Mais pour l'usage que j'en ferais vraiment, comparer trois baccalauréats avant de faire mon choix, l'outil est actuellement plus fatigant que rassurant : chaque changement de programme fait disparaître l'écran entier sans dire si mon travail est conservé (il l'est, mais rien ne le montre), les concentrations sont difficiles à comparer visuellement puisque le calendrier ne bouge presque jamais, et je n'ai aucune vue d'ensemble mettant côte à côte génie civil, mécanique et physique. Je repartirais avec une bonne idée du contenu détaillé de chaque programme pris séparément, mais pas avec une réponse claire à ma vraie question du moment : lequel choisir entre les trois.
