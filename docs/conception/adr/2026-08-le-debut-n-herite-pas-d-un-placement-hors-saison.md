# Changer « Début » n'hérite pas d'un placement que la nouvelle saison invalide

Date : 2026-08-29

## Contexte

Une évaluation persona du 2026-08-29 a reproduit deux fois en navigateur, sur B-GCI sans concentration, le piège suivant : départ A26, on met « Début » à H27.
Pendant environ une seconde, l'écran montre deux cheminements différents — le panneau de gauche affiche « H1-H27 » avec GCI-1000, GCI-1001, GCI-1011, GLG-1000, alors que `data/cours.json` n'offre GCI-1000 et GCI-1001 qu'à l'automne (le rapport nommait GLG-1000 à la place de GCI-1001 ; le catalogue offre GLG-1000 aux deux saisons — le piège est le même, un cours de moins).
Rien ne signale que cet affichage est transitoire : un export PDF/JSON ou un partage lancé à ce moment-là produit un cheminement erroné.

La cause est mécanique.
`Plan.displayed_placement` associe un sigle à un *indice* de session, jamais à un semestre.
L'ancien gestionnaire du sélecteur écrivait :

```rust
edit_plan(plan, history, &format!("Début déplacé à {value}"), |plan| plan.start = semester);
```

Changer le Début renomme donc toutes les sessions sans déplacer personne : GCI-1000 garde l'indice 1, et l'indice 1 s'appelle maintenant H1-H27.

La réparation, elle, arrivait tard.
`solve::unplaced_codes` ne regarde que « ce cours a-t-il un siège », pas « ce siège est-il tenable » : tous les cours en ont encore un, donc `unplaced` est vide et `solve::propose_needed` renvoie `false`.
Il faut attendre que `auto_verify` parte (500 ms), revienne avec `solutions: []`, puis que `auto_propose` reparte (500 ms de plus) pour que le placement soit corrigé — d'où la seconde observée.

L'ADR `2026-08-recalcul-visible-sur-la-grille` ne couvrait pas ce cas : il marque l'état transitoire à partir de `SolverState.running`, or ici la fausseté existe *avant* qu'aucune requête ne parte.

## Décision

**Le mensonge est retiré à la source plutôt que signalé.**

`state::set_start` devient le point d'entrée unique du changement de Début, sur le modèle exact de `state::set_horizon` (« la seule porte, qui évince les sièges automatiques que le nouvel horizon ne tient plus ») :

- les sièges *automatiques* que la nouvelle saison n'offre pas quittent `displayed_placement` dans le même geste, donc dans le même acte annulable — `edit_plan` clone le `Plan` entier avant l'édition, « Annuler » les rend tous d'un coup ;
- les gestes explicites de l'étudiant sont souverains : un cours épinglé n'est jamais désassis en silence, la vérification le refuse à voix haute comme elle l'a toujours fait ;
- un sigle que le catalogue ne connaît pas (`OPT-ION1`, un cours manuel disparu) n'est pas jugé, donc garde son siège — l'inconnu reste inconnu (TRU-1) ;
- l'horizon est réaffirmé (`set_horizon`) : basculer la saison de départ change la longueur du parcours quand le nombre de sessions est impair, et un siège hors créneau est la même famille de faute.

Les cours évincés redeviennent « à planifier », ce que `unplaced_codes` voit immédiatement : `propose_needed` renvoie `true` dès le premier réveil de `auto_propose`, sans attendre le verdict d'une vérification. La réparation part 500 ms après le geste au lieu d'environ 1 000 ms.

Rien ne disparaît en silence : `present::start_move_note` nomme les sigles retirés, dit pourquoi, et dit qui leur cherche une place. Rien n'est affiché quand rien n'a été retiré (ALR-3).

**L'export dit ce qu'il fige.**
`export::menu::pending_note` ajoute en tête du menu « Exporter ▾ », tant qu'une recherche tourne, la phrase « ⟳ recalcul en cours — un document exporté maintenant fige un placement provisoire. »
Le menu informe, il ne bloque pas : les entrées restent cliquables (AIR §E rejette la boîte « êtes-vous sûr ? », et un export volontairement provisoire reste légitime).

Les trois décisions — quels sièges tombent, quelle phrase les nomme, quel avertissement précède le menu — sont des fonctions pures de `state.rs`, `present.rs` et `export/menu.rs`, testées nativement ; `solve::placed_offerings` leur fournit les saisons offertes sous forme de table possédée, pour que la vue ne tienne aucun emprunt du snapshot pendant que `edit_plan` détient l'écriture du plan (AP-5).

`SolverState.proposed` n'est pas touché : le placement change, donc l'empreinte de la requête change, et la convergence tient comme avant.

**Ce que la décision ne ferme pas, et pourquoi.**
Un cours que la nouvelle saison offre bel et bien (GCI-1011, GLG-1000 dans l'exemple) garde son siège, et le solveur peut malgré tout le déplacer — un préalable réordonné, un plafond de crédits.
Cet état-là n'est pas faux, il est *périmé* : rien à l'écran ne prétend qu'il est vérifié, et c'est exactement ce que `2026-08-recalcul-visible-sur-la-grille` marque dès que la recherche part.
La frontière est là : ce que le catalogue tranche seul, sur-le-champ, se corrige ; ce qui demande le solveur se signale.

## Alternatives rejetées

- **Marquer l'état transitoire au lieu de le corriger** (« recalcul… » sur le panneau et la grille, export bloqué le temps du recalcul).
  C'est ce que faisait déjà `2026-08-recalcul-visible-sur-la-grille`, et c'est précisément ce qui n'a pas suffi : le marquage part de `running`, alors que la fausseté naît au clic, 500 ms plus tôt.
  L'étendre à « le plan a changé et le solveur n'a pas encore répondu » aurait voulu dire afficher « recalcul… » en permanence dans les états stables où `verification` est légitimement `None` (cours encore flottants, aucun programme choisi) — un mensonge de plus, pas un de moins.
  Ce qui se calcule sur-le-champ, sans solveur, ne se signale pas : il se corrige.
- **Vider tout le placement non épinglé au changement de Début.**
  Honnête, mais LAT-6 refuse de blanchir un écran pour le recalculer : les cours que la nouvelle saison offre bel et bien gardent leur siège, et seuls ceux qu'elle refuse le perdent.
- **Estampiller « provisoire » dans le document exporté** plutôt que dans le menu.
  EXP-1 le suggère, mais la recherche dure moins d'une seconde : le fichier porterait souvent une réserve sur un placement devenu final entre-temps — un mensonge dans l'autre sens. L'avertissement vit donc au moment et à l'endroit du geste.
- **Désassir aussi les cours épinglés hors saison.**
  Un geste explicite ne se défait pas en silence ; la vérification les refuse en les nommant (`pinned_refusal_line`), ce qui est la bonne porte.
