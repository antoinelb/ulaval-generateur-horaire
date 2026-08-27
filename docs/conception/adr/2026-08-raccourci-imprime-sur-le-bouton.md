# Raccourci imprimé sur le bouton

Date : 2026-08-27

## Contexte

`docs/ux/fonctionnalites.md` (Partie 1) cotait `Ctrl+Z` / `Ctrl+Y` / `Échap` « caché » : les boutons Annuler/Rétablir existent, mais rien à l'écran n'annonce leurs raccourcis, et Échap n'a aucun équivalent visible du tout.
La Partie 2 du même document propose le patron P1 (« raccourci imprimé sur le contrôle ») comme correctif conforme à AIR : un `<kbd>` affiché à demeure sur le bouton qu'il double, jamais seulement au survol (règle cœur AIR) ni seulement dans le `title` (LAY-5 exige un équivalent pointeur *découvrable*, pas caché derrière un hover).

## Décision

- `Ctrl+Z` et `Ctrl+Y` s'impriment en `<kbd>` directement dans les boutons « ↶ Annuler » et « ↷ Rétablir » de la bande d'état (`crates/ui/src/components/header.rs`), après le libellé.
- `Échap` n'a pas de bouton à annoter : la légende de la grille (`crates/ui/src/components/grid.rs`) le nomme dans sa clause sur le pointillé — « cliquer pour la forcer, Échap pour refermer ».
- Style du `kbd` (`crates/ui/assets/main.css`, `.status-undo kbd`) : police monospace du projet à `0.7em` (nettement sous le texte du bouton), couleur atténuée (`--muted-2`), fin liseré, coins arrondis, visible au repos — pas de hauteur de bouton modifiée (`line-height` maîtrisé).
- `docs/ux/fonctionnalites.md` : les trois raccourcis passent de « caché » à « évident » (sections 8 et 11), et sortent de la liste des fonctionnalités cachées par gravité.

## Alternatives rejetées

- **Fiche « ? » listant les raccourcis (patron P4)** : robuste mais ne montre rien au repos sur le bouton lui-même — l'utilisateur doit encore savoir qu'un raccourci existe pour aller le chercher.
- **Raccourci seulement dans le `title` existant** : c'est déjà le cas aujourd'hui et c'est précisément ce qui vaut la cote « caché » — une affordance au survol seul est explicitement rejetée par AIR (règle cœur, INP).
- **Étendre le patron à d'autres raccourcis** : hors mandat — Ctrl+Z, Ctrl+Y et Échap sont les trois seuls raccourcis de l'application (`crates/ui/src/components/shell.rs`).

## Conséquences

- Le bouton « ↶ Annuler » des toasts (retrait d'un cours/programme) n'est pas concerné : c'est un geste transitoire sans raccourci clavier associé, pas le même contrôle que la bande d'état.
