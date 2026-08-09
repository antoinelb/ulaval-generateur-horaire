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
    }
  ],
  "language_requirement": {"status": "reported"}
}
```

- `scope` : d'où vient l'exigence — `program`, `concentration` ou `profile` (la concentration et le profil choisis seulement).
- `status` : `satisfied`, `incomplete` (avec `missing`, le manque en cours ou en crédits, et `candidates`, la liste moins la sélection), ou `reported` — la règle n'a pas pu être comptée (pas de contrainte, mot-clé négocié, texte brut) et son `raw` est montré.
- `language_requirement.status` : `satisfied` si un cours d'une branche est dans la sélection, sinon `reported` — jamais « manquant », car un score de test peut en dispenser et le module ne peut pas le voir.

Les candidats ne sont volontairement **pas** filtrés par la faisabilité hebdomadaire : la couverture est la couche comptable ; composer avec l'horaire est un choix d'interface.
