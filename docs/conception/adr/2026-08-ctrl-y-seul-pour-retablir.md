# Ctrl+Y seul pour rétablir

## Contexte

Le handler clavier de la coquille (`crates/ui/src/components/shell.rs`) offrait deux raccourcis de rétablissement : `Ctrl+Maj+Z` (miroir de l'annulation) et `Ctrl+Y` (héritage Windows), via le bras de match `"Z" | "y"`.
Le travail sur la découvrabilité (`docs/ux/fonctionnalites.md`) a mis ce doublon en évidence : chaque raccourci à annoncer (kbd sur les boutons, fiche « ? ») coûte de la place, et un doublon double ce coût sans besoin exprimé.
Accessoirement, `Ctrl+Maj+Y` produisait `"Y"`, matché nulle part — un trou du doublon lui-même.

## Décision

`Ctrl+Y` est le seul raccourci de rétablissement ; `Ctrl+Z` reste l'annulation.
Le bras `"Z" | "y"` devient `"y"` (commit `6cb0ede`).

## Alternatives rejetées

- **Garder les deux** : aucun utilisateur ne l'a demandé, et chaque indice de découvrabilité aurait dû porter deux combinaisons pour la même action.
- **Couvrir aussi `"Y"` (`Ctrl+Maj+Y`)** : élargir un doublon plutôt que le retirer.
