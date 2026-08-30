# Alertes en toasts flottants en coin

Date : 2026-08-14

## Contexte

Retour d'Antoine (2026-08-14) : les messages empilés dans la bande de statut prenaient trop de place et poussaient le panneau et l'horaire vers le bas.
Quatre méthodes lui ont été proposées (tiroir ancré à la bande, bande repliée dépliable en place, toasts en coin, messages contextuels) ; il a choisi les **toasts flottants en coin**, et l'auto-effacement des messages de succès.

## Décisions

- La bande de statut ne garde que le statut du solveur et Annuler/Rétablir : **une ligne fixe, qui ne grandit plus jamais**.
- Les alertes vivent dans `header::Toasts` : pile en `position: fixed` bas-droite, par-dessus la grille. Le clic n'importe où sur un message le ferme (note 12 conservée) — **amendé le 2026-08-30 : seul le `✕` ferme, voir `2026-08-le-x-seul-ferme-le-message`** ; les 3 plus récents sont visibles, au-delà une carte « +N autres - tout afficher » déplie la pile (défilement interne, plafond ALR-3).
- **Seuls les ✓ (`Success`) s'auto-effacent après 5 s** — la priorité la plus basse, ce qu'ALR-4 permet ; les ⚠ et les erreurs persistent jusqu'au rejet explicite. La minuterie vit dans le composant (`use_effect` + `spawn`), pas dans `push_alert` : les rappels du worker appellent `push_alert` hors du runtime Dioxus, où `spawn` paniquerait.
- **Tension AIR assumée** : ALR-6 interdit qu'une alerte occlude les données vives ; les toasts recouvrent un coin de la grille tant que des messages existent. C'est l'arbitrage explicite d'Antoine — l'occlusion est bornée (un coin, rejetable d'un clic) contre une bande qui déplaçait tout le contenu.
- **Un doublon rafraîchit, il n'est plus avalé** (révision post-essai, rapport `2026-08-14`) : `push_alert` retire l'exemplaire existant et repousse le message en tête avec une clé neuve — relancer une recherche qui aboutit au même verdict doit répondre visiblement, et il n'y a toujours jamais deux fois le même message (ALR-3). Les clés d'alertes sont un compteur monotone jamais recyclé : une minuterie d'auto-effacement en retard ne peut pas tuer un message neuf qui aurait repris sa clé.

## Amendements

- `2026-08-toasts-un-par-sujet-et-rejet-memorise` : la déduplication par corps ne suffit pas aux notes que le solveur republie à chaque réponse — un **sujet** décide désormais du remplacement, et un rejet se souvient.

## Alternatives rejetées

- Bande repliée dépliable en place : conforme AIR au plus strict, mais le dépliage repousse encore le contenu pendant la lecture.
- Tiroir flottant ancré à la bande : garde la région réservée, mais recouvre le haut des deux panneaux — pire que le coin pour la grille.
- Messages contextuels (chaque alerte à l'endroit concerné) : le plus lisible à terme mais un chantier de reclassement complet — à reconsidérer si les toasts s'avèrent bruyants.
