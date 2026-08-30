# Infobulles sans tiret cadratin

## Contexte

Quatorze infobulles (`title`) employaient le tiret cadratin « — » comme
séparateur : six en littéral dans `crates/ui/src/components/`, huit
construites dans `crates/ui/src/present.rs` (`freeze_toggle`, `freeze_all`,
`chosen_chip_title`, `ribbon_body`, `grid_status_label`,
`bac_credit_tooltip`) et une dans `header.rs` (`stale_title`).

Le tiret cadratin est une ponctuation d'incise : il ne dit ni « et ensuite »,
ni « c'est-à-dire », ni « mais ».
Dans une infobulle lue vite, survolée d'une main, il coûte un temps de
lecture sans porter de sens que le point ou la virgule ne porteraient pas.

## Décision

Aucune infobulle n'emploie de tiret cadratin.
Le séparateur est le **point** quand les deux morceaux sont des propositions
indépendantes, la **virgule** quand le second précise le premier.
La ponctuation superflue est évitée plutôt que remplacée : quand le contexte
sépare déjà (une énumération, des points de suspension), rien ne prend la
place du tiret.

Conséquence de bord : `bac_credit_tooltip` termine désormais ses deux
branches par un point, parce que `stale_title` s'y accole pendant un
recalcul et que la césure ne peut plus venir d'un tiret.

Cette règle vaut pour les infobulles seules.
Le texte visible, les avis et les commentaires de code gardent leur
ponctuation ; ils se lisent posément, l'infobulle non.

## Rejeté

- **Le deux-points partout.** Il tient pour une définition
  (« Concomitance : … ») mais pas pour une suite de faits, et trois
  deux-points dans une même bulle se lisent moins bien qu'un point.
- **La parenthèse.** Elle hiérarchise l'information au lieu de l'enchaîner,
  et une infobulle n'a pas d'aparté à faire : ce qu'elle dit, elle le dit.
