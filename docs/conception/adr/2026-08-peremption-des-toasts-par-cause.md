# Les toasts d'échec de placement se périment par cause

## Contexte

Un toast « ANL-1010 : aucune place ne restait… » restait affiché après que le cours avait été placé avec succès ailleurs, ou même retiré du plan (rapport étudiante-gex 2026-08-19).
Deux causes : les toasts ⚠ n'avaient aucun mécanisme de péremption (seuls les ✓ s'auto-effacent, ADR `2026-08-alertes-en-toasts-flottants`) ; et `SolverState.left_out` n'était ni purgé au changement de plan (retiré de l'effet de péremption par l'ADR `2026-08-organigramme-en-continu-sans-bouton` : l'auto-application effaçait ce que sa propre réponse venait de rapporter) ni remis à zéro sur une réponse sans solution.

## Décision

- `Alert` porte une cause : `Sticky` (défaut — vit jusqu'au rejet, ALR-4), `LeftOut(code)`, `EmptyGrid`, `SolverError`. `push_alert` garde sa signature ; `push_caused_alert` étiquette.
- `SolverError` (« Le solveur n'a pas pu répondre — … ») se retire dès qu'une requête ultérieure répond : le refus décrivait une entrée qui n'existe plus (contre-test étudiante-gex 2026-08-20 : « pinned outside 1..=4 » survivait au retour à 8 sessions).
- `Document` étiquette les annonces de `apply_proposal` (injection, étés forcés, acquis présumés, mis de côté, verdict de complétude) : elles partent avec leur document — `swap_document` ne laisse survivre que les `Sticky` (« GMC-3020 en été » s'affichait sous B-GIN, contre-test étudiante-cegep 2026-08-20).
- Un effet `retire_stale_left_out` (abonné à `plan`/`snapshot`/`manual`, lisant `solver_state` et `alerts` par `peek`) recalcule les cours flottants (`unplaced_codes`) et retire ensemble l'entrée `left_out` et le toast `LeftOut(code)` dès qu'un code ne flotte plus (placé par une réponse ultérieure, placé à la main, retiré, crédité) ; `EmptyGrid` se retire dès que quelque chose est placé ou que rien ne flotte.
- **Le bogue d'auto-effacement ne revient pas** : les codes qu'une réponse vient de rapporter ne figurent pas dans le `displayed_placement` qu'elle écrit — ils flottent toujours, `stale_left_out` les garde. Ceci supersède la clause « l'effet ne l'efface plus » de `2026-08-organigramme-en-continu-sans-bouton` : la péremption revient, mais par cause, jamais en bloc au changement de plan.
- `apply_proposal` réécrit `left_out` sur **toute** réponse, y compris sans solution (il restait figé).

*Étendu le 2026-08-29 par `2026-08-peremption-de-toute-alerte-jugeable` : l'effet s'appelle désormais `retire_stale_alerts`, juge en plus la concentration choisie par défaut et délègue le verdict au pur `alerts::expired`.
Les avis de `apply_proposal` quittent `Document` pour une cause `Answer`, que la réponse suivante périme dès qu'elle ne les répète plus ; `Document` ne garde que les actes passés.*

## Alternatives rejetées

- **Affichage dérivé de `left_out` au lieu de toasts poussés** : l'ADR toasts a arbitré le push, et le panneau porte déjà le verdict dérivé (« N cours sans session »).
- **Re-périmer en bloc au changement de plan** : c'est le bogue que 6f36d0c corrigeait — l'auto-application est elle-même une édition.
