# Légende de grille permanente

Date : 2026-08-27

## Contexte

`p.grid-legend` (`crates/ui/src/components/grid.rs`) n'était rendue que si `schedule.read().report.courses` n'était pas vide.
La session gagne ses premiers cours de façon asynchrone : `auto_propose` écrit `plan.displayed_placement` 500 ms après la dernière saisie, pas à la frappe.
La légende apparaissait donc après coup, et la grille de 40,5 rem descendait d'une à deux lignes sous elle — un décalage sans geste de l'utilisateur, en violation de LAY-2.

## Décision

Rendre `p.grid-legend` inconditionnellement.
Elle décrit la notation de la grille elle-même (plein, pointillé, hachuré, ⇄ N), pas son contenu — elle a donc un sens même quand la session est vide, et sa présence permanente réserve sa place une fois pour toutes.

## Alternatives rejetées

- Réserver la hauteur de la légende par CSS (`min-height`) en la gardant conditionnelle : ajoute un vide qui ne sert jamais avant le premier placement, pour reproduire ce que le rendu inconditionnel obtient directement.
