# US-70 — B-GIN, concentration « Approche généraliste »

**Persona** : Anne-Sophie, au baccalauréat en génie industriel, qui ne veut pas se spécialiser.
**Intention** : combler ses 15 crédits à option sans contrainte de thème.

## Préconditions

- Programme « B-GIN », session d'admission « A26 ».
- C'est la **première** concentration de la liste : elle est sélectionnée d'office au chargement.

## Ce que la concentration ajoute

- Aucun cours obligatoire.
- Règle 1 : 15 crédits parmi 13 cours (`GIF-1003`, `GIN-4021`, `GMC-2007`, `GMC-4100`, `GMC-4200`…).
- `credits_required` vaut 15.
- Une note en prose : « L'étudiant admis au profil entrepreneurial choisit les activités qui y sont prévues. »

## Scénario

1. Anne-Sophie charge le B-GIN.
2. Elle place cinq cours de la Règle 1.
3. Elle lit le bilan et cherche la note.

## Résultats attendus

- L'en-tête de section affiche `Approche généraliste : 15 cr. / 15 cr.` une fois les cinq cours placés.
- Une seule carte de règle apparaît sous la section.

## Repères pour le test e2e

- `#cheminement-select` a `Approche généraliste` en première position et cinq concentrations avant les deux profils.
- La carte de la Règle 1 contient 13 `.course-line`.

## Variantes et cas limites

- **Écart connu** : le champ `notes` de la concentration n'est lu par aucun module du frontend. La note qui renvoie au profil entrepreneurial n'apparaît nulle part, alors que l'invariant du projet est de ne jamais rien perdre en silence.
- Le B-GIN n'a pas de « Cheminement sans concentration » nommé comme tel : « Approche généraliste » en tient lieu.
- Cette concentration est le seul cas du B-GIN à n'avoir qu'une règle : les quatre autres en ont trois ou quatre.
