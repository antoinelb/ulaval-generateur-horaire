# Toute alerte dont la cause se juge se périme ; les actes passés restent

## Contexte

L'ADR `2026-08-peremption-des-toasts-par-cause` a donné une péremption à trois causes (`LeftOut`, `EmptyGrid`, `SolverError`) et fait partir les `Document` avec leur document.
Les trois personas de l'évaluation du 2026-08-29 ont rapporté que le reste ne se périme pas :

1. « ⚠ Proposition ignorée : elle retirerait GEX-3100 de la grille » survit au retrait de GEX-3100 — la proposition suivante est adoptée sans un mot, et le toast obsolète reste identique, rechargement compris.
2. « ⚠ Concentration « Cheminement sans concentration » sélectionnée par défaut » survit au passage à « Eau et environnement » — le toast nomme encore l'ancienne alors que l'en-tête confirme la nouvelle.
3. Deux bascules de concentration laissent jusqu'à quatre bandeaux empilés par-dessus la grille, à fermer un à un.

Deux mécaniques manquaient. `apply_proposal` **republie** ses sujets à chaque réponse mais n'en **retire** aucun : un sujet sur lequel la réponse se tait (`ProposalKept` quand plus rien ne désassoit, `Assumed`/`Injected`/`SummersForced`/`SetAside`/`Completion` quand la condition tombe) reste à l'écran pour toujours, puisque `AlertCause::Document` n'expire qu'à la bascule de document.
Et la concentration par défaut voyageait dans `DocumentSwap.notes`, poussée en `Sticky` — la cause la moins jugeable de toutes, alors que la phrase décrit précisément l'état courant du plan.

## Décision

**Chaque cause porte son événement de péremption, et le recensement est exhaustif.**

| Cause | Ce qu'elle affirme | Se périme quand |
|---|---|---|
| `Sticky` | un acte passé ou une tolérance du chargement | jamais — jusqu'au rejet (ALR-4) |
| `LeftOut(code)` | ce cours ne tient nulle part | le code ne flotte plus (inchangé) |
| `EmptyGrid` | rien n'a pu être placé | quelque chose est placé, ou plus rien ne flotte (inchangé) |
| `SolverError` | le solveur a refusé la requête | une requête ultérieure répond (inchangé) |
| `Answer` (nouveau) | ce que la **dernière réponse** dit de la grille qu'elle propose | la réponse suivante qui règle une grille ne le dit plus |
| `DefaultConcentration(titre)` (nouveau) | cette concentration a été choisie à la place de l'étudiant | le plan porte une autre concentration |
| `Document` | un acte passé sur le document courant | la bascule de document |

- `AlertCause::Answer` remplace `Document` pour les six avis de `apply_proposal` (`Completion`, `SetAside`, `SummersForced`, `ProposalKept`, `Assumed`, `Injected`).
  La fonction accumule les sujets qu'elle publie (`say`) et appelle `AlertStack::retire_unsaid` **une seule fois**, après le bloc `Injected` : là, la réponse règle la grille, donc elle a dit tout ce qu'elle avait à dire.
  Placé après les deux sorties anticipées (proposition refusée, aucune solution) : ces réponses-là ne changent pas la grille affichée, donc rien de ce que la précédente en a dit n'est devenu faux.
  `retire_unsaid` ne juge que les `Answer` étiquetées d'un sujet : un `Document` reste, un `Answer` sans sujet aussi.
- `DocumentSwap` porte `default_concentration: Option<String>` à part de `notes` (qui reste la tolérance d'une étagère abîmée, `Sticky` à bon droit), et `persist::default_concentration_note` en garde la formulation.
  `swap_document` la pousse **après** `purge_document` — l'annonce appartient au document qui entre, pas à celui qui sort.
- Le jugement lui-même est pur : `alerts::expired(&Alert, &Standing)` — `Standing { floating, something_placed, concentration }` — et `retire_stale_left_out` devient `retire_stale_alerts`, simple câblage (AP-5).
  `floating: None` (catalogue absent, intake refusé) veut dire « injugeable » : les verdicts de placement restent, tandis que la concentration, qui ne demande que le plan, est jugée quand même — l'effet n'a plus de sortie anticipée qui saute la péremption.
- Les deux avis de bascule de bloc (`Ententes retirées…`, `Cours retirés avec l'ancien bloc…`) gardent `Document` — ce sont des actes passés, vrais, avec leur « Annuler » — mais reçoivent les sujets `ScopeGrants` et `ScopeDepartures` : une seconde bascule **remplace** la première au lieu de s'empiler à côté (ALR-3).
- **Le masquage (point 3) ne demande rien de plus.** `Toasts` en montre déjà au plus `TOASTS_VISIBLE = 3` plus le bouton « +N autres » : les quatre bandeaux observés étaient ce plafond atteint par des avis périmés. Trois causes de moins et un sujet unique par bascule ramènent la pile sous le plafond ; aucune nouvelle mécanique de disposition n'est introduite.

## Alternatives rejetées

- **Vider les avis de la réponse avant de republier** (au lieu de tracer ce qui est dit) : la réponse refusée et la réponse sans solution ne publient rien d'autre que leur verdict, et effaceraient des avis encore vrais de la grille qu'elles laissent en place.
- **Périmer aussi les actes passés** (injection, étés forcés, cours retirés avec l'ancien bloc) : arbitrage ALR-6 d'Antoine — ils annoncent quelque chose qui a eu lieu et reste vrai ; les périmer, c'est effacer la trace d'une modification que l'étudiant n'a pas vue passer.
- **Une cause `ScopeChange` retirée par la bascule suivante** : le remplacement par sujet existe déjà et fait exactement ça ; une cause de plus n'aurait rien jugé de neuf.
- **Réserver une hauteur ou déplacer la pile** pour le masquage : ce serait traiter le symptôme d'avis qui n'auraient jamais dû rester, et `2026-08-alertes-en-toasts-flottants` a déjà arbitré le coin bas-droite (LAY : rien ne bouge sous les toasts).
