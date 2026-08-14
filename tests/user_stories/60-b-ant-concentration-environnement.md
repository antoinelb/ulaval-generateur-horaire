# US-60 — B-ANT, concentration « Environnement »

**Persona** : Amélie, admise au baccalauréat en anthropologie, qui veut travailler sur les enjeux environnementaux.
**Intention** : voir les cours de sa concentration et suivre sa progression vers les 60 crédits qu'elle exige.

## Préconditions

Le B-ANT n'est pas encore servi par ce dépôt.
Pour jouer le scénario : déposer `data/programmes/B-ANT-A26.json` (il existe dans `../generateur_horaire/data/programmes/`), ajouter `b-ant;Baccalauréat en anthropologie` à `index-programmes.csv`, créer `b-ant/cours/cours-hors-catalogue.csv` et régénérer `data/programmes/index.json`.

## Ce que la concentration ajoute

- Un cours obligatoire : `ANT-2303`.
- Règle 1 : 9 crédits parmi 16 cours (`ANT-1200`, `ANT-1201`, `ANT-1202`…).
- Règle 2 : 30 à 36 crédits parmi 47 cours (`ANT-1105`, `ANT-1007`, `ANT-2304`…).
- Règle 3 : 0 à 6 crédits de langue (`ANL-2020`, `ANL-3010`, `FLS-2093`…).
- Règle 4 : 9 à 18 crédits hors discipline (`BIO-1910`, `DRT-1721`, `ENV-1010`…).
- `credits_required` vaut 60 : la section porte donc un total dans son en-tête de bilan.

## Scénario

1. Amélie choisit « B-ANT » puis la concentration « Environnement ».
2. Elle place `ANT-2303` et quelques cours de la Règle 2.
3. Elle lit l'en-tête de section du bilan.

## Résultats attendus

- L'en-tête du bilan affiche `Environnement : X cr. / 60 cr.` — les concentrations à `credits_required` sont les seules à afficher un total de section.
- Les quatre règles apparaissent avec leurs bornes, les intervalles étant affichés sous la forme `min à max cr.`
- La contribution de la section au total global est plafonnée à 60 crédits, même si Amélie place davantage.
- Le programme exige 90 crédits au total, pas 120 : le bilan doit le refléter.

## Repères pour le test e2e

- `#cheminement-select` contient `Environnement`, `Études autochtones` et `Profil international`.
- `#log-content` contient une ligne `Environnement : … cr. / 60 cr.`
- La ligne `Total :` cite 90 crédits exigés.

## Variantes et cas limites

- Le B-ANT n'a **aucun** cheminement sans concentration : la première concentration de la liste est sélectionnée d'office au chargement.
- La Règle 3 a un minimum de 0 : elle est satisfaite dès le départ et ne doit jamais être en avertissement.
- Aucun cheminement type n'existe pour ce programme : la fenêtre de chargement affiche le message d'absence.
