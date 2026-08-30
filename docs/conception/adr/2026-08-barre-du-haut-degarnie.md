# La barre du haut se dégarnit : le compte seul, et « Partager » avec « Exporter »

Date : 2026-08-30

> Suite : l'écart de 3 rem décrit ici est ramené à 2 rem par l'ADR `2026-08-tout-geler-sur-une-ligne-et-verdict-ecourte`, le voisin de « Réinitialiser » n'étant plus « Partager » mais « Tout geler ».
> Puis à 0.5 rem par l'ADR `2026-08-ecart-reduit-entre-tout-geler-et-reinitialiser`, où l'écart cesse de porter ACT-5 : la teinte, le libellé et l'annulabilité le portent seuls.

## Contexte

La barre du haut avait accumulé, à droite du sous-titre, quatre choses de natures différentes : le compte de crédits du bac, une parenthèse « (+9 cr en sus) » nommant les crédits hors total, un « ? » qui expliquait ce que « en sus » voulait dire (ADR `2026-08-vocabulaire-explique-en-place-a-la-demande`), puis « Partager » et « Réinitialiser » séparés par un trait vertical (ADR `2026-08-reinitialiser-annulable-depuis-son-avis`).

Chaque pièce répondait à un constat réel, mais l'empilement s'est retourné contre la lecture : la ligne se coupait en deux à 1280 px, et le « ? » venait s'intercaler entre le total du bac et celui de la session — deux nombres qui se lisent ensemble.

## Décision

Quatre retraits et un déplacement, demandés par Antoine :

- **La parenthèse des crédits hors total disparaît.** Le compte se lit « 104/120 cr au bac », rien de plus. `BacCreditNote` n'existait que pour porter ce suffixe à côté de son infobulle ; elle est remplacée par `present::bac_credit_tooltip`, et `bac_credit_label` perd son paramètre `note`.
- **Le « ? » qui expliquait « en sus » disparaît avec elle**, ainsi que `IN_ADDITION_HELP`, le signal qui le pliait, et le champ `BacReadout.en_sus` qui n'existait que pour décider de son affichage. Le « ? » de la *version* de programme, lui, reste : le mot « version » n'a pas d'équivalent courant.
- **Le trait vertical entre « Partager » et « Réinitialiser » disparaît.** L'écart de 3 rem qui portait la moitié de la séparation est conservé, reporté sur `.header-reset`.
- **« Partager » rejoint « Exporter » dans la bande de statut.** Les deux sortent le *document entier* — un lien qui le rouvre, un fichier qui le fige — alors que la barre du haut nomme ce qui est ouvert. Le geste rejoint donc son voisin de sens, et non plus son voisin de position.

L'information n'est pas perdue : l'infobulle du compte décompose toujours l'écart (« 9 cr de stages exigés mais ajoutés aux 120 cr, jamais comptés dedans »), et les stages gardent leur propre règle « Stages » dans le panneau, listée et comptée comme les autres (ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`). Rien d'exigé ne disparaît de l'écran, et l'infobulle n'est jamais le seul porteur — ce qui aurait été une affordance au survol, interdite par INP-5.

**ACT-5 s'en trouve mieux servi, pas moins.** La règle veut qu'un contrôle destructeur ne jouxte pas un contrôle fréquent ; « Partager » parti, plus aucun geste courant ne borde « Réinitialiser », qui garde sa teinte d'accent et reste à découvert, jamais dans un menu (LAY-7).

## Une note sur le vocabulaire

Le remplacement de « en sus » par « supplémentaire » a été demandé, puis rendu sans objet par le retrait de la parenthèse : le terme n'apparaît plus nulle part à l'écran.

Il reste, et doit rester, dans `core::parser::program` : `note.contains("en sus des crédits exigés")` lit la formulation de l'Université sur la page scrapée. C'est sa phrase, pas la nôtre ; la renommer casserait la détection de `credits_in_addition`. Le vocabulaire de la source et celui de l'affichage sont deux choses distinctes, et seul le second se choisit.

## Alternatives rejetées

- **Garder la parenthèse en la renommant « supplémentaires »** : c'était la demande initiale, remplacée par le retrait pur et simple. Le mot courant se passait de son « ? », mais la ligne restait chargée de deux nombres qui ne s'additionnent pas.
- **Cacher « Réinitialiser » dans un menu** pour l'écarter de « Partager » : interdit par LAY-7 — une action que l'étudiante doit pouvoir retrouver ne se range pas derrière un débordement.
- **Laisser « Partager » dans la barre du haut et n'en retirer que le trait** : l'écart seul suffisait à ACT-5, mais laissait le geste loin de son jumeau « Exporter ».
