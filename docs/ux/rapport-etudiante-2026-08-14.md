# Rapport d'essai utilisateur — Camille, 2e année GEX — 2026-08-14

Programme testé : baccalauréat en génie des eaux, B-GEX version A26.
Environ 95 actions dans le navigateur, fenêtre 1280 × 577.
Captures dans `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/77509ff8-8e54-4de7-82f6-e6b06a120e43/scratchpad/camille/`.
Aucune erreur JavaScript pendant toute la session (`console` et `errors` vides).

---

### Mon travail a disparu après un rechargement de page
- **Gravité** : majeur
- **Type** : bogue (observé une fois, non reproduit ensuite)
- **Reproduction** : 1. Programme B-GEX A26, « Proposer un organigramme ». 2. Ajouter GAE-2005 en H2-H27 par le champ « Code du cours à ajouter », puis le retirer avec son ✕. 3. Cocher « Ouvrir les étés », ajouter MAT-1906 puis MAT-1905 dans É28. 4. Recharger la page.
- **Attendu** : retrouver exactement ce que j'avais laissé (GAE-2005 absent, deux cours en É28).
- **Observé** : GAE-2005 était de retour en H2-H27 (18 cr, ⚠) et É28 était vide. Une dizaine de manipulations perdues d'un coup, sans aucun message. Erreur console : aucune. J'ai ensuite refait des retraits et des ajouts (y compris en session d'été) suivis d'un rechargement : à ces essais-là tout a persisté correctement, donc je n'ai pas réussi à reproduire. Je signale aussi qu'un message de développement « Hot-patch success! » de Dioxus traînait dans la page — le serveur a peut-être rechargé l'application en cours de route, ce qui expliquerait le retour en arrière. À vérifier hors mode développement.

### « Ouvrir les étés aux cours réguliers » décoché ne m'empêche pas de mettre des cours réguliers l'été
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** : 1. Décocher « Ouvrir les étés aux cours réguliers ». 2. Cliquer la tuile É28. 3. Champ « Code du cours à ajouter » → `MAT-1905` → « Ajouter ».
- **Attendu** : soit un refus (« les étés sont fermés, cochez la case »), soit au minimum un avertissement, comme pour la scolarité préparatoire qui, elle, retire les cours et le dit.
- **Observé** : le cours est ajouté sans un mot (« É28 3 MAT-1905 »), et le panneau continue d'afficher « Placement vérifié ✓ (préalables, plafond, une combinaison d'horaire possible par session) ». Même chose si je referme les étés alors que des cours y sont déjà : rien n'est retiré, rien n'est signalé. Pourtant l'application connaît la contrainte : un autre message dit « ⚠ Le cheminement affiché brise une contrainte (préalable, plafond, **été fermé** ou conflit d'horaire) ». Capture : `20-conflit.png` pour le style d'avertissement qui existe, absent ici. Erreur console : aucune.

### Scolarité préparatoire décochée : impossible de faire placer les cours, et le message tourne en rond
- **Gravité** : majeur
- **Type** : bogue (ou fonction non prévue mais annoncée par le texte)
- **Reproduction** : 1. Programme B-GEX A26, « Proposer un organigramme » (100/120 cr, tout est vérifié). 2. Ouvrir « Scolarité préparatoire » et décocher « déjà faite ». 3. Cliquer « Proposer un organigramme ».
- **Attendu** : puisque je déclare devoir faire ces 8 cours, qu'ils soient répartis dans mes sessions comme le reste.
- **Observé** : l'organigramme ne bouge pas d'un cours, et le panneau affiche « 8 cours sans session — proposez un organigramme ou placez-les » — c'est justement ce que je viens de faire. Le toast dit « ⚠ La recherche s'est arrêtée avant d'avoir tout exploré […] « Chercher plus longtemps » fouille davantage ». J'ai cliqué « Chercher plus longtemps » : un indicateur « recherche d'un organigramme - 0 s » apparaît (bien), puis… rien ne change et **aucun message ne me dit si la recherche a abouti ou échoué**. J'ai aussi monté le plafond à 18 cr et relancé : même résultat. Captures : `12-organigramme-decoche.png`, `13-pendant-recherche.png`, `14-apres-recherche.png`. Erreur console : aucune. Si ces cours ne doivent jamais être placés automatiquement, le texte « proposez un organigramme » ne devrait pas les compter.

### Les messages d'erreur des deux champs d'ajout sont écrits en bas du panneau, souvent hors écran, et ne partent jamais
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Faire défiler le panneau jusqu'en bas. 2. Champ « Code du cours à ajouter » → `XYZ-1234` → « Ajouter ». 3. Remonter dans le panneau et travailler ailleurs.
- **Attendu** : le même traitement que les autres messages, c'est-à-dire un toast en bas à droite qui se voit et se ferme.
- **Observé** : « « XYZ-1234 » est introuvable dans le catalogue — vérifiez le sigle (ex. GEX-1000). » s'affiche en petit rouge sous le bouton « Ajouter », donc sous le pli si le panneau est défilé ailleurs — au premier essai je ne l'ai vu qu'à moitié coupé par le bas de la fenêtre (capture `08-refus-code-prepa.png`). Pire, le message du formulaire « Cours absent du catalogue ? » (« MAT-0130 existe déjà dans le catalogue — le cours officiel prime. ») est resté affiché pendant tout le reste de ma session, sans croix pour le fermer.

### Quand j'affiche tous les messages, ils recouvrent la moitié droite de ma grille horaire
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Accumuler 4 messages (proposer un organigramme, ajouter un cours sans préalable, « Partager »…). 2. Cliquer « +1 autres messages - tout afficher ».
- **Attendu** : lire mes messages sans perdre de vue mon horaire.
- **Observé** : la pile dépliée occupe toute la hauteur du côté droit et cache les colonnes jeudi et vendredi de la grille, y compris les blocs de cours cliquables (capture `19-toasts-tout.png`). Avec 3 messages c'est déjà la moitié de vendredi qui est couverte (`03-organigramme.png`). Aucun contrôle du panneau ni de l'en-tête n'est masqué, en revanche.

### Le champ de recherche reste rempli après un rechargement et cache toutes les règles du programme
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Taper `MAT-1200` dans « Chercher dans tout le catalogue… ». 2. Recharger la page.
- **Attendu** : retrouver mon panneau normal (Obligatoires, Règle 1…, Scolarité préparatoire).
- **Observé** : le panneau ne montre que le résultat de recherche ; j'ai cru un instant que la liste des règles avait disparu, il faut penser à vider le champ (le ✕ est là) pour la revoir.

### Cliquer un cours dans la grille ne fait rien, sans que je sache pourquoi
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** : 1. Ouvrir une session. 2. Cliquer sur un bloc de cours plein (par exemple « Introduction à l'analyse MAT-1120 - A »).
- **Attendu** : d'après la légende « pointillé = autre plage possible (cliquer pour la forcer) », je m'attendais soit à un effet, soit à comprendre que ce bloc-là n'est pas cliquable.
- **Observé** : rien du tout, aucun message. Les blocs pleins sont pourtant de vrais boutons (curseur et focus), ce qui invite au clic.

### Détails de langue et de mise en page
- **Gravité** : mineur
- **Type** : friction
- « +1 autres messages - tout afficher » : au singulier, ce serait « +1 autre message ».
- Les tuiles d'été non planifiées s'affichent « É27 - — » en texte vertical dans une colonne de 20 px, alors que les autres sessions disent « à planifier ». Difficile à lire et à viser (capture `02-entete.png`).
- La grille horaire descend jusqu'à 22h30 : beaucoup de vide à faire défiler pour lire la note « GSC-1000 — sans plage hebdomadaire (à distance) : suivi hors grille » placée tout en bas (capture `05-horaire-bas2.png`).
- Dans les colonnes étroites, les titres sont coupés (« Méthodes statistiq… pour l'ingénie… ») ; le sigle reste lisible, donc ça passe.
- « Annuler » et « Rétablir » redeviennent grisés après un rechargement : je perds la possibilité de revenir en arrière sur ce que j'ai fait avant de recharger.
- Placer deux cours de scolarité préparatoire (6 cr) ne change pas le compteur « 100/120 cr au bac », alors qu'ajouter GAE-2005 rattaché à « entente… » le fait passer à 103/120. La règle du compteur n'est expliquée nulle part.

---

## Ce qui marche, et bien

**Scolarité préparatoire (case « déjà faite »)** — c'est solide. Cochée, les huit cours affichent « considéré comme déjà fait - décochez la case pour le placer » et je n'ai trouvé aucune porte dérobée :
- les rangées de la règle n'ont ni « + » ni « où le placer ? » ;
- la recherche du catalogue trouve bien MAT-0130 mais sans bouton d'ajout ;
- le champ « Code du cours à ajouter » refuse : « BIO-0150 fait partie de la scolarité préparatoire cochée « déjà faite » — décochez la case pour le placer. » ; il refuse aussi ` chm-0170 ` en minuscules avec des espaces ;
- le formulaire « Cours absent du catalogue ? » refuse de recréer MAT-0130 : « existe déjà dans le catalogue — le cours officiel prime » ;
- « Proposer un organigramme » ne place aucun cours 0xxx.

Décochée, les « + » apparaissent avec une indication utile par cours (« offert A-H - rentrerait ✓ », « l'ajouter ici créerait un conflit », « présumé acquis : CHM-0150 »). J'ai placé CHM-0150 et MAT-0130 en A1 : ils apparaissent bien dans la grille (capture `10-prepa-places.png`). En recochant, le toast les nomme : « ⚠ CHM-0150, MAT-0130 retirés des sessions : la scolarité préparatoire est marquée « déjà faite ». Décochez la case pour les replacer. » (capture `11-recoche.png`) et « Annuler » restaure tout — case décochée et cours replacés — puis « Rétablir » refait le retrait. Refait une deuxième fois après réinitialisation complète : comportement identique, message correctement au singulier pour un seul cours.

**Bouton « Réinitialiser… »** — conforme. Un clic simple n'efface rien : l'en-tête affiche « Tout effacer ? Cette action n'est pas annulable. » avec « Confirmer : tout effacer » et « Garder » (capture `23-reset-arme.png`). « Garder » désarme et je retrouve mon organigramme intact. Après confirmation, tout est vide (programme, sessions, historique) et le reste après rechargement.

**Toasts** — conformes à ce qui est annoncé : le ✓ de « Partager » disparaît seul après environ 5 s, les ⚠ persistent, cliquer n'importe où sur un message le ferme, trois clics de suite sur « Partager » ne créent qu'un seul message, et au-delà de trois s'affiche « +N autres messages - tout afficher ». Surtout, ils ne poussent plus le panneau ni l'horaire : je peux continuer à travailler pendant qu'ils sont là.

**Messages métier** — clairs, sans jargon, c'est le gros point fort :
- « GEX-3502 n'est pas offert à cette saison (offert : hiver). »
- « GEX-2001 est déjà placé en H8-H30 — retirez-le de là d'abord. »
- « ⚠ GAE-2005 ajouté, mais ses préalables ne semblent pas remplis (préalables : FOR-2151 OU GMC-1002). » — l'application me laisse faire mais me prévient, c'est exactement ce que je veux.

**Organigramme et horaire** — « Proposer un organigramme » répond en moins d'une seconde, place les 34 obligatoires et le stage GEX-1580 à l'été 27, et annonce honnêtement ce qui reste : « Placement vérifié ✓ » puis « ⚠ mais 4 sections de règles restent à combler ci-dessous — le bac n'est pas complet ». Les crédits par session sont plausibles (13, 15, 15, 15, 12, 6, 12, 12), un dépassement du plafond est signalé dans la tuile (19 ⚠) et dans l'en-tête (« ⚠ plafond de 15 cr dépassé »). La grille hebdomadaire est lisible, les heures plausibles (8h30 à 17h30 pour l'essentiel), et le cours à distance est expliqué au lieu d'être perdu. Le bouton « où le placer ? » propose les sessions possibles (« + É27 », « ici : É28 », « + É29 »…), c'est très parlant (capture `22-ou-le-placer.png`).

**Conflit d'horaire** — bien traité. En ajoutant MAT-1120 à H2 : la tuile passe en rouge avec « ⚠ conflit d'horaire », les en-têtes « Mercredi ⚠ » et « Jeudi ⚠ » sont marqués, les plages en cause sont hachurées et l'en-tête de l'horaire dit « ⚠ conflit d'horaire — plages en cause hachurées » (capture `21-conflit-grille.png`). En retirant le cours, tout revient à « combinaison automatique - sans conflit ✓ ». Je n'ai pas trouvé de moyen de changer de section pour résoudre le conflit autrement qu'en retirant un cours : les blocs pointillés annoncés par la légende, je n'en ai pas rencontré dans ce cas-là.

## Ce que je n'ai pas pu tester
- **Marquer un cours comme réussi** : aucun contrôle de ce nom dans l'interface (le plan indique que la notion a été retirée volontairement, un cours réussi se déclare en le plaçant dans sa session passée) — donc rien à signaler, mais je n'ai pas pu faire ce test.
- **Épingler / dépingler un cours à une session** : je n'ai trouvé aucun bouton « épingler ». Ajouter un cours à la session affichée semble jouer ce rôle, mais rien ne me le dit.
- **Ouvrir un lien de partage** : le fragment `#gWNkddJ…` est bien écrit dans la barre d'adresse, mais je n'ai pas pu coller le lien copié dans une autre fenêtre pour vérifier qu'il rouvre le même organigramme.
- **Les préférences** (journées compactes, matins libres) : rien à ce sujet dans l'interface — c'est prévu pour le dernier jalon, je ne le compte pas comme un défaut.

## Impression générale

Oui, je m'en servirais — c'est déjà nettement mieux que de recopier l'organigramme papier dans un tableur. En vingt minutes j'ai eu un bac complet réparti sur huit sessions, avec les préalables respectés, le stage placé, la grille hebdomadaire de chaque session et une liste honnête de ce qu'il me reste à choisir. Les messages sont écrits dans ma langue, pas dans celle des programmeurs, et l'application me laisse faire des bêtises tout en me disant lesquelles ; c'est exactement le bon équilibre.

Trois choses m'empêcheraient de m'y fier complètement. La première, et de loin la pire : j'ai vu une fois une dizaine de manipulations disparaître après un simple rechargement, sans avertissement. Tant que je ne suis pas certaine que ça ne peut pas arriver, je garderai une copie ailleurs. La deuxième : la case des étés ne tient pas sa promesse — je peux mettre des cours l'été alors que je l'ai fermée, et l'application me dit quand même « Placement vérifié ✓ », donc je ne peux pas prendre ce ✓ au pied de la lettre. La troisième : si je décoche la scolarité préparatoire parce que j'ai vraiment ces cours à faire, l'outil ne sait plus me proposer d'organigramme et me renvoie en boucle vers un bouton qui ne change rien — or c'est justement la situation de plusieurs de mes camarades admis avec des cours d'appoint.
