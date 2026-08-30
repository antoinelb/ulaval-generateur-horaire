# Rapport d'exploration — étudiante GEX

Persona : Camille, étudiante de 2e année au baccalauréat en génie des eaux (GEX).
Session localStorage vidée avant exploration (`etudiante-gex`). Environ 80 actions navigateur.
Captures dans `/home/antoine/.claude/jobs/1719c21c/tmp/`.

## Constats

### « Réinitialiser » efface tout le cheminement sans confirmation
- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Choisir B-GEX, ajouter MED-1100 à la Règle 1 (session A1-A26 passe à 16 cr).
  2. Cliquer sur le bouton « Réinitialiser » du bandeau du haut (juste à côté de « Partager »).
- **Attendu** : au minimum une question de confirmation (« Voulez-vous vraiment réinitialiser ? »), vu l'ampleur de la perte potentielle (tout un cheminement personnalisé).
- **Observé** : le plan revient instantanément à l'état généré par défaut (MED-1100 disparu, 13 cr au lieu de 16, la Règle 1 redevient « à combler ») sans aucun dialogue, aucun toast, rien. Le bouton « Annuler Ctrl+Z » reste actif et permet heureusement de tout récupérer — mais rien à l'écran n'indique que c'est possible, ni que quelque chose de grave vient de se produire. Une étudiante qui clique par réflexe (le bouton est juste à côté de « Partager », que je clique souvent) perd son travail sans le savoir récupérable. Capture : `44-reinitialiser.png`, `45-annuler-reinit.png`. Erreur console : aucune.

### « Partager » ne donne aucun retour visuel
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Depuis un plan avec des modifications. 2. Cliquer sur « Partager ».
- **Attendu** : un toast ou un changement visuel confirmant que le lien a été copié (ou au moins généré), pour que je sache que l'action a fonctionné.
- **Observé** : rien ne change visuellement à l'écran. En vérifiant l'URL de la page, elle contient bien un nouveau fragment `#…` encodant l'état — donc l'action a fonctionné — mais rien à l'écran ne me le dit. Capture : `41-partager.png`. Erreur console : aucune (le presse-papiers n'a pas pu être vérifié directement, permission refusée par le navigateur automatisé).

### Les bandeaux d'avertissement restent affichés longtemps et chevauchent la grille
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Ajouter un cours dont le préalable manque (ex. GAE-2005) — un avertissement apparaît en bas à droite. 2. Poursuivre d'autres actions (résoudre un conflit, changer de session, etc.).
- **Attendu** : que le message se referme après un délai raisonnable, ou clairement quand il n'est plus pertinent.
- **Observé** : les bandeaux (« GAE-2005 ajouté, mais ses préalables… », « Proposition ignorée… ») restent affichés sur plusieurs actions suivantes, jusqu'à chevaucher une partie de la grille horaire du vendredi (le dernier cours affiché sous eux est partiellement caché). Il y a un bouton « ✕ » pour les fermer, mais rien n'invite à le faire. Capture : `29-surcharge-session.png`, `32-surcharge-final.png`.

### « 1 cours hors grille » : terme peu explicite au premier contact
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Ajouter MED-1100 (cours offert à distance / sans horaire fixe) à une session. 2. Regarder l'alerte « ⚠ 1 cours hors grille » au-dessus de la grille horaire.
- **Attendu** : comprendre immédiatement pourquoi le cours ne s'affiche pas dans la grille.
- **Observé** : le message est correct une fois qu'on comprend que « hors grille » veut dire « ce cours compte dans les crédits de la session mais n'a pas d'horaire publié (souvent un cours à distance ou en ligne) » — mais rien à l'écran (pas d'info-bulle, pas de lien) n'explique le terme la première fois qu'on le voit. On doit déduire le sens en regardant la fiche du cours dans le panneau de gauche (« MED-1100 - 3 cr - placé en A1-A26 » n'indique pas non plus explicitement l'absence d'horaire, seulement l'absence du bloc dans la grille). Capture : `09-med1100-ajoute.png`.

## Ce qui fonctionne bien (à noter pour contexte)

- Le placement automatique complet du bac au moment du choix du programme est impressionnant : les 34 cours obligatoires sont répartis sur 8 sessions + un été obligatoire (GEX-1580), avec un total de crédits cohérent (100/120 + 9 cr en sus pour un stage) et un avertissement clair sur les 5 sections de règles à option qui restent à combler.
- Le détail d'une règle à option (ex. Règle 1 : « Choisissez 1 cours dans cette liste — rien n'est pris automatiquement ») est très clair, avec la liste des cours possibles, leurs préalables repliables, et des boutons de session explicites.
- Demander un cours dont le préalable n'est pas rempli (GAE-2005, préalable FOR-2151 ou GMC-1002) donne un message compréhensible en français : « GAE-2005 : préalable manquant — il faudrait FOR-2151 ou GMC-1002, ni acquis ni prévu au cheminement. Ajoutez-le aux cours à option, ou réglez-le par entente avec la direction. » C'est exactement le niveau d'explication qu'une étudiante peut comprendre sans jargon.
- Demander un cours d'hiver à l'automne n'est même pas possible : l'interface ne propose tout simplement pas de bouton de session hors saison pour ce cours (seulement H2, H4, H6, H8 pour un cours « offert H »). C'est une prévention plus efficace qu'un message d'erreur après coup.
- Créer un conflit d'horaire volontairement (forcer la section B de MAT-1900 qui chevauche Chimie des eaux le vendredi) est détecté immédiatement et signalé à trois endroits cohérents : l'onglet de la session (« ⚠ conflit d'horaire », rouge), l'en-tête « Vendredi ⚠ », et un bandeau au-dessus de la grille avec un bouton « Libérer les sections forcées ». Le clic sur ce bouton résout le conflit instantanément et le signalement disparaît. Reproduit deux fois (avant et après un rechargement de page), comportement identique les deux fois.
- Surcharger une session au-delà du plafond de crédits (17 cr) est permis mais clairement signalé : « 19 cr cette session ⚠ plafond de 17 cr dépassé » à la fois dans le bandeau du haut et sur l'onglet de la session concernée.
- Marquer un cours comme réussi (bouton « créditer ») fonctionne bien : le cours sort de la grille horaire, ses crédits sont retirés du total de la session, et la règle correspondante se met à jour (« ✓ 1/1 »). Annuler (recliquer sur « crédité ✓ ») remet tout exactement comme avant.
- La persistance après rechargement de la page est parfaite : programme choisi, cours ajoutés/retirés, statut « crédité », case « Ouvrir les étés » décochée — tout est conservé à l'identique après un `reload`.
- Geler/dégeler une session fonctionne et s'affiche clairement (« ❆ gelée » sous le nom de session, bouton qui devient « ❄ Dégeler »).
- Aucune erreur JavaScript n'est apparue dans la console pendant toute la session d'exploration (recherché à plusieurs reprises avec `agent-browser errors`/`console`).

## Pas encore construit (jalon 10, mentionné pour information)

- Aucune option de préférences de type « journées compactes », « matins libres » ou « pause dîner » n'est visible dans l'interface — cohérent avec le jalon 10 du plan de projet (« classement des combinaisons valides selon des préférences ») qui reste à livrer.
- Je n'ai pas testé la contribution d'un « cours manuel » (aussi prévue au jalon 10) ; j'ai seulement vu le bouton replié « Cours absent du catalogue ? » dans le panneau de gauche, sans l'ouvrir plus en profondeur.
- Le partage par URL, lui, est déjà construit et fonctionne (voir plus haut) — ce n'est donc pas à mettre au jalon 10 malgré ce qui était attendu au départ.

## Non testé / limites de cette session

- Je n'ai pas de véritable relevé de notes Capsule sous la main : j'ai seulement ouvert le tiroir « Charger depuis Capsule » et lu les instructions (ouvrir Capsule, Ctrl+U pour la source, Ctrl+A/Ctrl+C, coller) sans aller plus loin. Les instructions semblent claires mais je ne peux pas juger du résultat réel.
- Je n'ai pas testé l'import d'un programme via une URL ulaval.ca externe, ni le chargement d'un cheminement depuis un fichier JSON local, faute de fichier à disposition.
- Je n'ai pas testé le profil « international », ni changé la session de « Début » du bac.
- Je n'ai pas ouvert les exports PDF/JSON en détail (juste ouvert le menu « Exporter ▾ » pour voir les trois destinations, sans télécharger).

## Impression générale

Une fois le programme choisi, l'outil m'impressionne : il me construit un bac complet en un clic, avec un langage clair sur ce qui reste à choisir, des messages d'erreur compréhensibles (préalables manquants expliqués en français, pas de jargon), une détection de conflit d'horaire fiable et facile à résoudre, et une persistance sans faille après rechargement. C'est exactement le genre d'outil que j'utiliserais pour planifier mon propre cheminement en GEX.

Ce qui m'inquiète, c'est le bouton « Réinitialiser » placé juste à côté de « Partager » dans le bandeau du haut, sans aucune confirmation : un clic malheureux efface tout mon travail d'un coup, et rien à l'écran ne me dit que c'est récupérable via Ctrl+Z. Avant de m'y fier pour de vrai, j'aimerais que ce bouton demande une confirmation, ou qu'il soit visuellement séparé des actions courantes. Les autres frictions relevées (pas de retour visuel sur « Partager », bandeaux d'avertissement qui s'attardent et chevauchent la grille, terme « hors grille » pas expliqué au premier contact) sont mineures et n'empêcheraient pas de m'en servir.
