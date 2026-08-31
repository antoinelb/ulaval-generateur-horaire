# Programmes, règles et couverture

Un programme (`B-GEX-A26.json`) structure ses exigences en trois étages : les **cours obligatoires** (`mandatory`), les **règles** à choix (`rules`), et les blocs optionnels — **concentrations** et **profils** — qui portent chacun leurs propres obligatoires et règles.

## Les règles

Une règle vient du texte « Règle N – \<contrainte\> parmi : » de la page du répertoire :

```json
{
  "title": "Règle 1",
  "constraint": {"type": "credits", "min": 12, "max": 12},
  "courses": ["GGL-2601", "GLG-1000", "…"]
}
```

- `constraint` étiquette l'unité comptée : `course` (« Un cours parmi » = min 1, max 1) ou `credits` ; elle est absente quand la page ne nomme aucun nombre — la règle est alors montrée, jamais comptée.
- `courses` est une liste de sigles, une **référence** (« tous les cours de la Règle 1 du cheminement X »), ou un **mot-clé** : `"any"` (tout cours du cycle satisfait) ou `"negotiated"` (« convenus avec la direction » — pas de liste fixe).
- Ce que la grammaire ne comprend pas reste en `raw`, montré tel quel.
- `credits_in_addition: true` : les crédits de la règle (les stages des bacs de génie) sont *en sus* du total du programme.
- La règle « Stages » des bacs de génie liste **en tête** le stage exigé pour diplômer, les optionnels après.
  Sa contrainte compte les stages, mais le premier est exigé à lui seul : la règle reste incomplète tant qu'il n'est pas de la sélection, même si un stage optionnel la remplit numériquement.

## L'exigence linguistique

Le cours d'anglais (ou de français) exigé pour diplômer, dont un score de test de classement dispense, vit dans le champ dédié `language_requirement` — branche francophone et, quand la page la donne, non francophone.

## Le rapport de couverture

`verify_organigramme` avec un `program` produit `coverage` : la sélection satisfait-elle les exigences ?

```json
{
  "mandatory": [{"scope": "program", "satisfied": ["GEX-1000"], "missing": ["GCI-1001"]}],
  "rules": [
    {
      "scope": "concentration",
      "title": "Règle 1",
      "status": "incomplete",
      "counted": ["GGL-2601"],
      "missing": {"credits": 9},
      "candidates": ["GLG-1000", "…"]
    },
    {
      "scope": "concentration",
      "title": "Règle 2",
      "status": "incomplete",
      "counted": [],
      "elsewhere": ["GGL-2601"],
      "missing": {"credits": 3},
      "candidates": ["GLG-2000", "…"]
    }
  ],
  "language_requirement": {"status": "reported"}
}
```

- `scope` : d'où vient l'exigence — `program`, `concentration` ou `profile` (la concentration et le profil choisis seulement).
- `status` : `satisfied`, `incomplete` (avec `missing`, le manque en cours ou en crédits, et `candidates`, la liste moins la sélection), `reported` — la règle n'a pas pu être comptée (pas de contrainte, mot-clé négocié, texte brut) et son `raw` est montré —, `over_max` (la sélection dépasse le maximum de la règle : une violation, montrée en rouge sur cette règle seule, `counted` gardant tous les codes pour que l'interface écrive « 15/12 cr »), ou `uncounted` (les données de la règle empêchent tout comptage ; `defect` dit laquelle — `missing_course` ou `broken_reference`). Aucun de ces deux derniers n'interrompt le rapport : les autres règles restent comptées (ADR `2026-08-depassement-de-regle-en-statut-rouge`).
- `elsewhere` : les sigles que cette règle liste aussi, mais qu'une règle *précédente de la même portée* compte déjà — ici `GGL-2601`, réclamé par la Règle 1 — montrés pour que le cours ne semble pas oublié, mais absents de `counted` et de `candidates` : il ne compte jamais deux fois dans une même portée.
  Le champ est omis quand il est vide.
  Les portées restent indépendantes : le même cours peut être `counted` par une règle de la concentration et par une règle du profil à la fois.
- `language_requirement.status` : `satisfied` si un cours d'une branche est dans la sélection, sinon `reported` — jamais « manquant », car un score de test peut en dispenser et le module ne peut pas le voir.

Les candidats ne sont volontairement **pas** filtrés par la faisabilité hebdomadaire : la couverture est la couche comptable ; composer avec l'horaire est un choix d'interface.
