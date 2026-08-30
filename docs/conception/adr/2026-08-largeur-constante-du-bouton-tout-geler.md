# Largeur constante du bouton « Tout geler »

Date : 2026-08-30

## Contexte

Le bouton de la barre du haut bascule son libellé entre « Tout geler » et « Tout dégeler » (ADR `2026-08-bouton-tout-geler-dans-la-barre-du-haut`, ADR `2026-08-tout-geler-sur-une-ligne-et-verdict-ecourte` pour le retrait du ❄).
Les deux glyphes de plus élargissaient le bouton, mesuré au navigateur à 1280 px : **82 × 32 px** au repos, **97 × 32 px** une fois tout gelé.

Les 15 px se répercutaient sur « Réinitialiser », son voisin immédiat de droite dans `.header-actions`, qui se déplaçait d'autant.
C'est **LAY-1** : rien à l'écran ne bouge si l'utilisateur ne l'a pas bougé.
Ici ce n'est pas qu'esthétique — le geste suivant le plus probable après « Tout geler » est un clic dans cette zone, et la cible s'était déplacée sous le curseur.

Le dépôt traite déjà exactement ce problème pour le compteur de la minuterie : `.status-running-elapsed` fixe une `min-width` dimensionnée sur « 999 s » pour que chaque tick ne décale plus « Annuler la recherche ».

## Décision

Une classe dédiée `header-freeze` sur le bouton (sur le modèle de `header-reset`), portant deux déclarations dans `main.css` :

```css
.header-freeze {
  min-width: 6.75rem;
  justify-content: center;
}
```

**La valeur.**
Le poste de mesure résout `system-ui` en Verdana, la plus large des sans installées : « Tout dégeler » à 12 px y fait 74,8 px, contre 66,0 px en Arial et en Liberation Sans.
Les 97 px mesurés sont donc **déjà le haut de la fourchette des fontes réelles, pas son milieu** — la pile `--font-sans` (`system-ui, -apple-system, "Segoe UI", sans-serif`) ne propose ailleurs que des faces plus étroites.
6,75 rem = 108 px laissent 86 px de contenu (108 − 20 de padding − 2 de bordure, `box-sizing: border-box` étant global), soit **15 % au-dessus de Verdana**.

En `rem`, jamais en `px` : `html { font-size: 100% }` est le pivot du zoom texte à 200 % (INP-8), et le libellé lui-même est en rem — le rapport entre la largeur plancher et le texte est donc conservé à tout zoom.

`justify-content: center` n'est pas décoratif : `.status-undo` est un `inline-flex` sans alignement principal, donc dans un bouton élargi le libellé se collerait contre le padding de gauche.

La règle cohabite avec le `flex: none` de `.header-bar .status-undo` (ADR `2026-08-ecart-reduit-entre-tout-geler-et-reinitialiser`) plutôt que de le contrarier : `flex: none` donne une base de contenu incompressible, `min-width` ne fait que relever ce plancher.
Les 26 px pris à la barre le sont sur `.header-notice`, prose qui se recompose sans rien perdre — le même arbitrage que celui déjà consigné pour `flex: none`.

Ni le libellé ni la logique de bascule ne bougent : `present::freeze_all` est inchangée.

## Alternatives rejetées

**Empiler les deux libellés en grille CSS** (`display: grid`, les deux textes en `grid-area: 1/1`, l'inactif en `visibility: hidden`) — la largeur devient celle du plus long quelle que soit la fonte, garantie par construction.
Rejeté :

- la mesure retire l'essentiel de son intérêt. La garantie sert contre une fonte plus large que celle mesurée; or la mesure est déjà prise sur la plus large des fontes plausibles. La marge n'est plus une supposition, c'est du dégagement au-dessus d'une borne supérieure connue;
- la `min-width` **se dégrade proprement**. Si une fonte dépassait quand même, le décalage résiduel vaut (largeur naturelle − 6,75 rem), strictement plus petit que les 15 px corrigés aujourd'hui. Il n'existe pas de cas où elle fait pire que rien;
- le coût est réel : un champ de plus sur `FreezeAll`, contrat arbitré il y a deux ADR; un nœud DOM de plus; un doublon du libellé en `aria-hidden` dont il faut faire confiance aux lecteurs d'écran pour l'ignorer; et l'écrasement du `display: inline-flex` de `.status-undo`, qui existe pour une raison documentée (l'alignement du `kbd`, ADR `2026-08-raccourci-imprime-sur-le-bouton`).

Une abstraction qu'on n'a pas mesurée nécessaire est un coût, pas une sécurité — et `.status-running-elapsed` a déjà tranché le même arbitrage dans le même sens.

**Une `min-width` en pixels sur les 97 px mesurés.** Deux défauts cumulés : aucune marge pour une autre fonte, et une valeur qui ne suit pas le zoom texte à 200 % alors que le libellé, lui, le suit (INP-8).

**`white-space: nowrap` seul.** Ne traite pas le sujet : le bouton garderait sa largeur variable. Cette parade avait déjà été écartée pour le repli sur deux lignes, où elle aurait laissé le texte déborder de sa bordure.
