# US-30 — Retirer un cours de la grille

**Persona** : Étienne, qui abandonne un cours à option choisi trop vite.
**Intention** : le sortir de sa grille et le remettre à disposition dans le panneau.

## Préconditions

- Une grille contenant `DDU-2000`.

## Scénario

1. Étienne glisse la pastille `DDU-2000` depuis la grille vers le panneau de droite.

## Résultats attendus

- La pastille disparaît de la grille.
- La ligne du panneau reprend son apparence non placée.
- Les rangées vides excédentaires sont supprimées, dans la limite de cinq rangées minimum.
- Le bilan des crédits est recalculé sans ce cours, et les cours qui l'exigeaient en préalable sont désormais signalés.

## Repères pour le test e2e

- La zone de dépôt est `#rules-list` : elle accepte `dragover` et `drop`.
- Après le dépôt, `.dropped-tile[data-code="DDU-2000"]` n'existe plus et `.course-tile[data-code="DDU-2000"].placed` non plus.
- La ligne `Total :` du journal a diminué du nombre de crédits du cours.

## Variantes et cas limites

- Déposer sur le panneau un sigle qui n'est pas dans la grille ne fait rien.
- Retirer un cours placé dans « Cours complétés » fonctionne de la même façon.
- Il n'existe **pas** de bouton de suppression sur la pastille : seul le glisser-déposer retire un cours, ce qui est difficile à découvrir et impossible au clavier.
- Vider entièrement la grille est possible cours par cours; il n'existe pas de commande « tout effacer », sauf par changement de programme ou de millésime.
