# Fantômes dédupliqués et libellés compacts

Date : 2026-08-28

## Contexte

Sélectionner un cours révèle un bloc fantôme par créneau de chaque option alternative (`present.rs`, `grid_model`) ; `assign_lanes` (`present.rs`) divise ensuite la colonne du jour en couloirs de largeur égale.
Deux problèmes observés (rapport étudiante GEX 2026-08-27) :

- La déduplication `seen` qui fusionne les créneaux jumeaux d'une option hybride (deux sections au même horaire) ne s'appliquait qu'aux blocs pleins, jamais aux fantômes — un cours à 5 options avec sections jumelles produisait jusqu'à 5 couloirs pour 4 plages réelles.
- Chaque fantôme portait le titre complet du cours (`block.title = title`), qui se tronque à une ou deux lettres par ligne dans un couloir de ~25 px de large (« M / p / l / I » pour « Mathématiques pour l'ingénierie I ») — illisible visuellement, alors que l'arbre d'accessibilité révélait les vrais libellés.

## Décision

- Déduplication locale des fantômes par clé `(day_index, start, end, nrcs de l'option)` : `nrcs` est calculé une fois par option (`option_nrcs`), donc deux sections jumelles de la même option partagent la même clé et fusionnent, tandis que deux options distinctes qui tombent sur le même créneau gardent des `nrcs` différents et restent deux blocs cliquables.
- Nouvelle fonction pure `ghost_label(section: &Section) -> String` (`present.rs`) : la lettre de section quand la page en donne une, sinon le NRC, suivie des mêmes mots que `section_detail` selon le mode (« à distance », « hybride »). Un fantôme reçoit `title: ghost_label(section)` et `detail: String::new()` — le libellé compact tient dans un couloir étroit, sans répéter la ligne de détail.
- Le `title=` HTML du bouton (l'infobulle « Forcer la section … de … ») n'est pas touché : il reste le libellé long, lu au survol ou par un lecteur d'écran, pas contraint par la largeur du couloir.
- Le gestionnaire de clic n'est pas touché : il pinne déjà les bonnes sections par NRC, indépendamment du texte affiché.

## Alternatives rejetées

- **Vraie liste de choix (menu déroulant ou popover) au lieu de blocs sur la grille** — déplacerait l'action de sélection hors de la grille, perdant le repère spatial (l'heure/le jour) qui fait justement l'intérêt d'afficher les alternatives sur la grille plutôt qu'en liste.
- **Plafonner le nombre de couloirs affichés** (n'en montrer que 2-3, le reste dans un « +N ») — cacherait des options pourtant cliquables, contraire à LAY-2/l'exigence que rien à l'écran ne mente sur ce qui est disponible.
