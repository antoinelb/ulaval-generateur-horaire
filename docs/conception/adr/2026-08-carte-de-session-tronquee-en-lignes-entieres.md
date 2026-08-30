# La carte de session tronque en lignes entières et compte le reste

Date : 2026-08-30

Remplace la partie « la carte grandit au-delà de cinq » de l'ADR `2026-08-hauteur-minimale-du-ruban-pour-cinq-cours`.

## Contexte

Élodie rapporte (2026-08-29) : génie physique, session A1 avec 6 cours (13 cr) — « la dernière ligne de cours (« PHY-1003 ») est coupée à mi-hauteur, sans ascenseur ni mention « et plus » ».

L'ADR `2026-08-hauteur-minimale-du-ruban-pour-cinq-cours` avait décidé que la carte grandit au-delà de cinq sigles, « plutôt que de cacher des sigles dans un défilement », et promettait des « sigles toujours tous visibles ». La promesse ne tient pas, pour deux raisons lisibles dans le CSS :

1. **Le plancher est sous-dimensionné.** Son calcul suppose des sigles à `0.6875rem`; `.ribbon-card-codes` les rend à `0.875rem`, interligne 1.5, soit `1.3125rem` par ligne. Dans les `7.75rem` du plancher il ne reste que `5.31rem` pour le corps — moins de quatre lignes, pas cinq. Le débordement commence donc au quatrième sigle, pas au sixième.
2. **La rangée peut être comprimée.** `.ribbon` porte `overflow-x: auto`; un `overflow` autre que `visible` met la *taille minimale automatique* d'un élément flex à 0. La coquille est `height: 100vh; overflow: hidden` en colonne flex : dès que l'en-tête (dont le `⚠` se replie sur plusieurs lignes), le ruban, la bande de statut et le plancher de `.main-split` demandent plus que la fenêtre, c'est le ruban qui cède — et il coupe son contenu à l'endroit où il s'arrête, c'est-à-dire au milieu d'une ligne de texte.

Une coupe à mi-glyphe se lit comme un défaut d'affichage, pas comme une troncature voulue.

## Décision

**La rangée des sessions a une hauteur constante, et ce qu'une carte ne montre pas, elle le compte.**

- `present::ribbon_body` (pur, testé) décide combien de sigles la carte affiche : un budget fixe de `CARD_BODY_LINES = 5` lignes de corps. Les lignes d'annonce que la carte doit déjà rendre — conflit d'horaire, session gelée, annotation libre — se prennent **sur le même budget**, si bien qu'une carte qui a quelque chose à annoncer montre moins de sigles au lieu de grandir. Ce qui ne tient plus devient un « +2 », dernière ligne du corps, jamais une ligne coupée.
- Le « +N » porte en `title` les sigles absents et nomme le geste qui rend la session entière : le clic sur la carte, qui est déjà l'affordance visible du ruban (jamais un accès au seul survol). Le clavier y accède par le même bouton.
- `.ribbon-card` passe de `min-height: 7.75rem` à `height: 9.5rem` — en-tête plus cinq lignes réellement calculées à `0.875rem` — avec `overflow: hidden` en garde-fou : la troncature vient de Rust, le CSS ne fait que ne jamais mentir si elle échouait.
- `.ribbon` passe en `flex: none` : sa hauteur est désormais bornée par celle des cartes, donc la coquille n'a plus à la comprimer. Le jeu manquant va à `.main-split`, dont le panneau et la grille défilent chacun de leur côté. Sous 48rem de large, la coquille rend déjà le défilement à la page et rien ne change.
  Une soupape accompagne ce `flex: none`, la même que celle du bandeau d'en-tête : au-delà de `max-height: 40vh` (jamais atteint à l'échelle normale, la rangée faisant ~10.75rem) elle défile elle-même, ascenseur compris — au zoom texte 200 % (INP-8) elle ne peut donc pas manger l'écran.

C'est bien la « troncature nommée » et non le « défilement interne » qu'Antoine avait refusé en 2026-08-27 : aucun sigle ne se cache derrière un ascenseur muet — leur nombre est écrit sur la carte, et un clic ouvre la session entière.

## Alternatives rejetées

- **Garder la croissance et corriger seulement le plancher** (`min-height: 9.5rem`) : la hauteur de la rangée suivrait toujours son contenu, donc changerait toute seule au retour du placement automatique (500 ms après la frappe) — la violation LAY-1/LAY-2 que l'ADR de 2026-08-27 assumait faute de mieux; et la compression par la coquille recouperait quand même une carte chargée.
- **Un défilement interne des sigles** : refusé le 2026-08-27, et à raison — un ascenseur de quelques rem cache le nombre de cours au lieu de le dire.
- **Un plancher au pire cas (8 sigles)** : gaspille en permanence la hauteur de la grille, la zone la plus précieuse de l'écran.
- **Ne montrer que le nombre de cours et le total de crédits** : les sigles de la carte sont la source du glisser-déposer; les retirer coûterait un geste entier.
