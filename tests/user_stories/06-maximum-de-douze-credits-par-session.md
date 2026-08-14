# US-06 — Maximum de douze crédits par session

**Persona** : Rosalie, au B-GEX, qui doit rester sous les douze crédits par session pour des raisons de santé.
**Intention** : étaler son cheminement en ne dépassant jamais quatre cours de trois crédits par session.

## Préconditions

- Programme « B-GEX », cheminement type chargé (cinq cours par session d'automne et d'hiver).

## Scénario

1. Rosalie retire un cours de chaque session chargée et le pousse vers une session ultérieure.
2. Elle ajoute des colonnes avec le bouton « + » jusqu'à couvrir six années.
3. Elle vérifie session par session que le nombre de pastilles n'excède jamais quatre.

## Résultats attendus

- Le bouton « + » ajoute la session suivante dans l'ordre A → H → E → A, en incrémentant l'année au passage de l'automne à l'hiver.
- Chaque déplacement relance la vérification complète : préalables, offre, conflits, bilan.
- Repousser un cours vers une session plus tardive ne casse jamais un préalable; l'inverse, oui.

## Repères pour le test e2e

- Compter les `.dropped-tile` par index de colonne : le maximum vaut 4.
- Après `#btn-ajouter-colonne`, le dernier `thead th` suit la séquence attendue (`H30` → `E30` → `A30` → `H31`).

## Variantes et cas limites

- **Fonctionnalité à venir** : un plafond de crédits par session est une contrainte que le solveur de placement du dépôt `generateur_horaire` accepte déjà (`credit cap`). L'histoire décrit aujourd'hui un étalement manuel; elle deviendra « Rosalie saisit 12 comme plafond et la grille se réorganise ».
- Le total accumulé affiché dans le bilan ne dépend pas du plafond : il compte tout ce qui est placé.
- Une session laissée entièrement vide est légitime (interruption d'études) et ne doit produire aucune alerte.
