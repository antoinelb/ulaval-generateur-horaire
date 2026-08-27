# Compteur de recherche à largeur réservée

Date : 2026-08-27

## Contexte

`SolverStatus` (`crates/ui/src/components/header.rs`) rend « {what} - {elapsed} s » suivi du bouton « Annuler la recherche » dans le même `span.status-running`.
Une minuterie (`use_future`, tick 1 s) écrit `elapsed` chaque seconde ; son texte s'élargit à chaque palier de chiffre (9 s → 10 s, 99 s → 100 s), ce qui décale le bouton pendant que l'utilisateur vise le clic — en violation de LAY-2.

## Décision

Isoler le compte dans son propre `span.status-running-elapsed`, avec `font-variant-numeric: tabular-nums`, `min-width: 2.75rem` (loge « 999 s ») et `text-align: right`.
999 s ≈ 16 min est bien au-delà d'une recherche réelle avant annulation ; au-delà de cette borne le texte s'élargirait quand même, ce cas est assumé plutôt que couvert.
Le bouton « Annuler la recherche » ne bouge donc plus à chaque tick.

## Alternatives rejetées

- Police à chasse fixe pour tout `.status-running` : change l'apparence du libellé « {what} » sans nécessité, pour un problème qui ne touche que le compte.
- Tronquer l'affichage à deux chiffres (« 99+ s ») : perd l'information exacte au-delà de 99 s sans raccourcir le risque de décalage plus tôt que 100 s.
