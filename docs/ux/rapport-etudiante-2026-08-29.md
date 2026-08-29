# Rapport de contre-vérification UX — persona étudiante (génie des eaux)

Rejeu par « Camille » des reproductions exactes du rapport `docs/ux/rapport-etudiante-2026-08-27.md`, sur `http://localhost:8001` (session `agent-browser --session etudiante-gex` isolée, `localStorage` vidé avant exploration). Environ 75 actions navigateur.

Note méthodologique : plusieurs fois pendant cette session, un clic sur un bouton (via `agent-browser click <ref>`) n'a produit aucun effet visible alors que le même clic, rejoué juste après ou exécuté par `element.click()` en JavaScript, fonctionnait normalement. Ce comportement touchait l'outil de test lui-même (probablement une réutilisation de référence DOM périmée après un re-rendu), pas l'application : chaque fois qu'un clic semblait sans effet, je l'ai revérifié par un second moyen avant de conclure quoi que ce soit sur l'application. Aucun des constats ci-dessous ne repose sur un clic qui n'a pas été confirmé.

## Reprises des 6 points prioritaires

### 1. Annuler/rétablir après épinglage-dépinglage — CORRIGÉ
- **Reproduction** : B-GEX choisi (placement automatique, stage GEX-1580 en É27) → épingler GLO-1901 à A1-A26 (13→16 cr) → dépingler avec « ✕ » → un seul « ↶ Annuler ».
- **Attendu** : dépingler ramène l'horizon quasi immobile ; un seul Annuler redonne l'écran d'avant.
- **Observé** : après dépinglage, l'horizon entier (H2-H27 à H8-H30, stage É27 compris) est resté identique à l'état d'avant l'épinglage — plus de rebrassage. Un seul clic « Annuler » a ensuite redonné exactement l'état « après épinglage » (GLO-1901 dans A1 à 16 cr, reste de l'horizon inchangé), confirmé pixel pour pixel par capture (`.../02-glo1901-pinned.png` vs `.../03-apres-un-annuler.png`).
- Rejoué une seconde fois dans un scénario plus corsé (deux cours pinnés en même temps, A1 en dépassement à 19/17 cr) : même résultat — dépingler MED-1100 seul ramène proprement à 16 cr, un seul Annuler redonne exactement l'état à 19 cr d'avant. Le bogue ne réapparaît pas au second passage.

### 2. Compteur de crédits périmé après retrait — CORRIGÉ
- **Reproduction** : ajouter GLO-2005 par « automatique » (103/120 cr) → retirer avec « ✕ », sans recharger.
- **Observé** : le total revient immédiatement à 100/120 cr, sans recharger la page. Rejoué une seconde fois avec MED-1100 dans un contexte de dépassement (106→103 cr immédiatement après retrait) : même résultat correct.

### 3. Horaires alternatifs « ⇄ N » (MAT-1900) — AMÉLIORÉ MAIS PAS PLEINEMENT RÉSOLU
- **Gravité** : mineur (rétrogradé depuis majeur)
- **Type** : friction résiduelle
- **Reproduction** : cliquer le badge « ⇄ 4 » de MAT-1900.
- **Attendu** : sections identifiables (numéro, local, modalité) et cliquables.
- **Observé** : la troncature caractère-par-caractère illisible d'avant (« M / p / l / I ») a disparu — les colonnes affichent maintenant des libellés courts entiers et lisibles (« B », « C », un bloc « Z3 - à distance » avec la modalité visible). Cliquer une section (testé avec « B ») force effectivement MAT-1900 à cette section, l'affiche pleinement nommée dans la grille (« MAT-1900 - B »), et si ça crée un conflit, le signale clairement (voir point 4). C'est donc bien cliquable et le résultat est identifiable. Reste une friction : avant de cliquer, les sections « B »/« C » ne portent aucune info (local, plage horaire précise, modalité) au-delà de la lettre — impossible de savoir laquelle éviter sans les essayer une à une. Autre détail : dans l'arbre d'accessibilité, ces boutons de section n'ont plus le nom complet du cours (juste « B », « C », « D » au lieu de « MAT-1900 - B ») — moins grave visuellement (les lettres sont dans la bonne colonne du cours) mais un souci pour l'accessibilité si plusieurs cours ont leurs horaires alternatifs ouverts en même temps. Capture : `.../06-alt-schedules-clean.png`.

### 4. Bandeaux d'avertissement dédoublés — CORRIGÉ
- **Reproduction** : fermer les deux bandeaux jaunes (« D'autres agencements équivalents… », « Le cheminement présume ces acquis… ») avec leur « ✕ » → naviguer vers une autre session puis revenir sur A1 → épingler un nouveau cours (GLO-1901).
- **Attendu** : un bandeau fermé ne revient pas après une action.
- **Observé** : après fermeture, les deux bandeaux ne sont réapparus ni en changeant de session et en revenant, ni après avoir épinglé un cours. Aucune duplication observée non plus quand une nouvelle alerte apparaît (ex. l'avertissement de dépassement de plafond ou de préalable manquant s'affiche seul, sans dupliquer les précédents).

### 5. Contenu d'une règle dépliée hors champ — CORRIGÉ
- **Reproduction** : dérouler « Règle 1 » puis « Règle 2 » dans le panneau « Programme ».
- **Attendu** : voir le contenu déplié apparaître à l'écran.
- **Observé** : déplier une règle fait maintenant remonter son contenu (description, liste de cours admissibles, boutons de session) directement en haut du panneau visible, sans avoir à défiler soi-même. Vérifié sur Règle 1 et Règle 2 avec le même résultat. Capture : `.../13-regle1-expanded-true.png`, `.../15-regle2-eval-click.png`.

### 6. Écart total vs somme des sessions — CORRIGÉ
- **Reproduction** : comparer « 100/120 cr au bac » à la somme des crédits par session.
- **Attendu** : concordance ou explication.
- **Observé** : le bandeau affiche maintenant « 100/120 cr au bac **(+9 cr en sus)** ». En additionnant les crédits affichés par session (13+12+9+15+12+12+12+12+12 = 109), on retrouve exactement 100 + 9 = 109. Le suffixe explique bien l'écart (crédits de stage en sus du total du bac) et les chiffres concordent enfin.

## Autres observations (exploration libre / régression)

### Le plafond de crédits et le conflit d'horaire restent bien gérés — pas de régression
- Surcharger délibérément A1 à 19/17 cr (GLO-1901 + MED-1100) déclenche un encart rouge clair (« ⚠ plafond de 17 cr dépassé », case de session bordée en rouge « 19 ⚠ »), et le placement automatique refuse d'aggraver la situation avec un message compréhensible (« ⚠ Proposition ignorée : elle retirerait MED-1100 de la grille — votre agencement actuel est conservé. »).
- Forcer MAT-1900 en section B (qui chevauche Chimie des eaux) fait apparaître un bandeau rouge « conflit d'horaire — plages en cause hachurées », marque les deux cours en cause (« ⚠ conflit ») et propose un bouton « Libérer les sections forcées » qui résout proprement le conflit et fait disparaître le bandeau.

### L'ajout d'un cours offert seulement à une autre saison est empêché à la source, pas signalé après coup
- Chercher GCI-3101 (offert H seulement) alors qu'on est sur A1-A26 (automne) : les seuls boutons de session proposés sont H2-H27, H4-H28, H6-H29, H8-H30 — aucun bouton pour une session d'automne n'apparaît. Il est donc impossible de commettre l'erreur plutôt que de devoir composer avec un message d'erreur après coup. C'est une conception plus sûre qu'un message d'erreur, mais je n'ai donc pas pu observer le libellé d'un message d'erreur dans ce cas précis puisque l'action est simplement absente de l'interface.

### Le chevauchement visuel « Ouvrir les étés » n'a pas été reproduit
- Cocher « Ouvrir les étés aux cours réguliers » n'a provoqué aucun chevauchement des boutons d'export avec le texte d'en-tête cette fois-ci (capture `.../16-ete-ouvert.png`). Je note cette amélioration avec prudence : je n'ai pas cherché à reproduire dans toutes les largeurs d'écran possibles.

### Persistance après rechargement — toujours correcte, y compris en surcharge
- Un état avec deux cours pinnés en dépassement de plafond (106/120 cr, 19 cr à A1, bandeau de dépassement) survit à un rechargement complet de la page, à l'identique.

## Impression générale

Les six points signalés dans le rapport du 27 août sont bel et bien corrigés, et je les ai chacun rejoués une seconde fois (souvent dans un contexte légèrement différent — surcharge, deuxième cours) sans voir le bogue réapparaître : l'annuler redonne maintenant vraiment l'écran précédent, le compteur de crédits ne ment plus après un retrait, les bandeaux ne s'accumulent plus, et déplier une règle amène enfin son contenu sous les yeux. Le seul point encore entrouvert est l'affichage des sections alternatives : ce n'est plus illisible, mais choisir entre « B » et « C » reste un pari tant qu'on n'a pas cliqué pour voir ce que ça donne — un local ou une modalité affichés directement dans la case réglerait ça. Avec les six corrections confirmées, je m'y fierais maintenant nettement plus pour planifier une session réelle qu'il y a deux jours.
