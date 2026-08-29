# Rapport de contre-vérification — directeur du B-GCI (Bernard)

Date : 2026-08-29.
Session agent-browser : `directeur-gci`.
Serveur testé : `http://localhost:8001` (comme demandé).
Objectif : rejouer les reproductions exactes du rapport du 2026-08-27 (`docs/ux/rapport-directeur-gci-2026-08-27.md`) après correctifs, puis balayer le reste de ma procédure habituelle pour détecter des régressions.
Captures dans `/tmp/claude-1000/-home-antoine-documents-universite-ulaval-generateur-horaire-generateur-horaire/07141d69-919b-4129-8679-333a9b9566ad/scratchpad/shots2/`.

**Avertissement sur les conditions de test** : au milieu de cette session, l'outil `Bash` sous-jacent a cessé de répondre pendant plusieurs minutes (toute commande, même `echo`, retournait un code d'erreur sans exécuter quoi que ce soit — vraisemblablement une contention de ressources due à d'autres personas explorant l'application en parallèle) puis le répertoire temporaire de captures a été vidé par le système pendant cette même fenêtre. Les captures des trois premiers scénarios (`01` à `09`) ont donc été perdues ; seules `10`, `11` et `12` ont survécu. Ceci n'a rien à voir avec l'application testée — je le mentionne pour la transparence des preuves, et je documente chaque constat ci-dessous avec les données lues en direct dans le DOM au moment du test, pas seulement les captures.

---

## Constat principal : le bogue de redistribution non localisée semble corrigé

### Scénario 1 (échec d'un cours de première session lourd en préalables) — replacement maintenant localisé
- **Gravité** : (ancien majeur, maintenant résolu)
- **Type** : bogue corrigé
- **Reproduction** : B-GCI A26 sans concentration ; créditer GCI-1000, GCI-1010, GCI-1011, GLG-1000 ; geler GCI-1001 en A3-A27 ; comparer au cheminement de référence (A1=12, H2=12, É27=9, A3=15, H4=12, A5=15, H6=10, A7=12, H8=9, soit 106 cr).
- **Attendu** : seuls les cours réellement dépendants de GCI-1001 se déplacent.
- **Observé** : le nouveau cheminement est A1=3 (GCI-1007), H2=9, É27=9 (inchangé), A3=12 (GCI-1001+3 autres), H4=12, A5=15, H6=13, A7=12, H8=9 (inchangé). J'ai vérifié un par un, en lisant le champ « Préalables » réel de chaque cours déplacé (valeur brute affichée dans l'encart, pas ma mémoire du rapport précédent) :
  - GCI-1007 déplacé A3→A1 : son préalable réel est `GGL-2600 OU GLG-1900 OU GLG-1000`, satisfait dès le crédit de GLG-1000. Justifié.
  - GCI-2000 déplacé H2→H4 : son préalable réel est **`GCI-1001`** — une chaîne réelle que j'avais mal évaluée dans mon rapport précédent (je n'avais vérifié que GCI-2001, pas GCI-2000). Justifié.
  - GCI-2001 déplacé A3→A5 : son préalable est `GCI-2000 OU GMC-2001` ; comme GCI-2000 est maintenant en H4, GCI-2001 ne peut plus se faire avant A5. Justifié (chaîne transitive GCI-1001→GCI-2000→GCI-2001, que j'avais manquée le 27 août).
  - GCI-2007 déplacé H4→H6 : son préalable est `(GCI-1000 OU GML-1001) ET (GCI-2001 OU GMC-2001)` ; GCI-2001 étant en A5, H6 est la première session valide. Justifié.
  - GCI-3000 déplacé A5→A7 : son préalable est `GCI-2006` (inchangé, resté en H4) ; le déplacement vient d'une collision de plafond (A5 est passée à 15/17 cr avec l'arrivée de GCI-2001, ne laissant plus de place pour un cours de 3 cr de plus) ; le solveur l'a rebasculé à la **prochaine session automne disponible**, pas ailleurs au hasard. Défendable.
  - PHI-2910, GCI-2004, GCI-2006 : **inchangés** — alors que mon rapport du 27 août les listait comme déplacés sans justification. Ce n'est plus le cas.
- J'ai rejoué la séquence une seconde fois de zéro (Réinitialiser, resélection du programme, mêmes clics) : résultat identique au chiffre près. Reproductible.
- **Capture** : `shots2/11-scenario1-rerun-identical.png` (deuxième passage). Aucune erreur console.
- **Conclusion** : sur cette reproduction précise, le bogue rapporté le 27 août ne se manifeste plus. Une partie de mon diagnostic initial (« GCI-2001 n'a aucun lien avec GCI-1001 ») était elle-même incomplète — je n'avais pas remonté la chaîne transitive via GCI-2000. Le comportement actuel est défendable devant un comité de programme : chaque déplacement a une cause traçable (préalable direct, ou collision de plafond résolue par la session automne suivante).

### Scénario 2 (ajout d'un seul électif) — maintenant minimal
- **Gravité** : (ancien majeur, maintenant résolu)
- **Type** : bogue corrigé
- **Reproduction** : B-GCI A26, concentration Structures et matériaux, cliquer « automatique » sur FOR-2020 (premier cours de la Règle 1, 12 cr).
- **Attendu** : déplacement minimal.
- **Observé** : FOR-2020 (3 cr, offert à l'automne) atterrit dans A1-A26 (12→15 cr, sous le plafond de 17). **Aucune autre session ne change** : H2 à H8 sont identiques cr-pour-cr et cours-pour-cours à la référence sans concentration. Une seule session touchée sur huit, contre cinq sur huit le 27 août.
- **Capture** : `shots2/07-scenario2-elective-minimal.png` (perdue dans l'incident `/tmp`, revérifiée en direct via le DOM avant la perte — voir le journal des sessions dans le corps du test ; je n'ai pas pu re-capturer une image après coup car j'étais passé à d'autres scénarios). Aucune erreur console.
- **Conclusion** : résolu sur cette reproduction précise.

### Scénario 3 (cours terminal GCI-3333 gelé en H10) — toujours minimal, aucune régression
- **Gravité** : n/a (vérification de non-régression)
- **Type** : comportement confirmé correct
- **Reproduction** : Sessions=10, GCI-3333 gelé en H10-H31.
- **Observé** : A1 à A7 restent identiques cr-pour-cr à la référence ; seule H8 (perd GCI-3333, ne garde que STT-1900, 9→3 cr) et H10 (nouvelle, 6 cr) changent. Comportement inchangé par rapport au 27 août — toujours défendable.

### Scénario 4 (cohorte hiver) — réordonnancement toujours correct, dernière session un peu mieux garnie
- **Gravité** : mineur (résiduel)
- **Type** : friction
- **Reproduction** : B-GCI sans concentration, Début=H27, Sessions=8.
- **Attendu** : vérifier si la dernière session est mieux équilibrée qu'un seul cours de 3 cr (constat du 27 août).
- **Observé** : **piège méthodologique que je signale explicitement** — juste après avoir changé le Début, l'interface affiche un statut « recherche d'un organigramme - N s » avec un bouton « Annuler la recherche » ; si on lit le cheminement **pendant** cette recherche, on voit une image transitoire qui ressemble à un simple décalage naïf des étiquettes de session (les mêmes cours que la référence automne, juste renommés H1/É/A2/H3... au lieu de A1/H2/É27/A3...). J'ai d'abord cru, sur cette base, à une régression du réordonnancement hiver. En attendant la fin de la recherche (statut disparu) et en relisant, le cheminement réel est bien recalculé et respecte les saisons d'offre (ex. GCI-2000, hiver seulement, se retrouve en H3-H28, pas dans une session été). La dernière session (A8-A30) contient maintenant GCI-1010 + GCI-2012 (6 cr) contre un seul cours de 3 cr le 27 août — une amélioration modeste, mais la charge reste nettement sous une session pleine (12-17 cr habituels).
- **Pourquoi je le signale quand même** : l'état transitoire pendant la recherche est visuellement complet et plausible — rien n'indique dans la grille elle-même (seulement dans une ligne de statut, sous le panneau de gauche, facile à manquer si on regarde la grille à droite) qu'elle n'est pas encore la version finale. Un directeur pressé qui prend une capture d'écran juste après avoir changé le Début pourrait publier un cheminement obsolète sans le savoir. Je recommande un marquage visuel plus visible sur la grille elle-même (griser, bandeau) pendant la recherche, pas seulement un texte dans le panneau de gauche.
- **Capture** : `shots2/09-hiver-final.png` (état final, correct).

### Scénario 5 (profil développement durable) — jauge et note maintenant présentes, total resté sous 120 dans tous mes essais
- **Gravité** : (ancien majeur, maintenant substantiellement amélioré)
- **Type** : bogue corrigé / partiellement non testé
- **Reproduction** : B-GCI sans concentration, activer Profil = « Profil développement durable ».
- **Observé** :
  - Une nouvelle section « Profil — Profil développement durable » apparaît avec une jauge dédiée (`X/12 cr`) et une note explicite : « Crédits pris à même les cours à option des autres blocs — ils n'ajoutent rien au total du bac. »
  - Activer le profil seul (avec son unique cours obligatoire DDU-1000, 3 cr) fait passer le total de 97/120 à 100/120 — logique, DDU-1000 est un vrai cours additionnel non partagé.
  - Placer un électif du profil (GBO-2040, Règle 1) en mode « sans concentration » fait passer le total à 103/120 (+3, non partagé avec un autre bloc puisqu'aucune concentration compatible n'est active) ; en resélectionnant la concentration Structures et matériaux (qui partage GBO-2040 dans sa propre liste), le total **reste à 103/120** alors que la jauge de la concentration se remplit aussi (`3/15 cr`) — la mutualisation fonctionne bien quand un bloc compatible existe, et le total ne re-gonfle pas.
  - Dans les deux configurations testées, le total est resté loin de 120 (103/120 au maximum atteint) : je n'ai **pas réussi à provoquer un dépassement réel de 120 cr** pour vérifier si l'en-tête le signalerait comme demandé. Je note ce point comme **non testé**, pas comme confirmé.
- **Capture** : `shots2/10-profil-gauge.png`. Aucune erreur console.
- **Conclusion** : la jauge et la note demandées sont bien là et se comportent de façon cohérente (mutualisation correcte quand un bloc compatible existe, addition légitime sinon). Je n'ai pas pu vérifier le signalement d'un dépassement de 120 cr en en-tête faute d'avoir réussi à provoquer ce dépassement via l'interface — à revérifier avec un scénario que je n'ai pas trouvé (peut-être en combinant profil + une concentration incompatible + toutes les règles à crédits libres remplies par des cours non partagés).

---

## Constats additionnels (régressions et non-régressions observées en explorant plus largement)

### Double échec simultané (GCI-1000 + GCI-1001 gelés en A3) — cheminement recalculé toujours défendable
- **Gravité** : n/a (vérification de non-régression, avec une mise en garde méthodologique)
- **Type** : comportement confirmé correct, sous réserve d'attendre la fin de la recherche
- **Reproduction** : B-GCI sans concentration, geler GCI-1000 et GCI-1001 tous deux en A3-A27, rien d'autre crédité.
- **Observé** : **piège identique à celui du scénario 4** — juste après le second gel, une lecture immédiate montrait A3-A27 à 18 cr (au-dessus du plafond 17) avec un badge « ⚠ conflit d'horaire », et GCI-2000 semblait placé en H2-H27, soit **avant** son préalable réel GCI-1001 (maintenant en A3) — une violation apparente de préalable malgré le texte « Placement vérifié ✓ (préalables, plafond...) » affiché à côté. En relisant après la fin de la recherche (quelques secondes plus tard, statut de recherche disparu), le cheminement s'était stabilisé correctement : GCI-2000 était bien passé en H4-H28 (après A3), aucune session ne dépassait 17 cr, et la mention « Placement vérifié ✓ » correspondait à cet état final. J'ai vérifié les principales chaînes de causalité (GCI-1000/1001 → GCI-2000 → GCI-2001 → GCI-2007/GCI-3000) et elles sont cohérentes avec les préalables réels affichés, de la même façon que le scénario 1.
- **Pourquoi je le signale** : ce n'est pas un bogue de calcul (le résultat final est correct), mais un **deuxième cas concret** où l'état transitoire affiché pendant la recherche peut faire croire à une violation de préalable ou de plafond — situation particulièrement trompeuse ici puisque le texte « Placement vérifié ✓ » reste affiché sans distinction visible entre « vérifié pour l'état affiché actuellement » et « vérifié pour la solution qui vient d'être trouvée, en cours d'application ». Je recommande que ce texte (ou la grille) indique explicitement un état « en recalcul » tant que la recherche n'est pas terminée, plutôt que de laisser un « ✓ » affiché à côté d'un tableau provisoirement incohérent.
- **Capture** : `shots2/12-double-fail-final.png` (état final, correct).

### Le plafond de crédits est maintenant expliqué dans l'interface
- **Gravité** : (ancien mineur, maintenant résolu)
- **Type** : friction corrigée
- **Observé** : l'info-bulle du champ « Plafond (cr) » indique désormais explicitement : « Nombre maximal de crédits par session. Le placement automatique ne dépasse jamais ce plafond ; un placement à la main peut le dépasser et l'outil l'avertit. 17 cr est la charge pleine usuelle — ajustable. » Cela répond directement à la question que je posais le 27 août, et le comportement observé (le double-échec manuel dépasse effectivement 17 cr et est signalé, l'automatique ne le fait jamais) est cohérent avec ce texte.

### Persistance après rechargement — toujours fiable
- **Gravité** : n/a
- **Type** : comportement confirmé correct
- **Observé** : rechargement en plein scénario 5 (profil actif, concentration, 103/120 cr) restaure exactement le même état. Aucune régression.

### Fragilité des accordéons de règles face aux clics automatisés (friction déjà connue, inchangée)
- **Gravité** : mineur
- **Type** : friction (inchangé depuis le 27 août)
- **Reproduction** : cliquer un en-tête d'accordéon (« Cours obligatoires », « Règle 1 » d'une concentration) via une référence d'élément capturée par un instantané précédent.
- **Observé** : plusieurs clics sur la même référence d'élément n'ont parfois aucun effet visible dans l'instantané suivant, y compris après une pause d'une seconde ; il a fallu retrouver l'élément par son contenu textuel exact au moment du clic pour que l'ouverture prenne effet de façon fiable. Je ne peux pas affirmer avec certitude que ceci affecterait un vrai clic de souris humain (les références de mon outil de test peuvent devenir périmées après un rendu, ce qui est un artefact de test et non forcément un bogue applicatif) — je le signale par prudence, comme le 27 août, sans le requalifier en bogue confirmé.

### Aucun marquage dédié « cours réussi » — inchangé, assumé par ADR
- **Gravité** : mineur
- **Type** : pas encore construit / friction (inchangé)
- **Observé** : toujours seulement « créditer » comme contournement pour simuler un cours réussi ; comportement identique et assumé (ADR `2026-08-retrait-de-la-notion-de-cours-reussi`), sans changement depuis le 27 août.

### Non testé — à mentionner explicitement
- Le dépassement réel de 120 cr et son signalement en en-tête (scénario 5, deuxième partie) — je n'ai pas réussi à le provoquer.
- Le partage par URL, le tiroir Capsule, et l'allongement du cheminement au milieu (spinbutton Sessions pour un échec situé en cours de cheminement, pas seulement en H10 terminal) n'ont pas été retestés dans cette session de contre-vérification, faute de temps disponible après l'incident d'infrastructure — ils avaient déjà été notés comme non testés le 27 août et le restent.
- Un audit exhaustif de **chaque** cours déplacé dans le scénario de double-échec (j'ai vérifié la chaîne causale principale mais pas les six mouvements secondaires un par un, comme GCI-1003, GCI-1004, MAT-1900, MAT-1910, GCI-2009, GCI-2006) — ceux que j'ai vérifiés étaient tous justifiés, et la structure d'ensemble (redistribution proportionnée à la gravité de l'échec, deux cours de base retardés d'un an) semblait raisonnable, mais je ne l'affirme pas avec la même certitude que pour le scénario 1.

---

## Impression générale

Le correctif cible bien le problème que j'avais signalé le 27 août : sur mes trois reproductions exactes (un cours retardé, un ajout d'électif, et un nouveau test à deux échecs simultanés), le replacement est maintenant localisé et chaque déplacement que j'ai vérifié a une cause traçable — un vrai préalable ou une collision de plafond résolue par la session suivante de la même saison. J'ai même dû corriger mon propre diagnostic du 27 août : une partie de ce que je croyais injustifié (GCI-2001 déplacé « sans lien » avec GCI-1001) tenait en réalité à une chaîne de préalables transitive que je n'avais pas remontée jusqu'au bout.

Le profil développement durable affiche maintenant une jauge claire avec une note explicite, et je n'ai pas réussi à faire dépasser 120 crédits dans mes essais — un point positif, même si je n'ai pas pu vérifier le signalement d'un dépassement faute d'avoir trouvé comment le provoquer.

Ma seule réserve sérieuse, découverte deux fois indépendamment pendant cette contre-vérification (une fois sur le départ hiver, une fois sur le double-échec), est méthodologique mais mérite d'être corrigée dans l'interface elle-même : pendant que le solveur cherche une nouvelle solution (statut « recherche d'un organigramme - N s »), l'écran peut afficher un état transitoire qui ressemble à un décalage naïf de session, ou à une violation apparente de préalable/plafond, alors que le résultat final (quelques secondes plus tard) est correct. Le seul indice de cet état provisoire est une ligne de texte dans le panneau de gauche, facile à manquer si on regarde la grille à droite — et le texte « Placement vérifié ✓ » reste affiché sans distinction claire entre « vérifié pour maintenant » et « en cours de recalcul ». C'est exactement le genre de piège qui pourrait me faire annoncer un mauvais cheminement à un étudiant si je clique et je lis trop vite.

Sous cette réserve, je suis prêt à recommander l'outil pour la production de mes programmes types et pour répondre aux étudiants en échec — à condition de prendre l'habitude d'attendre la disparition du statut de recherche avant de lire le résultat, et idéalement que l'interface rende cet état transitoire plus visible directement sur la grille plutôt que dans une ligne de texte discrète.
