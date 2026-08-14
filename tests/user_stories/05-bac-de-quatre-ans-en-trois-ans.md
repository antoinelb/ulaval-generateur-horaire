# US-05 — Terminer un bac de quatre ans en trois ans

**Persona** : Hugo, admis au B-GEX à l'automne 2026, qui veut diplômer en avril 2029.
**Intention** : compresser 120 crédits en neuf sessions, étés compris.

## Préconditions

- Programme « B-GEX », session d'admission « A26 », cheminement type chargé.

## Scénario

1. Hugo remonte des cours des dernières sessions vers les sessions d'été E27 et E28, jusqu'ici presque vides.
2. Il charge chaque session d'automne et d'hiver à six cours au lieu de cinq, ce qui ajoute une rangée à la grille.
3. Il supprime les colonnes A29 et H30, devenues vides, avec le bouton « − » au-dessus du tableau.
4. Il relit le journal.

## Résultats attendus

- La grille accepte plus de cinq cours par session : une rangée est ajoutée automatiquement dès qu'une colonne déborde.
- Un cours déplacé vers une session où il n'est pas offert est signalé `cours-non-offert` — c'est le principal frein réel à la compression, l'été n'offrant qu'une fraction du catalogue.
- Un cours déplacé avant son préalable est signalé, même si le total de crédits reste correct.
- La suppression d'une colonne est refusée tant que celle-ci contient une pastille.
- Le total du bilan atteint `120 cr. / 120 cr.` malgré la compression.

## Repères pour le test e2e

- Après un dépôt dans une colonne pleine, `.table-wrapper tbody tr` compte une rangée de plus.
- `#btn-retirer-colonne` ne modifie pas le nombre de `thead th` quand la dernière colonne contient une pastille.
- Le nombre de `thead th` ne descend jamais sous 2.

## Variantes et cas limites

- Compresser en gardant `GEX-1580` (stage, 9 crédits, préalable « Crédits exigés : 24 ») trop tôt déclenche l'erreur de crédits accumulés (US-41).
- Les crédits des stages sont *en sus* : ils ne font pas avancer le compteur des 120 crédits ni celui des préalables en crédits.
- Une compression extrême — tout en trois sessions — doit rester possible mécaniquement; l'application avertit, elle n'interdit pas.
