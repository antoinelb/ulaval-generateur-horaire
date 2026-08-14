# US-10 — Changement de programme

**Persona** : Océane, qui passe du B-GMC au B-GIN après une année, avec la plupart de ses cours reconnus.
**Intention** : repartir de la grille du nouveau programme sans ressaisir ses cours réussis.

## Préconditions

- Une grille remplie sous « B-GMC ».

## Scénario

1. Océane sauvegarde son cheminement actuel en CSV.
2. Elle change « Programme » pour « B-GIN ».
3. Elle recharge son CSV par « Charger un cheminement » → « Parcourir… ».
4. Elle déplace vers « Cours complétés » les cours reconnus par le nouveau programme.

## Résultats attendus

- Le changement de programme recharge le catalogue, l'index des millésimes, le menu des spécialisations et le panneau de règles.
- **La grille est vidée** au passage : sans la sauvegarde préalable, le travail est perdu.
- Le CSV rechargé replace chaque sigle dans la colonne dont l'en-tête porte le même code de session; une session absente de la nouvelle grille fait apparaître une colonne supplémentaire.
- Un cours du B-GMC qui n'appartient à aucune règle du B-GIN reste affichable dans la grille — il vient du catalogue complet — mais ne compte dans aucune règle du bilan.

## Repères pour le test e2e

- Après `#programme-select` changé, `#programme-subtitle` nomme le nouveau programme et `.dropped-tile` est absent.
- Après rechargement du CSV, le nombre de `.dropped-tile` est celui du fichier.
- Une ligne du CSV dont le code de session est inconnu ajoute un `thead th` portant ce code.

## Variantes et cas limites

- Le catalogue `data/cours.json` n'est téléchargé qu'une fois, puis relu : changer de programme plusieurs fois ne doit pas provoquer de nouvelle requête réseau.
- Le fichier `cours-hors-catalogue.csv` est propre à chaque programme : un pseudo-cours du B-GEX n'a ni titre ni crédits sous le B-GIN.
- Changer de programme puis revenir doit redonner exactement le même panneau qu'au départ.
