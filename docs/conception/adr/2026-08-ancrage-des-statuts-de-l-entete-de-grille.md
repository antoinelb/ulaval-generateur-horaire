# Le pivot élastique de l'entête de grille est porté par le statut, pas par le titre

Date : 2026-08-30

## Contexte

L'entête de la grille horaire (`div.grid-head`, `WeeklyGrid`) aligne quatre choses sur une rangée : le titre de session (« A5 — Automne 2028 »), l'avertissement « ⚠ N cours hors grille » (conditionnel, `off_grid > 0`), le statut de l'horaire (inconditionnel — « combinaison automatique - sans conflit ✓ », préfixé de « ⟳ recalcul en cours… — » tant que le solveur cherche, ADR `2026-08-recalcul-visible-sur-la-grille`), et le bouton « Libérer les sections forcées ».

Deux défauts s'y sont accumulés.

**L'alignement.** `align-items: baseline` alignait les lignes de base : les statuts en `0.75rem` partageaient la base du titre en `0.9375rem`, donc leur centre optique tombait sous celui du titre. Ils paraissaient décalés vers le bas.

**L'ancrage.** `.grid-head h2 { margin-right: auto }` faisait du titre le pivot : tout le reste était tassé à droite, et dans un tel amas seul l'élément le plus à droite garde son bord. Le statut occupant cette place, l'apparition du préfixe « ⟳ recalcul en cours… — » — qui survient à *chaque* saisie, 500 ms après (`auto_propose`) — poussait « ⚠ N cours hors grille » vers la gauche. Un décalage asynchrone récurrent d'un élément déjà à l'écran, que l'utilisateur n'a pas provoqué : LAY-2.

## Décision

- `.grid-head` passe à `align-items: center`. Titre, statuts et bouton se centrent sur le même axe médian. `align-content: flex-start` reste tel quel : il gouverne l'empilement des lignes repliées, pas l'alignement dans une ligne, donc la réserve de hauteur de l'ADR `2026-08-reserve-de-hauteur-limitee-a-la-bande-ou-la-rangee-se-plie` se comporte à l'identique.
- Le statut est rendu **avant** l'avertissement dans le `rsx!`, et le pivot élastique déménage du titre vers lui : `margin-right: auto` quitte `.grid-head h2` pour `.grid-head h2 + .grid-status`. Le statut est ainsi ancré à gauche, contre la session ; l'avertissement et les boutons restent ancrés à droite.
- Aucune classe nouvelle côté Rust : le sélecteur d'adjacence suffit parce que le `span` de statut est inconditionnel et suit immédiatement le `h2`. Le seul changement Rust est l'ordre des deux `span`.

Conséquence : ni l'arrivée du préfixe de recalcul (fréquente) ni celle de l'avertissement (au résultat du solveur) ne déplace l'autre. Le décalage asynchrone qui subsistait dans cette rangée disparaît.

Effet de bord bienvenu : le statut disposant de tout l'espace libre jusqu'au pivot, `text-overflow: ellipsis` le tronque bien plus rarement — le « ✓ » final cessait de s'afficher aux largeurs moyennes.

## Même arbitrage dans la bande de statut

`div.status-strip` (`HeaderBar`) souffrait du même défaut par une autre cause : `align-items: flex-start`, hérité du commit initial de l'interface sans commentaire ni ADR. Les boutons « ↶ Annuler », « Partager » et « Exporter ▾ » y font 29 px et le texte « recalcul du placement - N s » 18 px, si bien que ce dernier pendait **7,5 px au-dessus du centre** de la bande.

Elle passe elle aussi à `align-items: center`. Rien ne réclamait `flex-start` : aucun enfant de la bande n'est multiligne — les avis sont des toasts (`.toasts`, hors de la bande), le menu d'export est en `position: absolute`, et le calque de retrait (`.status-drop`) est un frère absolu. La hauteur de la bande reste celle de ses boutons, identique dans les trois états du solveur (repos, temporisation, recherche annulable), donc `min-height: 2.5rem` protège toujours ce qu'il protégeait.

## Alternatives rejetées

- **Garder les deux statuts tassés à droite**, en se contentant d'inverser leur ordre. Le sens de lecture demandé était obtenu, mais le défaut LAY-2 changeait seulement de victime : l'arrivée de « ⚠ N cours hors grille » déplaçait alors le statut de la largeur entière de l'avertissement. Porter le pivot par le statut coûte la même ligne de CSS et ne déplace plus rien.
- **Réserver la place de l'avertissement en permanence** (une boîte vide de largeur fixe quand `off_grid == 0`). Aurait aussi stabilisé la rangée, mais la largeur dépend du nombre de chiffres du compte, et cela ajoutait un élément vide à l'arbre pour un problème que l'ancrage règle sans rien rendre de plus.
- **Ne centrer que la pastille de recalcul** (`align-self: center` sur `.grid-status--searching`). Le recalcul se serait alors trouvé décalé par rapport à l'avertissement resté sur la ligne de base — deux statuts voisins désalignés entre eux.
- **Scinder le statut en deux `span`** pour styler « ⟳ recalcul en cours… » séparément du verdict. Rejeté : `grid_status_label` produit une chaîne unique justement pour qu'aucune hauteur nouvelle n'apparaisse (ADR `2026-08-recalcul-visible-sur-la-grille`), et le besoin ici est un ancrage, pas un style distinct.
