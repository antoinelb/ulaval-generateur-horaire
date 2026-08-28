# Le panneau défile vers ce que le geste vient d'ouvrir

## Contexte

Le panneau est un long défileur interne (le body ne bouge jamais) : une règle dépliée ou des résultats de recherche atterrissent souvent sous le pli, et les deux testeuses ont cliqué « dans le vide » en croyant l'action sans effet (rapports du 2026-08-19).

## Décision

- Deux accroches `onmounted` + `MountedData::scroll_to_with_options` (aucune feature web-sys à ajouter) : le contenu d'une section qu'on vient de déplier, et le bloc de résultats quand on tape une recherche.
- **Seul un geste défile** (LAT-7) : un signal armé par le clic de dépliage (resp. la frappe dans le champ) et consommé au montage — une page restaurée avec sa règle dépliée ou sa recherche sauvegardée monte les mêmes nœuds sans bouger.
- `ScrollLogicalPosition::Nearest` + `Smooth` : déjà visible ⇒ aucun mouvement (ERR-6, ne jamais perdre la position) ; sous le pli ⇒ le déplacement minimal.

## Alternatives rejetées

- **`scrollIntoView` via eval JS** : Dioxus 0.7 porte l'API nativement.
- **Défiler depuis un effet de données** : c'est l'auto-rafraîchissement qui défile — interdit par LAT-7 et la source du bogue classique « la page saute pendant que je lis ».

## Conséquence (2026-08-28)

`Nearest` s'arrête dès qu'un bord du contenu déplié devient visible — sur une règle plus haute que le panneau, c'est son bord bas qui satisfait `Nearest` en premier, laissant l'en-tête et les premiers cours de la règle toujours hors champ (rapport étudiante-gex 2026-08-27).
`SectionView` passe donc à `vertical: ScrollLogicalPosition::Start` (horizontal reste `Nearest`, rien à corriger sur l'axe latéral) ; le mécanisme armé par le clic puis consommé par `onmounted` ne change pas.

Le même défaut touchait trois autres replis natifs (`<details>`) : le détail technique d'une erreur Capsule, les préalables corrigibles (`PrereqField`) et le formulaire « Cours absent du catalogue ? ». Un composant partagé, `Disclosure` (`crates/ui/src/components/panel.rs`), leur apporte le même comportement.
`Disclosure` diffère de `SectionView` sur un point technique : `SectionView` est un div/bouton maison dont le contenu n'existe dans le DOM que déplié — un div qui apparaît reçoit un `onmounted` neuf à chaque ouverture, ce qui porte le mécanisme « armé au clic, consommé au montage ». Un vrai `<details>` contrôlé, lui, n'est jamais démonté quand on bascule son attribut `open` : le navigateur masque son contenu sans le retirer du DOM, donc `onmounted` n'y tirerait qu'une seule fois, à la création de la page. `Disclosure` capture donc la référence du nœud une seule fois (`onmounted`) et déclenche le défilement directement depuis le clic sur le résumé, quand ce clic est celui qui ouvre — un geste réel à chaque fois, jamais un montage ou une restauration, donc LAT-7 tient tout autant.
Les résultats de recherche (`PanelBody`, armés à chaque frappe plutôt qu'à un clic d'ouverture) restent volontairement à `Nearest` : `Start` les aurait fait remonter en haut du panneau à chaque lettre tapée, alors que `Nearest` s'arrête sitôt les résultats visibles.
