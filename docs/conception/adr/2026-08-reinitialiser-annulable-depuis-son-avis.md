# « Réinitialiser » porte son propre « Annuler », et s'éloigne de « Partager »

Date : 2026-08-30

## Contexte

Depuis l'ADR `2026-08-reinitialiser-reste-dans-le-programme`, le bouton vide le document sans quitter son programme, et le pas d'annulation qu'il arme survit.
Camille (étudiante en génie des eaux, 2026-08-29) rapporte pourtant que rien ne le dit :

> « Le plan revient instantanément à l'état généré par défaut sans aucun dialogue, aucun toast, rien. Le bouton « Annuler Ctrl+Z » reste actif et permet heureusement de tout récupérer — mais rien à l'écran n'indique que c'est possible. »

Deux défauts distincts derrière un seul clic :

- l'avis poussé après la réinitialisation était un `AlertBody::Success`, donc effacé tout seul après 5 s (`SUCCESS_TOAST_MS`) — l'issue de secours disparaissait avant d'avoir été lue, et elle n'était de toute façon qu'une phrase désignant un bouton situé ailleurs à l'écran ;
- le bouton était **collé à « Partager »** dans le bandeau, c'est-à-dire au geste que Camille pose le plus souvent. ACT-5 interdit exactement cette adjacence.

## Décision

- **ACT-5, séparation.** « Réinitialiser » reste dans le bandeau, visible et atteignable (LAY-7 : jamais dans un menu de débordement), mais un trait vertical (`.header-sep`) et un écart total de 3 rem le détachent de « Partager », et `.header-reset` le teinte de l'accent. La couleur ne porte pas seule la différence (INP-3) : l'écart et le trait la disent d'abord.
- **ACT-2, annulabilité visible.** L'acte pousse `AlertBody::DocumentReset(Box<Plan>)`, sur le patron exact de `AlertBody::LocalProgramRemoved` : l'avis **transporte le document tel qu'il était**, affiche ce qui a été vidé (`present::reset_note`, qui nomme le programme et son millésime) et porte son propre bouton « ↶ Annuler ». Comme il n'est pas un `Success`, aucune minuterie ne l'efface : il tient jusqu'au clic de l'étudiante (ALR-4).
- **L'annulation est un acte de l'historique, pas un `state::undo`.** `components::restore_document` réinstalle le plan transporté par un `edit_plan` étiqueté « Retour avant la réinitialisation ». L'avis persistant peut être consommé longtemps après ; d'ici là la réinitialisation n'est peut-être plus le sommet de la pile, et défaire à sa place ce que l'étudiante a fait entre-temps serait une deuxième surprise.

## Alternatives rejetées

- **Un dialogue de confirmation** (« Êtes-vous sûre ? »). Rejeté par **ACT-2**, explicitement : sous pression on clique au travers des dialogues, et un dialogue posé sur un acte *sûr* — celui-ci l'est, il est annulable — entraîne le clic-réflexe qui défait ensuite les dialogues posés sur les actes réellement irréversibles. Le dialogue ne rendrait d'ailleurs rien annulable : il déplace la charge sur l'étudiante au lieu de la porter. L'annulabilité **est** la protection.
- **Cacher « Réinitialiser » derrière un menu de débordement** : LAY-7 l'interdit pour une action que l'étudiante doit pouvoir retrouver. La distance suffit ; l'invisibilité n'est pas une sécurité.
- **Un compte à rebours annulable (ACT-3)** : l'acte est instantané dans ses effets et parfaitement réversible ; retarder l'affichage du résultat coûterait plus qu'il ne protège.
- **Laisser l'avis en `Success` et compter sur « ↶ Annuler » dans la bande d'état** : c'est l'état rapporté par Camille — le recours existe, l'écran ne le dit pas, et il disparaît après 5 s.
- **Faire porter à l'avis un simple `state::undo`** : plus court, mais il annule le sommet de la pile, pas la réinitialisation, dès que l'étudiante a touché autre chose.
