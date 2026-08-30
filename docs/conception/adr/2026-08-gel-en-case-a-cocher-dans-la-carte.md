# Le gel d'une session est une case à cocher dans sa carte, plus un bouton sur l'horaire

Date : 2026-08-30

Déplace la commande décidée par l'ADR `2026-08-sessions-gelees-generalisent-les-completees`; s'appuie sur `2026-08-carte-de-session-chassis-et-face`.

## Contexte

Antoine, 2026-08-30 : « Geler dans les sessions d'organigramme devrait être entre le titre (H2-H27) et le nombre de crédits et ça devrait un checkbox là plutôt qu'un bouton sur l'horaire. »

Jusqu'ici la bascule vivait dans l'en-tête de l'horaire (`.grid-head`), et la carte de la session ne faisait qu'*annoncer* le résultat par un insigne « ❄ gelée » dans son corps. Trois choses clochent :

- **Le geste est loin de son objet.** Geler, c'est décider du cheminement, pas de la semaine. Pour geler trois sessions il fallait afficher chacune d'elles, descendre à l'en-tête de l'horaire, cliquer, revenir.
- **L'état ne se lit pas d'ensemble.** Le bouton ne dit l'état que de la session affichée; savoir lesquelles sont gelées demandait de les visiter une à une, l'insigne dans la carte étant le seul aperçu.
- **L'insigne coûte une ligne de sigles.** Il compte dans les `notes` de `present::ribbon_model`, donc une session gelée montrait un cours de moins qu'une autre — un prix payé pour répéter ce que la commande dira désormais elle-même.

## Décision

**La bascule du gel est une case à cocher dans l'en-tête de la carte, entre le libellé de la session et ses crédits. Le bouton de l'horaire est retiré, et l'insigne « ❄ gelée » disparaît du corps.**

- La case coche et décoche par `edit_plan(plan, history, « Session gelée » / « Session dégelée », …)` sur `plan.frozen` : un acte étiqueté et annulable, jamais une confirmation (ACT-2). La sémantique ne change pas, seul le lieu du geste change.
- Les trois textes de la case viennent de `present::freeze_toggle` (pur, testé) : son **nom accessible ne bascule pas avec l'état** — « Geler la session A1-A26 », toujours — parce que c'est la case cochée qui dit si la session est gelée. Une étiquette qui basculerait ferait lire « Dégeler la session » à une case décochée. Le `title` explique ce que geler veut dire dans l'état où l'on est, et reprend mot pour mot les formulations déjà écrites.
- **Le glyphe ❄ reste, collé à la case** : une case cochée seule ne dit pas « gelée », et l'état ne tient jamais à la seule couleur (INP-3). La bordure en tirets de `.ribbon-card--frozen` reste, en redondance.
- **L'insigne « ❄ gelée » du corps est retiré** : il répétait une commande désormais visible sur la même carte, deux lignes plus haut. `frozen` sort du compte `notes` de `ribbon_model` — sans cela la carte réserverait une ligne à un insigne qui n'existe plus. Une session gelée montre donc autant de sigles qu'une autre.
- **La bande d'un été vide (`.ribbon-summer`, 1.75rem de large) reçoit la même case**, en haut, empilée au-dessus de sa face — même restructuration châssis/bouton que la carte. Sans cela, retirer le bouton de l'horaire aurait supprimé en silence toute affordance de gel sur un été vide, le cas où geler est justement le plus utile. Le ❄ n'y tient pas à côté de la case : il reste le préfixe du contenu vertical, dont le `title` porte le mot.

## Alternatives rejetées

- **Garder les deux commandes** (horaire *et* carte) : deux vérités à tenir synchrones pour un état unique, et Antoine a demandé « plutôt que ».
- **Un bouton-bascule dans la carte au lieu d'une case** : l'en-tête n'a pas la largeur d'un mot, et un bouton dont l'étiquette bascule (« Geler » / « ❄ Dégeler ») demande de lire le verbe pour connaître l'état — une case cochée le donne d'un coup d'œil sur les huit sessions.
- **Garder l'insigne « ❄ gelée » sous la case** : redondance payée en une ligne de sigles, sur une carte dont le budget est justement le point serré.
- **Laisser l'été vide sans case, en renvoyant à l'horaire** : l'horaire n'a plus la commande; ce serait une capacité perdue en silence.

## Réserve assumée

La case fait 0.875rem (14 px) et son étiquette cliquable la hauteur de l'en-tête, soit ~30 × 18 px : sous le minimum de 48 dp d'INP-1. La densité du ruban — huit cartes de 5.5rem minimum sur une rangée — ne laisse pas la place d'une cible conforme sans reprendre la mise en page entière. L'étiquette s'étire déjà sur toute la hauteur de l'en-tête pour offrir la plus grande cible que cette densité permette, et le clavier y accède normalement (INP-4 : la case est focusable, Espace la bascule). À rouvrir si le ruban est un jour repris.
