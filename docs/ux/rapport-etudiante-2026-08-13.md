# Rapport d'exploration — Camille, 2e année, bac en génie des eaux (GEX)

**Date** : 2026-08-13
**Contexte** : session vierge (`localStorage` vidé), navigateur piloté sur `http://localhost:8000`.
**Captures** : `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/77509ff8-8e54-4de7-82f6-e6b06a120e43/scratchpad/`

---

### L'interface cesse de répondre au bout de quelques manipulations ; seul un rechargement la débloque
- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** :
  1. Choisir B-GEX, cliquer « Proposer un organigramme ».
  2. Ouvrir la session A1·A26 (onglet de gauche), retirer un cours avec ✕ — ça marche (deux fois de suite chez moi : BIO-0150, puis CHM-0150).
  3. Continuer à cliquer les ✕ et ✓ des cours suivants (CHM-0160, CHM-0170), puis « Annuler », puis l'onglet « Bac complet ».
  4. Recharger la page, puis recliquer les mêmes boutons.
- **Attendu** : chaque clic fait quelque chose, comme les deux premiers.
- **Observé** : à partir d'un certain moment, plus rien ne bouge. Le ✕ ne retire plus aucun cours, le ✓ ne marque plus rien, « Annuler » reste sans effet et « Rétablir » reste grisé, l'onglet « Bac complet » refuse de s'activer même après 3 clics d'affilée. Le compteur reste figé (« 18 cr cette session »). Seules les tuiles du ruban continuent de répondre. Après un rechargement (F5), les mêmes boutons refonctionnent immédiatement (l'onglet bascule, « Annuler » redevient actif). C'est le constat qui a saboté toute ma session : je n'ai jamais pu savoir si un clic allait compter ou non. (Captures : `15-annuler.png`, `17-onglet-bloque.png` — aucune erreur console, `agent-browser errors` est resté muet tout du long.)

---

### Impossible de retirer un cours que je viens d'ajouter par code
- **Gravité** : bloquant
- **Type** : bogue
- **Reproduction** :
  1. Dans l'onglet de la session A1·A26, ajouter ALL-1010 avec le champ « Ajouter par code… », puis ACT-1000, puis ADM-2000.
  2. Dans la liste « Dans cette session », cliquer le ✕ de ADM-2000. Recliquer. Cliquer celui d'ACT-1000. Celui d'ALL-1010.
  3. Recharger la page et recommencer.
- **Attendu** : le cours disparaît de la session et le total de crédits baisse.
- **Observé** : aucun des trois ne part. Le total reste à 24 cr, la tuile A1·A26 les liste toujours, aucun message. Comme j'avais volontairement créé un conflit avec ces deux cours, je me suis retrouvée coincée avec un conflit que je ne pouvais plus défaire — ni par ✕, ni par « Annuler » (voir plus bas), ni en changeant de section (ACT-1000 n'a qu'une seule plage, cliquer son bloc n'offre aucune alternative). (Captures : `13b-conflit.png`, `14-retrait.png`, erreur console : aucune.)

---

### Un cours d'hiver ajouté à une session d'automne est accepté sans un mot, et compté deux fois
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Ouvrir la session A1·A26 (Automne 2026).
  2. Ajouter DRT-1721 par code (ce cours est marqué « offert H » dans la liste des obligatoires).
- **Attendu** : un refus, ou au minimum « ce cours n'est pas offert à l'automne ».
- **Observé** : il est accepté sans message, la session passe à 21 cr, et il apparaît à la fois dans la tuile A1·A26 **et** dans la tuile H6·H29 où le solveur l'avait placé. Le panneau de gauche dit « DRT-1721 · 3 cr · placé en H6·H29 » pendant que la tuile d'automne le compte quand même. Il n'apparaît nulle part dans la grille horaire de l'automne, sans explication. Ce n'est qu'au **deuxième** clic sur « Proposer un organigramme » (le premier ne produit rien) qu'un message finit par sortir : « ⚠ DRT-1721 : aucune session de l'horizon ne peut l'accueillir (offre et épinglages ne laissent rien). » — affiché deux fois de suite, et dans un vocabulaire (« horizon », « épinglages ») que je ne comprends pas. (Captures : `16-cours-hiver-automne.png`, `18-alertes-double.png`.)

---

### Un cours dont je n'ai visiblement pas les préalables entre sans avertissement
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Dans la session A1·A26 (ma toute première session, où je n'ai encore rien fait), ajouter GCI-2008 par code. Ce cours est affiché « préalables non remplis » dans la liste des obligatoires, où il n'a même pas de bouton « + ».
- **Attendu** : un refus ou un avertissement « il vous manque tel préalable ».
- **Observé** : accepté en silence, session à 24 cr, et le cours apparaît en double (A1·A26 et A5·A28). Le message que j'espérais — quel préalable me manque — n'existe nulle part : la liste dit seulement « préalables non remplis », jamais lesquels. (Capture : `17-onglet-bloque.png`.)

---

### Les mêmes séances apparaissent en double (ou en quadruple) dans la grille, et se déclarent en conflit avec elles-mêmes
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Proposer un organigramme, ouvrir A1·A26 : regarder le lundi.
  2. Ouvrir H2·H27 : regarder le mercredi et le jeudi.
- **Attendu** : une séance = un bloc.
- **Observé** : « Biologie générale BIO-0150 · ZA · hybride » est dessiné **deux fois** côte à côte au même créneau du lundi ; « Hydrogéologie GGL-2600 · ZA · hybride » est dessiné **quatre fois** (deux le mercredi, deux le jeudi). Pire : quand un conflit survient dans la session, ces doublons sont hachurés et marqués « ⚠ conflit » — le cours est donc en conflit avec lui-même, ce qui me fait chercher un problème inexistant. (Captures : `05-horaire-a1.png`, `13b-conflit.png`, `19-lien-partage.png`.)

---

### Deux fois exactement le même baccalauréat en génie des eaux dans la liste des programmes
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Onglet « Bac complet » sur une session vierge.
  2. Faire défiler la liste des programmes.
- **Attendu** : une entrée par programme.
- **Observé** : deux entrées rigoureusement identiques, « Baccalauréat en génie des eaux — B-GEX · version A26 · 120 cr », l'une sous l'autre. Rien ne les distingue, donc je ne sais pas laquelle est la bonne ni ce que je risque en choisissant la mauvaise. (Capture : `02-liste-programmes.png`.)

---

### « Scolarité préparatoire » et « Règle 5 » ne s'ouvrent pas : je ne peux pas voir ni cocher leurs cours
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Choisir B-GEX, onglet « Bac complet ».
  2. Cliquer la ligne « Scolarité préparatoire — ▸ ». Recliquer. Attendre. Recliquer une troisième fois.
  3. Faire pareil avec « Règle 5 · 3 cr — ▸ ».
- **Attendu** : la liste des cours s'ouvre comme pour « Obligatoires » et « Règle 1 », avec les boutons ✓ pour marquer mes cours réussis.
- **Observé** : rien, jamais ; la flèche reste ▸. « Obligatoires » et « Règle 1 » s'ouvrent parfaitement au premier clic dans le même écran, donc ce n'est pas mon clic. Ce sont précisément les deux règles dont le compteur affiche « — » au lieu d'un x/y. Résultat : je n'ai **pas pu** faire ce pour quoi j'étais venue, cocher ma scolarité préparatoire depuis sa règle. J'ai contourné en cherchant MAT-0150, MAT-0260 et PHY-0150 un par un dans la recherche du catalogue et en cliquant leur ✓ — ça, ça marche bien. Mais il a fallu que je devine les codes : je les ai lus par hasard dans la mention « présumé acquis : MAT-0150, MAT-0260, PHY-0150 » sous GCI-1900. (Capture : `03-programme-choisi.png`.)

---

### « Cheminement vérifié ✓ » alors que cinq règles du programme sont à zéro
- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Choisir B-GEX, « Proposer un organigramme », puis « Vérifier le cheminement ».
  2. Comparer le message vert au bloc des règles juste en dessous.
- **Attendu** : soit le cheminement est complet, soit on me dit ce qui manque pour diplômer.
- **Observé** : « Cheminement vérifié ✓ — préalables, plafond et horaires respectés. » en vert, pendant que Règle 1 (0/1), Règle 2 (0/3 cr), Règle 3 (0/3 cr) et Règle 4 (0/3 cr) sont encadrées en rouge juste dessous. Comme étudiante, je ne sais pas conclure : est-ce que je diplôme avec ça, oui ou non ? L'organigramme proposé ne place spontanément aucun cours pour ces règles, et rien ne me dit qu'il faut que je les choisisse moi-même. Aucun total « X / 120 crédits » n'est affiché nulle part pour m'aider à trancher. (Captures : `06-verifier.png`, `07-verifier2.png`.)

---

### Les règles n'ont pas de nom : « Règle 1 », « Règle 2 »… je ne sais pas de quoi elles parlent
- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Choisir B-GEX, regarder le bloc des règles.
- **Attendu** : « Règle 2 · concentration en hydraulique », « Règle 4 · cours de sciences humaines », quelque chose qui me dise quoi choisir.
- **Observé** : « Règle 1 · 1 parmi », « Règle 2 · 3–9 cr », « Règle 3 · 3–9 cr », « Règle 4 · 3 cr », « Règle 5 · 3 cr ». Il faut ouvrir chaque accordéon et déduire le thème de la liste de cours (Règle 1 = santé et sécurité, j'ai deviné). Pour les deux qui ne s'ouvrent pas, je ne peux même pas deviner. C'est exactement l'information que je viens chercher dans un planificateur.

---

### Dépasser le plafond de crédits ne déclenche aucun signal
- **Gravité** : majeur
- **Type** : bogue
- **Reproduction** :
  1. Laisser « Plafond (cr) » à 15.
  2. Ajouter ALL-1010, ACT-1000, ADM-2000 puis DRT-1721 et GCI-2008 à la session A1·A26.
- **Attendu** : que le « 24 » vire au rouge, ou un message « vous dépassez le plafond que vous avez fixé (15 cr) ».
- **Observé** : « 24 cr cette session » s'affiche du même gris que « 15 cr », la tuile A1·A26 affiche « 24 » sans aucune marque, et le champ « Plafond (cr) » à 15 juste à côté n'a aucun effet visible. Rien ne me dit que je suis en surcharge. (Capture : `11-surcharge.png` à 18 cr, `18-alertes-double.png` à 24 cr.)

---

### En cas de conflit, les blocs deviennent illisibles — juste au moment où j'ai besoin de lire
- **Gravité** : majeur
- **Type** : friction
- **Reproduction** :
  1. Ajouter ACT-1000 et ADM-2000 dans la même session (tous deux mardi 15:30).
  2. Regarder la colonne du mardi.
- **Attendu** : pouvoir identifier quels cours se chevauchent.
- **Observé** : cinq blocs compressés côte à côte, titres coupés en morceaux : « Bi… gé… », « Al… ve… », « Él… de chi… gé… et des », « Int… à l'a… I », « En… en ge… : pont en… ». Les sigles restent lisibles (heureusement), mais je dois deviner. Le signalement en lui-même est bon : bandeau « ⚠ conflit d'horaire — plages en cause hachurées », les jours fautifs marqués ⚠ dans l'entête, hachures rouges — c'est clair et je l'ai compris tout de suite. (Capture : `13b-conflit.png`.)

---

### Un cours seul dans sa journée est marqué « en conflit »
- **Gravité** : mineur
- **Type** : bogue
- **Reproduction** :
  1. Provoquer le conflit ACT-1000 / ADM-2000 le mardi dans A1·A26.
  2. Regarder la colonne du samedi.
- **Attendu** : le samedi n'est pas concerné.
- **Observé** : « Éléments de chimie générale et des solutions · CHM-0170 · A ⚠ conflit », hachuré en rouge, seul bloc de la journée de 9:30 à 11:30, et « Samedi ⚠ » dans l'entête. Rien ne le chevauche. (Capture : `13b-conflit.png`.)

---

### Le premier clic sur une tuile du ruban est parfois avalé
- **Gravité** : mineur
- **Type** : bogue
- **Reproduction** :
  1. Après « Vérifier le cheminement », cliquer la tuile « H2·H27 » du ruban.
  2. Recliquer. Recliquer encore (j'ai aussi essayé en visant le texte plutôt que la tuile).
- **Attendu** : l'horaire de H2 s'affiche.
- **Observé** : l'entête reste « Horaire · A1·A26 — Automne 2026 » et A1 reste encadré en rouge, trois clics de suite. Après avoir touché l'onglet de gauche, la même tuile a fonctionné du premier coup, et toutes les suivantes aussi. Impossible de savoir quand ça va répondre. (Capture : `08-clic-h2.png`.)

---

### Les mêmes messages d'alerte s'empilent en double
- **Gravité** : mineur
- **Type** : bogue
- **Reproduction** :
  1. Ajouter DRT-1721 à la session d'automne, puis cliquer deux fois « Proposer un organigramme ».
- **Attendu** : un message.
- **Observé** : « ⚠ DRT-1721 : aucune session de l'horizon ne peut l'accueillir » affiché deux fois, l'un sous l'autre, avec chacun sa croix de fermeture. (Capture : `18-alertes-double.png`.)

---

### Des messages écrits pour un développeur, pas pour moi
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Choisir B-GEX, « Proposer un organigramme », lire le bandeau d'alertes.
- **Attendu** : des phrases qui me disent quoi faire.
- **Observé** : « ⚠ Nombre maximal de solutions atteint : d'autres agencements existent peut-être. » — je ne sais pas si c'est grave ni ce que je devrais faire. « ⚠ Présumé acquis : Examen Chimie générale avec résultat de N à P — vérifiez vous-même. » — « de N à P » ne veut rien dire pour moi (des notes ? des lettres ?), et « vérifiez vous-même » ne me dit pas quoi vérifier ni où. Ajoutons « horizon » et « épinglages » dans le message de DRT-1721. (Capture : `04-organigramme.png`.)

---

### Un cours affiche « + » et, juste à côté, « aucune session ne peut l'accueillir »
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Chercher MAT-0150 dans le catalogue (programme choisi, rien encore placé).
  2. Cliquer le bouton « sessions ? ».
- **Attendu** : la liste des sessions où le cours pourrait aller.
- **Observé** : le bouton se transforme en texte « aucune session ne peut l'accueillir », alors que la ligne juste au-dessus dit « MAT-0150 · 4 cr · offert A·H·É » et que le bouton « + » est bien là, cliquable. Deux affirmations contradictoires côte à côte. Par ailleurs le libellé « sessions ? » ne m'annonce pas ce qu'il va faire.

---

### Le message « aucun cours » s'affiche alors que la session contient mon stage
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Proposer un organigramme, ouvrir la tuile É27 (qui contient GEX-1580).
- **Attendu** : cohérence.
- **Observé** : « Aucun cours avec horaire publié pour cette session. Ajoutez des cours par code dans le panneau de gauche. » suivi, deux lignes plus bas, de « GEX-1580 — sans plage hebdomadaire (à distance) : suivi hors grille. » Cette seconde phrase, elle, est parfaitement claire et m'a rassurée. La première m'a d'abord fait croire que mon stage avait disparu. (Capture : `09-ete27.png`.)

---

### Les tuiles d'été sont des bandes de 15 pixels avec du texte tourné
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Regarder le ruban des sessions.
- **Attendu** : pouvoir lire et viser mes sessions d'été comme les autres.
- **Observé** : « É27 », « É28 », « É29 », « É30 » sont écrits verticalement dans des colonnes minuscules entre les sessions régulières, alors que les autres tuiles font 130 px de large. Quand É27 contient mon stage, il faut incliner la tête pour lire « É27 · GEX-1580 ». (Capture : `01-accueil.png`.)

---

### « Ouvrir les étés » n'a rien changé pour moi
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Onglet « Bac complet », cocher « Ouvrir les étés aux cours réguliers ».
  2. Cliquer « Proposer un organigramme ».
- **Attendu** : quelques cours migrent vers É28/É29 et mes sessions d'hiver/automne s'allègent.
- **Observé** : É28, É29 et É30 restent à « — » et la répartition ne bouge pas d'un cours. Rien ne me dit si c'est normal (mon cheminement était déjà complet et le solveur ne recommence pas) ou si la case n'a pas d'effet. Je note honnêtement que mon état était déjà pollué par DRT-1721 et GCI-2008 mal placés à ce moment-là — je n'ai pas pu refaire l'essai proprement, puisque je n'arrivais plus à les retirer. (Capture : `18-alertes-double.png`.)

---

### « Rétablir » redevient gris et j'ai perdu tout mon historique après le rechargement
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Faire une dizaine de manipulations, recharger la page (F5).
  2. Cliquer « Annuler » quatre ou cinq fois.
- **Attendu** : remonter le fil de mes actions, au moins les plus récentes.
- **Observé** : le rechargement conserve parfaitement mon travail (programme, organigramme, cours ajoutés, section forcée, session ouverte — c'est très bien) mais l'historique repart de zéro : après quelques clics « Annuler » se grise, et mes ajouts d'avant le rechargement (DRT-1721, GCI-2008) sont définitivement là. Combiné au ✕ qui ne les retire pas, je n'ai plus aucun moyen de nettoyer.

---

### « Partager » annonce un succès avec un pictogramme d'avertissement
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Ouvrir une session, cliquer « Partager ».
- **Attendu** : « Lien copié ».
- **Observé** : « ⚠ Lien copié — il rouvre cet horaire tel quel : http://localhost:8000/?h=h2027.GGL-2600%3A14871%2C… » — le ⚠ orange m'a fait croire à une erreur, et l'URL entière (avec ses `%3A`) reste affichée en travers de l'écran jusqu'à ce que je la ferme à la main. Sur le fond, ça marche très bien : j'ai ouvert le lien et j'ai eu « Horaire partagé importé dans H2·H27. » avec la bonne grille, sans perdre le reste de mon cheminement. (Capture : `19-lien-partage.png`.)

---

### « combinaison automatique » reste affiché après que j'aie forcé une section à la main
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Ajouter ALL-1010, cliquer son bloc, puis cliquer la plage pointillée du mercredi (section Q).
- **Attendu** : l'entête reflète que j'ai fait un choix.
- **Observé** : l'entête dit toujours « combinaison automatique · sans conflit ✓ ». Seul le panneau de gauche mentionne « ALL-1010 · 3 cr · section forcée » avec un bouton ⇄ pour revenir en arrière. Cela dit, ce mécanisme est le meilleur de l'application : la légende « Plein = plage retenue · pointillé = autre plage possible (cliquer pour la forcer) · hachuré = conflit » est limpide, et voir les quatre sections d'ALL-1010 apparaître en pointillé d'un seul clic est exactement ce que j'espérais. (Captures : `12-plages-alt.png`.)

---

### Le pied de page dit « date de récolte inconnue »
- **Gravité** : mineur
- **Type** : friction
- **Reproduction** :
  1. Descendre en bas de page.
- **Attendu** : savoir de quand datent les horaires que je consulte.
- **Observé** : « v0.1.0 · build d3ad104 · 8834 cours · données : date de récolte inconnue · empreinte 221bd58fec17f0ae ». Sans date, je ne sais pas si l'horaire affiché est celui publié ou une vieille copie — et pour un cours d'automne 2026 planifié aujourd'hui, c'est la question que je me pose en premier. (Le cron de mise à jour est prévu au jalon 5 ; l'empreinte hexadécimale, elle, ne me sert à rien.)

---

### Ce que je n'ai pas vu et qui est prévu plus tard
- **Type** : pas encore construit
- Aucun réglage de préférences (journées compactes, matins libres, pause dîner) nulle part — c'est le jalon 10 du plan.
- Le formulaire « Cours absent du catalogue ? » existe (code, titre, crédits, NRC, heures) mais je ne l'ai pas essayé, faute d'actions restantes ; il correspond au « cours manuel » du jalon 10.
- La fraîcheur des données (cron CI) est le jalon 5.

### Tests que je n'ai pas pu faire
- **Épingler un cours à une session précise puis le dépingler** : je n'ai trouvé aucun bouton d'épinglage dans l'interface. Le mot « épinglages » apparaît dans un message d'erreur, mais je n'ai jamais vu où on épingle. J'ai seulement pu forcer une *section* horaire (bouton ⇄), ce qui n'est pas la même chose.
- **Résoudre le conflit en changeant de section** : ACT-1000 n'offre qu'une seule plage (cliquer son bloc n'affiche rien d'autre) et je n'ai pas pu retirer l'un des deux cours — donc le conflit est resté insoluble jusqu'à la fin.
- **Annuler un marquage « réussi »** : le bouton ↩ apparaît bien après avoir coché MAT-0150 dans la recherche, mais je ne l'ai pas cliqué avant que l'interface ne se bloque.
- **Ouvrir puis refermer l'été proprement** : décoché puis recoché via « Annuler »/« Rétablir », mais sans regénération concluante (voir plus haut).

---

## Impression générale

Non, je ne planifierais pas ma session avec ça aujourd'hui — pas parce que l'outil est mauvais, mais parce que je ne peux pas lui faire confiance sur mes propres gestes. Le fond est vraiment prometteur : choisir B-GEX et voir en une fraction de seconde mes huit sessions se remplir avec le stage à l'été, c'est impressionnant ; la grille hebdomadaire est belle et lisible ; le système de plages pointillées cliquables pour changer de section est la meilleure idée de l'interface ; le signalement de conflit (bandeau, jours marqués, hachures) est immédiatement compréhensible ; le lien de partage marche du premier coup ; et rien n'est perdu quand je recharge.

Ce qui m'arrête, c'est que la moitié de mes clics ne produisent rien et que je ne peux pas le prévoir. J'ajoute un cours, il rentre ; je veux l'enlever, il reste. Je clique une session du ruban, elle ne s'ouvre pas ; je reclique plus tard, elle s'ouvre. Je clique « Annuler », rien. J'essaie d'ouvrir « Scolarité préparatoire » — la règle même pour laquelle je suis venue, moi qui ai des cours de mise à niveau à déclarer — et elle ne s'ouvre jamais. Un planificateur où l'on ne peut pas défaire ses essais n'est pas un planificateur : je n'ose plus rien essayer, de peur de salir un état que je ne pourrai pas nettoyer autrement qu'en vidant tout.

Deuxième chose qui m'arrêterait même si tout répondait : l'outil accepte n'importe quoi sans me prévenir. Un cours d'hiver à l'automne, un cours dont je n'ai pas les préalables, 24 crédits sous un plafond de 15 — tout passe en silence. Et il me dit « Cheminement vérifié ✓ » pendant que quatre règles du programme sont à zéro juste en dessous. Si je m'y fiais, je m'inscrirais à des cours impossibles en croyant mon bac bouclé. C'est plus dangereux qu'utile.

Si je devais prioriser : rendre les boutons fiables (le ✕ d'abord), ouvrir les deux règles muettes, refuser — ou au moins signaler — les ajouts impossibles, et donner un nom aux règles avec un total « X / 120 crédits ». Avec ça, je reviendrais volontiers : le reste est déjà mieux que ce que j'utilise aujourd'hui (un PDF d'organigramme et le Capsule de l'Université).
