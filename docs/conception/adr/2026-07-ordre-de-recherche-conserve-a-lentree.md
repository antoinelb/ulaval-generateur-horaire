# Ordre de recherche conservé à l'ordre d'entrée (fail-first rejeté)

## Contexte

En optimisant l'énumération de B (2026-07-31), l'heuristique CSP classique « fail-first » (placer d'abord le cours au plus petit domaine) a été essayée : tri des candidats par (taille de domaine, code).
Elle promettait de tuer les branches condamnées près de la racine.

## Décision

Conserver l'ordre d'entrée des cours (la liste du programme, quasi topologique puisque le curriculum liste les préalables avant leurs dépendants).
Mesure sur la commande de référence (bac GEX + 6 électifs, cap 16, 100 000 solutions) : fail-first régresse de 1,7 s à 2,9 s ; sur le cap serré 13, gain marginal (5,7 → 5,3 s).
L'énumération dense profite d'un ordre où les préalables précèdent leurs dépendants (vérdicts décidés tôt, moins de re-vérifications), ce que l'ordre d'entrée donne déjà ; le cas pathologique que fail-first visait (cours condamné placé tard) est couvert par le filtre d'implaçabilité (ADR `2026-07-implacabilite-prouvee-avant-la-recherche`).

## Alternatives rejetées

- Tri (taille de domaine, code) : mesuré, régression sur le cas nominal.
- Tri topologique explicite par préalables : l'ordre d'entrée l'approxime déjà ; à revisiter si un programme réel arrivait dans un ordre hostile.
