# US-34 — Choisir la section d'un cours

**Persona** : Raphaël, qui veut éviter le laboratoire du vendredi après-midi.
**Intention** : changer la section d'un cours pour dénouer un conflit d'horaire.

`IFT-1903` a quatre combinaisons d'inscription à l'automne 2026, `MAT-0130` en a sept.
Une combinaison est une inscription complète : cours magistral et laboratoire pris ensemble.

## Préconditions

- `IFT-1903` placé dans une colonne d'automne.

## Scénario

1. Raphaël ouvre le menu déroulant sur la pastille `IFT-1903`.
2. Il choisit une autre section.
3. Il ouvre la grille horaire de session pour vérifier.

## Résultats attendus

- Le menu n'apparaît que lorsque le cours a **plus d'une** section pour la saison de la colonne.
- Les options sont nommées par les sections jointes par `+`, ou par leurs NRC à défaut.
- Changer de section relance la vérification des conflits et rafraîchit la fenêtre de grille horaire si elle est ouverte.
- Le menu ne déclenche pas le glisser-déposer de la pastille.
- Déplacer la pastille dans une autre colonne reconstruit le menu et **perd** la section choisie : le choix repart sur la première option.

## Repères pour le test e2e

- `.dropped-tile[data-code="IFT-1903"] select.section-select` existe et compte quatre `option`.
- Un cours à une seule section n'a aucun `select`.
- `selectOption` sur ce menu modifie la présence de `.cours-en-conflit`.

## Variantes et cas limites

- Une session sans horaire publié réutilise l'horaire de la même saison la plus récente : un cours placé en A29 affiche les sections de A26.
- Une colonne d'hiver pour un cours offert seulement à l'automne n'a aucune section : pas de menu, et le cours est signalé non offert.
- Le choix de section n'est ni sauvegardé dans le CSV ni restauré au rechargement.
