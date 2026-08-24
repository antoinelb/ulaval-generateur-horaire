# Canal de contact dans le pied de page

Date : 2026-08-24

## Contexte

L'application n'offrait aucun point de contact visible.
La seule adresse de signalement était le bouton « Proposer au catalogue (GitHub) » du panneau des cours manuels (`panel.rs`), qui n'apparaît qu'après avoir ajouté un cours à la main.
Un étudiant qui tombe sur un bogue n'avait nulle part où le dire.

Demande d'Antoine (2026-08-24) : un bref message de contact, en gris pâle, en bas à droite.

## Décisions

- **Le canal de signalement est une ligne du pied de page**, à droite de la ligne de provenance, dans le même `footer` : courriel (`mailto:`) et dépôt GitHub, tous deux cliquables.
  `.footer` passe en `display: flex; flex-wrap: wrap`, et le contact porte `margin-left: auto` : il reste à droite aussi bien côte à côte que replié sur sa propre ligne.
  Les deux lignes ne tiennent côte à côte qu'au-delà de ~1240 px ; sous ce seuil elles s'empilent, sans règle ajoutée au `@media` existant.
  Le sélecteur doit être `p.footer-contact` et non `.footer-contact` : la règle `.footer p { margin: 0 }` qui la précède est plus spécifique et écrasait le `auto`.
- **Le pied passe de `--muted-2` à `--muted`** : `#9a948c` sur `--band` donnait 2,85:1, sous le plancher INP-2 de 4,5:1 ; `#6f6a63` donne 5,1:1. La violation préexistait sur la ligne de provenance.
  Elle est corrigée pour toute la bande plutôt que pour le seul texte neuf, Antoine ayant tranché contre deux gris différents dans une même bande.
- **`.toasts` remonte de `bottom: 0.75rem` à `4rem`**. C'est le seul élément `position: fixed` du projet et il occupait déjà le coin bas-droite ; le coin étant vide jusqu'ici, l'occlusion ne se voyait pas.
  Une alerte ne doit pas recouvrir le canal par lequel on signale les problèmes.
  `4rem` couvre le pied sur une ligne (~2 rem) comme replié sur deux (~3,4 rem), le cas courant sous 1240 px.
  **Résidu assumé** : à un zoom texte de 200 %, le pied replié atteint ~174 px alors que les toasts sont à 128 px — un toast recouvre alors le haut de la ligne de provenance, pas le contact. Suivre la hauteur réelle du pied demanderait de la mesurer en JS ; la constante couvre les cas réels.
- **Écart INP-1 assumé** : les deux liens font des cibles d'environ 28 px, pas les 48 px exigés. Arbitrage explicite d'Antoine — porter la bande à 48 px lui volait de la hauteur, dans un `.shell` en `height: 100vh`. L'écart est marqué en commentaire dans `shell.rs` plutôt que passé sous silence.
- **Limite connue** : l'écran d'erreur `Failure` ne porte pas ce message. `Screen` aiguille vers `Failure` *ou* `Shell`, et le pied ne vit que dans `Shell` ; quand le catalogue ne charge pas — le moment où l'on veut le plus signaler un problème — il n'y a que `error.action`.
  À traiter séparément.

## Alternatives rejetées

- **Un élément `position: fixed` dédié en bas à droite** : collision frontale avec `.toasts`, qui occupe déjà ce coin, et une seconde région flottante là où LAY-1 demande des régions stables.
  Le pied existant est déjà la bande du bas ; y ajouter une moitié droite coûte trois propriétés CSS.
- **Une entrée dans le panneau gauche** : invisible au moment où l'on en a besoin, et le panneau est déjà dense.
- **Élargir la bande à 48 px pour satisfaire INP-1** : conforme, mais arbitré contre par Antoine (voir ci-dessus).
- **Une constante partagée pour l'URL du dépôt**, factorisée avec `panel.rs` : deux usages aux formes différentes (`/issues/new?…` contre la racine), et refactorer du code voisin qui marche pour deux occurrences n'en vaut pas le diff.
