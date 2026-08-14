# US-23 — Charger un cheminement type

**Persona** : Élodie, nouvellement admise, qui veut partir de la grille officielle de son programme.
**Intention** : remplir sa grille en un clic.

## Préconditions

- `b-gex/cheminements-types/index-cheminements-types.csv` associe une session d'admission, une étiquette et un fichier.

## Scénario

1. Élodie clique « Charger un cheminement ».
2. La fenêtre liste les cheminements types de sa session d'admission.
3. Elle clique le bouton « A26 ».

## Résultats attendus

- La fenêtre s'ouvre avec « Chargement… » puis la liste réelle.
- Seuls les cheminements dont la première colonne correspond à la session d'admission courante sont proposés.
- Le clic ferme la fenêtre, vide la grille, puis replace les cours du fichier ligne par ligne.
- La première ligne du fichier va dans la colonne « Cours complétés », quelle que soit son étiquette; les suivantes sont appariées par code de session.
- Les cours placés apparaissent grisés dans le panneau de droite (classe `placed`).

## Repères pour le test e2e

- `#modal-chargement` reçoit la classe `visible`.
- `#modal-cheminements-dynamiques button` compte au moins une entrée pour `b-gex` + `A26`.
- Après le clic, `#modal-chargement` perd `visible` et `.dropped-tile` est non vide.
- `.rules-panel .course-tile.placed` compte autant d'éléments que de cours placés appartenant au programme.

## Variantes et cas limites

- Un clic sur le fond gris ferme la fenêtre sans rien charger; un clic dans la boîte ne la ferme pas.
- Si `index-cheminements-types.csv` est absent, la fenêtre affiche « Impossible de charger la liste des cheminements : … » en rouge.
- Si aucun cheminement ne correspond à la session d'admission, le message le dit explicitement en nommant la session.
- Un fichier de cheminement citant un sigle inconnu place quand même une pastille, sans titre ni crédits.
- Le fichier `index-cheminements-types.csv` commence par un BOM UTF-8 : le premier code de session doit malgré tout être reconnu.
