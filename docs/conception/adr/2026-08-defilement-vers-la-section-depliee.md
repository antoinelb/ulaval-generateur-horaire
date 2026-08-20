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
