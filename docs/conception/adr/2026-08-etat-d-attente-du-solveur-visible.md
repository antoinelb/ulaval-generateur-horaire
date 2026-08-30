# L'attente d'une réponse du solveur est un état de première classe, visible dès la première image

Date : 2026-08-30

## Contexte

Deux rapports du 2026-08-29 décrivent le même défaut par deux symptômes.

**Bernard, directeur de programme.**
Charger un cheminement complet, changer « Début », capturer l'écran immédiatement : l'écran affiche transitoirement « 30/120 cr » au lieu du 105/120 final et des sessions « à planifier », **sans aucun signe de calcul en cours**.
Deux captures, mêmes entrées, quelques instants d'écart.

**Élodie, finissante au cégep.**
Choisir génie civil, sélectionner « Eau et environnement », cliquer immédiatement sur le triangle « ▸ » de « Règle 1 » : le clic n'ouvre pas la règle, et il a une fois ramené le sélecteur de concentration à « Cheminement sans concentration ».
Le même triangle a souvent demandé deux ou trois clics, sans aucun retour visuel entre les tentatives ratées.
Une pause d'environ 1,5 s avant de cliquer faisait disparaître le problème.

La cause commune est une **fenêtre d'attente que rien n'annonçait**.

`auto_propose` et `auto_verify` attendent 500 ms après le dernier changement de plan avant d'envoyer quoi que ce soit ; `SolverState.running` n'est posé qu'à l'envoi (`next_query`).
Pendant toute cette temporisation, `running` vaut `None` : le voile de la grille, le statut « ⟳ recalcul en cours… » et le verdict du panneau — tous conditionnés à `running.is_some()` par l'ADR `2026-08-recalcul-visible-sur-la-grille` — restent muets.
L'écran affiche un état intermédiaire réel (le placement que `set_start` vient d'amputer des cours hors saison) avec l'apparence d'un résultat arrêté.
Le 1,5 s d'Élodie, c'est exactement cette temporisation plus l'aller-retour du solveur.

À cela s'ajoutait un second trou : l'effet de changement de plan faisait `state.verification = None`.
Le bloc `panel-verdicts`, qui n'a pas de hauteur réservée (ADR `2026-08-verdicts-du-panneau-sans-hauteur-reservee`), se vidait donc puis se remplissait à nouveau une seconde plus tard — et **tout ce qui est en dessous, chaque entête « Règle N ▸ », se déplaçait deux fois sous le curseur** (LAT-7 : « auto-refresh never moves anything under the pointer »).
Le verdict `verification_verdict(searching)` prévu par l'ADR `2026-08-recalcul-visible-sur-la-grille` ne pouvait d'ailleurs jamais s'afficher pendant un recalcul : son `if let Some(verification)` venait d'être mis à `None` par ce même effet.

## Décision

**1. « Une réponse est attendue » devient un fait porté par l'état, décidé par une fonction pure.**

`SolverState.awaited_since: Option<u64>` est posé par `track_plan_change` **au changement de plan**, avant toute requête, et effacé quand une réponse se pose (`handle_worker_answer`), à l'annulation (`cancel_search`), ou par une minuterie si la temporisation retombe sans qu'aucune requête ne soit partie — une annonce ne survit jamais à l'attente qu'elle décrit.

`crate::solve::awaited_ms(awaited_since, running_started_ms, now_ms) -> Option<u64>` décide seule, et est testée nativement : le plus ancien des deux horodatages gouverne, pour que le compteur ne recule pas au moment où la temporisation devient une vraie recherche ; une horloge reculée donne 0, jamais un âge enroulé.
`crate::solve::RECALC_DEBOUNCE_MS` est désormais la définition unique des 500 ms, partagée par les deux effets et par la minuterie.

**2. Un indicateur explicite avec son temps écoulé (LAT-4), dans la ligne de verdict du panneau.**

> **Remplacé le 2026-08-30** par l'ADR `2026-08-attente-du-solveur-dans-la-bande-de-statut` : `RecalcNotice` et `.panel-recalc` n'existent plus, la bande de statut portant seule l'annonce et son compteur. La hauteur réservée dans le panneau était une bande blanche permanente ; la bande de statut n'a rien à réserver. Le reste du présent ADR — `awaited_since`, `awaited_ms`, `held_while_awaited`, la hauteur réservée du verdict d'état — reste en vigueur.

`RecalcNotice`, premier enfant de `div.panel-verdicts`, affiche « ⟳ Recalcul du placement… N s — valeurs de la solution précédente. » (`present::recalc_notice`, pure et testée).
Composant à part, pour deux raisons : sa minuterie d'une seconde ne re-rend que cette ligne — dans `OrganigrammeControls` elle relancerait `conflicted_sessions` (l'horaire hebdomadaire de chaque session) chaque seconde, LAT-3 — et la ligne est **toujours montée**, hauteur réservée par `.panel-recalc`, pour que son apparition ne déplace rien (LAY-1).
Le bouton « Annuler la recherche » reste où il est, dans la bande de statut : on n'annule que ce qui est parti.

`awaited_since` remplace `running` comme condition du voile de la grille et du ruban, et de la note « un export lancé pendant un recalcul fige un état provisoire ».

**3. Les totaux gardent leur dernière valeur arrêtée (LAT-6).**

`present::held_while_awaited(settled, current, awaited)` rend la dernière valeur arrêtée pendant l'attente, avec un booléen qui dit qu'elle date ; sans valeur arrêtée (tout premier calcul), la valeur courante passe, non marquée.
L'entête l'applique à `panel::credit_readout`, qui compose **les deux totaux d'un seul tenant** — les tenir séparément laisserait afficher un total de bac et un total de session décrivant deux cheminements différents.
La valeur tenue est atténuée (`.header-credits--stale`) et le dit en toutes lettres dans son `title` : l'opacité seule ne porte jamais un fait (INP-3).

**4. Le bloc de verdicts garde sa hauteur pendant toute l'attente.**

Trois changements s'additionnent pour y arriver, et il fallait les trois.

- `state.verification = None` au changement de plan devient `state.verification_stale = true`.
  Le verdict reste, et `verification_verdict(true)` le rend enfin visible tel que l'ADR `2026-08-recalcul-visible-sur-la-grille` l'avait prévu : « ⟳ recalcul en cours… (le verdict précédent ne s'applique plus) », jamais un ✓.
  Trois gardes suivent le drapeau : `auto_verify` revérifie quand le verdict est périmé, `auto_propose` ne répare que sur un « aucune solution » **frais**, et `swap_document` efface le verdict lui-même — il jugeait le document quitté.
- Tout ce que le bloc affiche est rassemblé dans un `VerdictFacts` unique, tenu par le même `held_while_awaited` que les totaux.
  Sans cela la ligne « N cours sans session — placement automatique en cours… » apparaissait puis repartait, à elle seule 50 px.
- Les deux formulations du verdict d'état partagent la classe `panel-verdict--state`, dont le `min-height` réserve trois lignes : la version « en cours » est plus courte d'une ligne que le ✓, et cette ligne manquante valait 21 px.
  La ligne d'attente elle-même réserve ses deux lignes (`.panel-recalc`).

Mesuré au navigateur, en échantillonnant `getBoundingClientRect().top` des quatre premiers entêtes de règle toutes les 50 ms pendant un changement de « Début » sur B-GCI A26 : `1002,1058,1114,1182` → `1052,1108,1164,1232` → `983,1039,1095,1163` → retour, soit deux déplacements de 50 puis 69 px avant ; `1004,1060,1116,1184` du début à la fin après, hauteur du bloc constante à 180 px.
Un clic simulé 150 ms après un changement de concentration, aux coordonnées relevées **avant** le changement, atterrit maintenant sur l'entête visé et le déplie.

## Ce que ce n'est pas

Le sélecteur de concentration **ne peut pas** recevoir un `onchange` déclenché par un re-rendu, hypothèse envisagée pour le symptôme d'Élodie.
Dioxus pose `selected` en propriété IDL et non en attribut (`dioxus-interpreter-js/src/js/core.js` : `case"selected":node.selected=truthy(value)`), et l'affectation de `HTMLOptionElement.selected` ne déclenche aucun événement ; `onchange` est câblé sur l'événement DOM `change`, que seule une interaction humaine émet.
`set_scope` sort d'ailleurs immédiatement quand la valeur n'a pas changé.
Le sélecteur n'a jamais bougé tout seul : c'est la mise en page sous le curseur qui bougeait.

## Alternatives rejetées

- **Poser `running` dès l'armement de la temporisation** plutôt qu'un champ neuf — `running.id` identifie une requête réelle à laquelle une réponse est appariée (`handle_worker_answer`) et que « Annuler la recherche » tue ; en fabriquer une qui n'existe pas aurait rendu annulable une recherche qui n'a pas commencé et appariable une réponse qui ne viendra pas.
- **Réserver la hauteur de tout le bloc `panel-verdicts`** (boîte à hauteur fixe qui défile en interne) — cela aurait figé la mise en page pour de bon, mais en cachant des avertissements derrière un défilement interne de trois lignes.
  L'ADR `2026-08-verdicts-du-panneau-sans-hauteur-reservee` reste donc en vigueur pour le bloc dans son ensemble : seules la ligne d'attente et la ligne de verdict d'état ont une hauteur réservée, parce qu'elles ont chacune un nombre de formulations connu et fini.
  Ce que le bloc a gagné n'est pas une hauteur fixe, c'est de ne plus changer **pendant** l'attente.
- **Descendre les verdicts sous la liste des règles**, pour que rien au-dessus d'elles ne change de hauteur — les verdicts auraient quitté les réglages qu'ils expliquent, et se seraient retrouvés sous une liste longue de plusieurs écrans.
- **Bloquer les clics pendant le recalcul** — déjà rejeté par l'ADR `2026-08-recalcul-visible-sur-la-grille`, et pour la même raison : le recalcul se redéclenche à chaque frappe, l'interface serait saccadée.
- **Afficher la valeur intermédiaire, simplement atténuée**, plutôt que la tenir — un « 30/120 cr » n'a jamais décrit un cheminement ; l'atténuer n'en fait pas un chiffre vrai (Core Ten : « l'interface ne ment jamais sur ce qu'elle sait »).
