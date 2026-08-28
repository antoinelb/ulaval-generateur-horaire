# Rapport d'exploration UX — Élodie, finissante au cégep

**Date :** 2026-08-27
**Persona :** future étudiante en sciences de la nature, aucun cours universitaire réussi, hésite entre génie civil, génie mécanique et génie physique à l'Université Laval.
**Session de navigateur isolée :** `agent-browser --session etudiante-cegep`

Captures dans `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/07141d69-919b-4129-8679-333a9b9566ad/scratchpad/screenshots/`.

---

## Constats, du plus grave au plus bénin

### Choisir un programme pour la première fois affiche un cheminement daté de 2024, avec 97 crédits déjà « acquis »
- **Gravité** : majeur
- **Type** : bogue (ou friction non documentée — je n'ai trouvé aucun ADR justifiant ce défaut)
- **Reproduction** :
  1. Cliquer « Réinitialiser » pour repartir d'un état vraiment vide (« aucun programme choisi », 0 cr).
  2. Choisir « Baccalauréat en génie civil » (B-GCI, version A26) — un tout nouveau choix, sans historique.
  3. Regarder le champ « Début » du panneau de gauche et la première session affichée.
- **Attendu** : en tant que future admise sans aucun cours réussi, je m'attendais à démarrer à la première session à venir (Automne 2026 ou plus tard), avec 0 crédit déjà comptabilisé nulle part.
- **Observé** : le champ « Début » est pré-rempli à **A24** (Automne 2024, il y a deux ans), la grille affiche « A1 — Automne 2024 » avec un horaire réel généré pour ce trimestre passé, et l'en-tête indique déjà **« 97/120 cr au bac »**, avec les cinq premières sessions marquées d'un « ✓ » dans la barre de navigation — sans qu'aucune case « cours déjà réussi » n'ait jamais été cochée par moi. Rien à l'écran n'explique que « Début » dans le passé fait ce genre de chose. En changeant manuellement « Début » à A26, les « ✓ » disparaissent mais le total de crédits (97/120) ne change pas — capture : `03-genie-civil-defaut-A24.png`, `04-genie-civil-A26-sans-concentration.png`. Erreur console : aucune.
- **Impact pour Élodie** : le tout premier écran d'un programme fraîchement choisi ressemble à celui d'une étudiante en fin de bac, pas à une nouvelle admise. Si je n'avais pas remarqué et corrigé le champ « Début », j'aurais comparé des cheminements qui n'ont aucun sens pour ma situation.

### Changer de concentration ne vide pas le cheminement précédent — un cours choisi manuellement reste placé et se recompte silencieusement sous une autre règle
- **Gravité** : majeur
- **Type** : friction (comportement volontaire d'après le plan de projet — « changeable à tout moment sans toucher la grille placée » — mais non expliqué à l'écran)
- **Reproduction** :
  1. Génie civil, concentration « Eau et environnement », dérouler « Règle 1 » et cliquer « automatique » sur *Évaluation environnementale* (FOR-2020) pour la placer.
  2. Le total passe de 97/120 à 100/120, le cours apparaît « placé en A3-A27 ».
  3. Remettre la concentration à « Cheminement sans concentration ».
- **Attendu** : revenir à « sans concentration » me semblait équivalent à annuler l'essai de la concentration précédente — je m'attendais à un cheminement redevenu neutre (comme avant l'essai), ou au minimum à un avertissement.
- **Observé** : le total reste à **100/120**, FOR-2020 reste « placé en A3-A27 », et la section « Concentration — Cheminement sans concentration → Règle 1 » affiche maintenant **3/15 cr** — le cours d'une concentration que je n'ai plus sélectionnée compte pour une règle différente, sans aucun message. Le comportement est identique en le refaisant une seconde fois (testé sur génie mécanique : Robotique → Génie du développement durable → sans concentration, capture `07-residu-concentration.png`).
- **Impact pour Élodie** : en comparant plusieurs concentrations d'affilée (exactement ce que je suis venue faire), mon compteur de crédits et mes règles satisfaites deviennent un mélange de tous mes essais précédents, sans que je puisse distinguer « ce qui vient du cheminement de base » de « ce qui traîne d'un essai ». Je n'ai trouvé aucun bouton « réinitialiser seulement ce programme » — seul « Réinitialiser » global (qui efface aussi génie mécanique et génie physique) semble tout nettoyer.

### Dans la grille horaire, le code de section d'un cours au titre long est caché sous le bloc suivant
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Génie physique, concentration « Génie médical et biophotonique », première session (A1 — Automne 2026).
  2. Regarder mercredi, vers 16h30-17h30 : le bloc « Mécanique et relativité restreinte ».
- **Attendu** : pouvoir lire le sigle et la section de chaque cours affiché dans la grille.
- **Observé** : le titre du cours occupe deux lignes et repousse le sigle (« PHY-1002 - A ») hors du bloc visible, où il est recouvert par le bloc du cours suivant (« Introduction à la programmation avec Python ») — capture `08-gph-genie-medical.png`, zoom `08-crop-overlap2.png`. Le sigle du cours de 16h30 est illisible sans cliquer dessus. Erreur console : aucune (visuel seulement).
- **Impact pour Élodie** : impossible de vérifier d'un coup d'œil quel cours/section est réellement à cet horaire; il faut cliquer chaque bloc suspect pour en être sûre.

### Le programme choisi par défaut n'est pas cohérent entre génie civil/mécanique (« sans concentration ») et génie physique (une concentration précise, choisie sans le dire)
- **Gravité** : mineur
- **Type** : friction (comportement documenté — « défaut : la première concentration du millésime » — mais surprenant vu d'écran)
- **Reproduction** :
  1. Choisir génie civil ou génie mécanique fraîchement : la concentration par défaut est « Cheminement sans concentration ».
  2. Choisir génie physique fraîchement : la concentration par défaut est « Aéronautique et aérospatiale », pas « Aucune ».
- **Attendu** : un point de comparaison neutre identique pour les trois programmes au premier contact.
- **Observé** : pour génie physique, l'en-tête affiche directement « — Aéronautique et aérospatiale » sans que j'aie rien choisi, et le menu « Concentration » liste pourtant « Aucune » en premier. Rien n'indique que ce choix a été fait à ma place.
- **Impact pour Élodie** : en comparant les trois programmes « tels quels » à l'ouverture, je compare en fait génie civil sans concentration à génie physique *avec* une concentration précise — une comparaison faussée si je ne remarque pas le menu déroulant.

### Doublons dans la liste des préalables « présumés acquis »
- **Gravité** : mineur
- **Type** : bogue
- **Reproduction** :
  1. Génie civil ou génie physique, n'importe quelle session avec des cours de scolarité préparatoire en amont.
  2. Regarder le bandeau jaune « ⚠ Le cheminement présume ces acquis… » en bas à droite de l'horaire.
- **Attendu** : une liste de sigles sans répétition.
- **Observé** : pour génie civil, la liste affiche « MAT-0130, MAT-0150, MAT-0150, MAT-0260 » (MAT-0150 en double). Pour génie physique, « MAT-0150, MAT-0260, MAT-0150, MAT-0260, MAT-0130, MAT-0150, MAT-0260, PHY-0150 » (MAT-0150 trois fois, MAT-0260 trois fois) — capture `08-gph-genie-medical.png`.
- **Impact pour Élodie** : mineur, mais ça donne l'impression que la liste des préalables présumés n'est pas fiable/vérifiée.

### Beaucoup de vocabulaire non expliqué à l'écran
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : parcourir le panneau de gauche de n'importe quel programme.
- **Attendu** : en tant que future étudiante sans vocabulaire universitaire, je m'attendais à des info-bulles ou du texte explicatif pour les termes spécialisés.
- **Observé** : la case à cocher « Permettre un préalable en concomitance » n'explique nulle part ce qu'est la « concomitance » (l'astérisque « GCI-2010* » du répertoire n'est pas expliquée non plus). Le menu « Rattacher [cours] à une règle » propose une option « entente… » sans précision (« entente avec la direction » n'apparaît qu'en infobulle du bouton, pas dans le menu lui-même). Le bouton « créditer » à côté de chaque cours électif n'a pas d'explication de ce qu'il fait par rapport à « automatique ».
- **Impact pour Élodie** : je dois deviner par essai-erreur ce que font plusieurs commandes, alors que je suis exactement le public (nouvelle admise) qui n'a pas ce vocabulaire.

### Remplir une exigence complémentaire à la main demande de faire défiler une très longue liste de cours, un par un
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Génie civil, dérouler « Autres exigences – Règle 1 » (3 crédits à choisir).
- **Attendu** : un choix rapide, vu qu'il s'agit d'un petit 3 crédits.
- **Observé** : la liste déroule une dizaine de cours d'anglais (Intermediate English II, Advanced English I, Advanced English II, Workplace English, etc.), chacun avec son propre bloc de préalables et une rangée de 8 à 12 boutons de session à cliquer un par un — capture `05-debug-panel.png`. Rien ne permet de filtrer ou de voir d'un coup d'œil lesquels me seraient réellement offerts en A1.
- **Impact pour Élodie** : comparer « combien ça prend d'efforts pour combler mes 3 crédits d'anglais » entre programmes demande de faire défiler la même longue liste à chaque programme.

### État initial de la session avec des données d'un test précédent (contexte, pas un vrai bogue applicatif)
- **Gravité** : mineur
- **Type** : friction (persistance `localStorage` qui fonctionne comme prévu, mais surprenante ici)
- **Reproduction** : ouvrir l'application avec la session `etudiante-cegep` sans avoir encore rien fait.
- **Attendu** : en tant que toute nouvelle utilisatrice, un état vierge.
- **Observé** : le tout premier écran affichait déjà « Baccalauréat en génie mécanique … 111/120 cr au bac », avec des cours placés — capture `01-premier-contact.png`. Après « Réinitialiser » puis nouvel essai, retourner sur génie mécanique ou génie physique faisait réapparaître ces mêmes credits (111/120, puis 104/120 avec un cours GEL-4799 placé en toute première session, un sigle 4xxx pourtant avancé). Ceci vient très probablement de `localStorage` laissé par une exploration antérieure de ce même profil de navigateur, pas d'un défaut de l'app — mais ça a rendu plus difficile de juger le vrai « premier contact » pour génie mécanique et génie physique (contrairement à génie civil, testé après un `Réinitialiser` complet et confirmé propre). Je note ce point explicitement plutôt que de le passer sous silence : je n'ai pas pu observer un premier contact 100 % vierge pour génie mécanique et génie physique.

---

## Tests prévus non complétés

- Je n'ai pas testé le bouton « Partager » (aller-retour par URL), ni « Exporter l'organigramme »/« Exporter l'horaire ».
- Je n'ai pas testé « Charger depuis Capsule » (import de relevé de notes).
- Je n'ai pas testé « Cours absent du catalogue ? » (ajout manuel d'un cours).
- Je n'ai pas testé les profils (« Profil entrepreneurial », « Profil international », etc.) au-delà de constater leur présence dans le menu.
- Je n'ai pas testé les cases « Ouvrir les étés aux cours réguliers » et « Permettre un préalable en concomitance ».
- Je n'ai essayé que 3 des 4 concentrations de génie civil (sans concentration, Eau et environnement, Géotechnique) et 3 des 7 de génie physique (Aucune, Photonique, Génie médical et biophotonique) — pas les autres, faute de budget d'actions.

## Impression générale

L'outil répond bien à une partie de ma question : une fois qu'on comprend comment le lire, chaque programme affiche clairement ses cours obligatoires, ses règles à combler et un horaire hebdomadaire concret par session, avec un vrai calcul de non-conflit — la mécanique de base marche, sans latence perceptible, et la reprise après un rechargement de page fonctionne bien (le programme et la concentration actifs sont retrouvés, et l'exploration des autres programmes reste disponible en arrière-plan).

Mais pour mon vrai besoin — **comparer** génie civil, génie mécanique et génie physique afin de choisir — l'outil m'aide moins que je l'espérais. D'abord parce que le premier programme que je choisis vraiment (génie civil) démarre par défaut sur une date passée avec des crédits déjà comptés, ce qui aurait pu me faire abandonner l'outil avant même de commencer si je n'avais pas creusé. Ensuite parce que mes essais de concentrations laissent des traces les uns dans les autres sans avertissement : après avoir essayé deux ou trois concentrations d'un même programme, je ne sais plus si le nombre de crédits affiché reflète vraiment « ce cheminement-là » ou un mélange de mes essais précédents — exactement le genre de doute qui rend une comparaison peu fiable. Enfin, rien dans l'interface ne m'aide directement à comparer : pas de vue côte-à-côte, pas de mise en évidence des cours communs aux trois programmes (plusieurs cours de première session se ressemblent, ex. MAT-1900/STT-1900 apparaissent dans plusieurs baccalauréats, mais je dois le remarquer moi-même en changeant d'onglet), pas de résumé de la charge par session pour arbitrer entre les trois. Je ressors de cette session avec une bonne idée de ce que contient chaque programme individuellement, mais pas avec une réponse claire à « lequel me convient le mieux » — pour ça, il me faudrait recommencer une comparaison propre, session par session, en gardant moi-même un tableau à part.
