# US-21 — Choisir la session d'admission (millésime)

**Persona** : Sabrina, admise au B-GMC à l'automne 2024, qui veut la version du programme de son année.
**Intention** : retrouver les règles telles qu'elles étaient à son admission.

## Préconditions

- `data/programmes/index.json` liste les fichiers `B-GMC-*.json`.

## Scénario

1. Sabrina choisit « B-GMC ».
2. Elle ouvre « Session d'admission » et choisit « A24 ».
3. Elle compare le nombre de crédits exigés avec celui de « A26 ».

## Résultats attendus

- Le menu ne contient que les millésimes du programme sélectionné, triés du plus récent au plus ancien, l'automne primant sur l'été puis l'hiver à année égale.
- Seules les entrées au format `[AHE]\d{2}` sont retenues; tout autre nom de fichier est ignoré.
- Changer de millésime régénère les en-têtes de colonnes à partir de la session choisie et vide la grille.
- Le panneau de règles et le bilan reflètent le millésime choisi.

## Repères pour le test e2e

- L'ordre de `#admission-select option` est décroissant : `A26, H26, A25, H25, …`.
- Le deuxième `thead th` vaut la valeur choisie.
- `#programme-subtitle` contient le millésime.

## Variantes et cas limites

- Si l'index est introuvable, le menu reste vide et aucun programme n'est chargé; le message d'erreur doit rester lisible plutôt que de laisser une page muette.
- Un fichier présent dans le dossier mais absent de `index.json` n'apparaît pas : l'index est la seule source, un hébergement statique ne sachant pas lister un répertoire.
- La session d'admission filtre aussi la liste des cheminements types proposés (US-23).
