# US-24 — Charger un cheminement depuis un fichier local

**Persona** : Marc-Antoine, qui reprend le cheminement qu'il avait sauvegardé la session dernière.
**Intention** : retrouver sa grille exactement comme il l'avait laissée.

## Préconditions

- Un fichier `cheminement.csv` produit par la fonction de sauvegarde (US-26).

## Scénario

1. Marc-Antoine clique « Charger un cheminement » puis « Parcourir… ».
2. Il choisit son fichier.

## Résultats attendus

- La fenêtre se ferme et la grille est remplacée par le contenu du fichier.
- Le séparateur est détecté automatiquement : point-virgule ou virgule, selon celui qui est le plus fréquent sur la première ligne.
- Les lignes entièrement vides sont ignorées.
- Un code de session absent de la grille courante ajoute une colonne à droite portant ce code.
- Si une colonne reçoit plus de cours qu'il n'y a de rangées, des rangées sont ajoutées.

## Repères pour le test e2e

- `#input-fichier-cheminement` reçoit le fichier via `setInputFiles`.
- Le nombre de `.dropped-tile` égale le nombre de sigles du fichier.
- Chaque pastille se trouve dans la colonne dont l'en-tête porte le code de sa ligne.

## Variantes et cas limites

- Recharger deux fois le même fichier donne la même grille : le chargement vide d'abord toutes les pastilles.
- Un fichier vide ou ne contenant que des séparateurs ne modifie rien.
- Un fichier contenant deux fois le même sigle place deux pastilles avec le même `data-code` : la deuxième rend ambigus les sélecteurs et les déplacements. Comportement à corriger ou à documenter.
- Le champ de fichier est vidé à chaque ouverture de la fenêtre, pour qu'un même fichier puisse être rechargé.
