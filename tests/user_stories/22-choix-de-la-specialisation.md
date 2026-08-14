# US-22 — Choisir la spécialisation

**Persona** : Bruno, au B-GEX, qui veut voir ce qu'ajoute le profil international.
**Intention** : comparer les règles avec et sans spécialisation.

## Préconditions

- Programme « B-GEX » A26 : `concentrations` vide, `profiles` contient « Profil international ».

## Scénario

1. Bruno ouvre le menu « Spécialisation ».
2. Il choisit « Profil international ».
3. Il parcourt le panneau de droite.

## Résultats attendus

- Le menu est peuplé depuis les concentrations **et** les profils du fichier de programme, dans cet ordre.
- Le panneau affiche d'abord la section « Activités communes », puis la section de la spécialisation choisie, chacune sous un en-tête repliable.
- Les cours obligatoires d'une spécialisation apparaissent sous une règle « Cours obligatoires » portant le nom de la spécialisation.
- Une spécialisation déclarant `credits_required` affiche ses crédits dans son en-tête de bilan; sans ce champ, l'en-tête n'affiche que le nom.

## Repères pour le test e2e

- `#cheminement-select option` reflète les spécialisations du millésime courant.
- `.rule-section-header` existe pour chaque section; un clic bascule `.collapsed` et masque le `.rule-section-collapsible` suivant.
- `.rule-header` a la forme `<titre> (<spécialisation>) (<n> crédits)`.

## Variantes et cas limites

- Le HTML statique contient trois options par défaut (« Cheminement sans concentration », « Robotique », « Génie du bâtiment durable ») : elles doivent être remplacées au premier chargement, jamais visibles pour un programme qui ne les a pas.
- Un programme sans spécialisation laisse le menu vide et n'affiche que « Activités communes ».
- Changer de spécialisation ne vide pas la grille (US-09).
