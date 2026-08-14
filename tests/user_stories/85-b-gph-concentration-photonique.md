# US-85 — B-GPH, concentration « Photonique »

**Persona** : Jérémie, en génie physique, qui vise l'optique et les lasers.
**Intention** : combler sa concentration et comparer avec les concentrations voisines.

## Préconditions

- Programme « B-GPH », session d'admission « A26 ».

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 8 cours (`GEL-2001`, `GEL-4201`, `GEL-4203`, `GPH-4100`, `GPH-4101`, `GPH-4102`, `GPH-4103`, `GPH-4104`).
- `credits_required` vaut 15.

## Scénario

1. Jérémie place cinq cours sous la concentration « Photonique ».
2. Il passe à « Génie médical et biophotonique », puis à « Signaux et communications ».
3. Il revient à « Photonique ».

## Résultats attendus

- Les allers-retours entre concentrations ne vident jamais la grille.
- Un cours partagé (`GPH-4101`, `GEL-2001`, `GEL-4201`) reste compté au bilan dans chaque concentration qui le liste.
- Un cours placé qui n'appartient pas à la concentration affichée reste dans la grille sans compter au bilan.
- Revenir à la concentration de départ redonne exactement le bilan initial.

## Repères pour le test e2e

- Le nombre de `.dropped-tile` est constant sur les quatre changements.
- La ligne `Règle 1 : … cr.` du journal varie selon la concentration affichée, pour la même grille.

## Variantes et cas limites

- Trois concentrations du B-GPH partagent des cours `GEL-` et `GPH-` : c'est le meilleur terrain pour tester qu'un même cours change de rôle sans changer d'apparence.
- Un test de non-régression utile : placer une grille, parcourir les neuf spécialisations, revenir à la première et comparer le journal caractère par caractère.
