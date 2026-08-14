# US-90 — M-GEX, un programme de deuxième cycle sans spécialisation

**Persona** : Geneviève, à la maîtrise en génie des eaux avec mémoire.
**Intention** : planifier ses 45 crédits, dont l'essentiel est de la recherche.

C'est le seul programme des données qui n'a **ni concentration ni profil**, et le seul de deuxième cycle.

## Préconditions

Le M-GEX n'est pas encore servi par ce dépôt.
Pour jouer le scénario : déposer `data/programmes/M-GEX-A26.json`, ajouter `m-gex;Maîtrise en génie des eaux - avec mémoire` à `index-programmes.csv`, créer `m-gex/cours/cours-hors-catalogue.csv` et régénérer `data/programmes/index.json`.

## Ce que le programme a de particulier

- `credits_required` vaut 45, non 120.
- `cycle` vaut 2 : tous les cours sont des `7xxx` et des `6xxx`.
- Six cours obligatoires : `GCI-7077`, `GEX-6001`, `GEX-6811`, `GEX-6812`, `GEX-6813`, `GEX-6814`.
- Règle 1 : 12 crédits parmi 16 cours (`GCI-7004`, `GCI-7010`, `GCI-7021`, `GCI-7030`, `GEX-7002`, `GEX-7004`…).
- `possible_semester_start` vaut `["A", "H", "E"]` : l'admission est possible aux trois saisons, contrairement à tous les baccalauréats.
- Une note en prose sur la prolongation des études (`TRE-6800`) et les activités de recherche.
- Aucune exigence linguistique, aucune scolarité préparatoire, aucun stage.

## Scénario

1. Geneviève charge le M-GEX.
2. Elle constate que le menu « Spécialisation » est vide.
3. Elle place les six cours obligatoires et quatre cours de la Règle 1.

## Résultats attendus

- Le menu « Spécialisation » ne contient aucune option; le panneau n'affiche que la section « Activités communes ».
- Le bilan atteint `45 cr. / 45 cr.` avec les six obligatoires et 12 crédits de la Règle 1, selon leurs crédits réels.
- L'absence de scolarité préparatoire signifie qu'aucune case à cocher n'apparaît : la vérification doit fonctionner sans elle (US-38).
- Une grille de 45 crédits tient en cinq ou six sessions : les onze colonnes par défaut sont largement suffisantes.

## Repères pour le test e2e

- `#cheminement-select option` est vide.
- `#scolarite-completee` n'existe pas.
- La ligne `Total :` cite 45 crédits exigés.

## Variantes et cas limites

- Une admission à l'été (`E26`) est possible pour ce programme seulement : la séquence de colonnes commence alors par `E`, puis `A`, `H`, `E`, `A`… (US-08).
- Les activités de recherche (`GEX-6811` à `GEX-6814`) portent des crédits élevés et n'ont pas d'horaire publié : elles occupent une case sans plage horaire, comme les cours à l'étranger (US-03).
- **Écart connu** : la note en prose du programme n'est affichée nulle part, comme pour les profils négociés (US-80).
- Le champ `cycle` n'est lu par aucun module : rien ne distingue visuellement un programme de maîtrise d'un baccalauréat dans l'interface.
