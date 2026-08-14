# US-33 — Rechercher un cours dans le panneau

**Persona** : Sophie, qui cherche un cours dont elle ne se rappelle que le titre.
**Intention** : filtrer la longue liste des activités pédagogiques.

## Préconditions

- Un programme chargé avec plusieurs règles.

## Scénario

1. Sophie tape « hydra » dans le champ de recherche.
2. Elle efface, puis tape « GCI ».
3. Elle efface complètement.

## Résultats attendus

- Le filtre est insensible à la casse et porte sur le sigle **et** sur le titre du cours.
- Les lignes de cours qui ne correspondent pas sont masquées.
- Une carte de règle dont plus aucun cours n'est visible est masquée à son tour.
- Un champ vide réaffiche tout.
- Le champ est vidé à chaque reconstruction du panneau (changement de programme, de millésime ou de spécialisation).

## Repères pour le test e2e

- `#cours-search` est un `input[type=search]`.
- Après saisie, compter les `.course-line` dont `style.display` n'est pas `none`.
- Une `.rule-card` sans ligne visible a `style.display: none`.

## Variantes et cas limites

- Une recherche sans résultat masque toutes les cartes : le panneau apparaît vide, sans message explicatif.
- Les en-têtes de section restent visibles même quand toutes leurs cartes sont masquées.
- La recherche ne porte que sur les cours du programme affiché, pas sur le catalogue complet; un cours hors programme n'est atteignable que par un fichier de cheminement ou par Capsule.
