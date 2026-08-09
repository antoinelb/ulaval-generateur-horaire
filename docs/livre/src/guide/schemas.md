# Schémas d'entrée et de sortie

La référence exacte est le fichier [`ulaval_scheduler_wasm.d.ts`](https://antoinelb.github.io/ulaval-generateur-horaire/pkg/ulaval_scheduler_wasm.d.ts) du paquet : chaque interface y est dérivée de la structure Rust correspondante, et ce chapitre n'en reprend que les points qui surprennent.

## `null` explicite ou clé absente

Les snapshots écrivent des `null` explicites ; les rapports omettent certaines clés.
Le `.d.ts` distingue les deux :

- `last_offered: number | null`, `options: Section[][] | null`, `section: string | null`, `prerequisites?: Prerequisites | null` : la clé est là, sa valeur peut être `null`.
- `counted?: string[]`, `coverage?: CoverageReport`, `notes?: string[]` : la clé disparaît quand il n'y a rien à dire.
- Cas particulier : `CourseReport.valid` et `Alternative.valid` sont **absentes quand vraies** — seul `valid: false` est écrit.
  Lire `course.valid === false` pour détecter un conflit, jamais `!course.valid`… qui serait vrai aussi quand la clé absente signifie « valide ».

## Les formes à retenir

### `Credits` — nombre ou fourchette

```ts
type Credits = number | { min: number; max: number };
```

Presque tous les cours portent un nombre.
Une fourchette est un stage dont l'étudiant choisit lui-même la pondération (MED-1911 vaut « 6 à 12 » crédits).

### `PrereqTree` — l'arbre ET/OU des préalables

```ts
type PrereqTree =
  | string                                  // un code de cours
  | { raw: string }                         // un opérande hors grammaire, verbatim
  | { all: PrereqTree[] }                   // ET
  | { any: PrereqTree[] }                   // OU
  | { program_credits: ProgramCredits };    // seuil de crédits accumulés
```

### `Rule` — une règle de programme

```ts
type Rule = {
  title: string;
  constraint?: Constraint;      // absente quand la page ne nomme aucun nombre
  notes?: string[];
  credits_in_addition?: boolean; // crédits « en sus » du total du programme
} & RuleCourses;
```

`RuleCourses` est une union aplatie dans l'objet : une liste (`courses: string[]`), une référence à une autre règle, un mot-clé (`"any"`, `"negotiated"`) ou du texte brut seul.
`Constraint` étiquette l'unité comptée — `{type: "course"}` ou `{type: "credits"}` — parce qu'un `{min, max}` nu serait ambigu.

### Types à serde manuel

Quatre types core sérialisent une forme différente de leur structure Rust ; le `.d.ts` les déclare en alias :

```ts
type Time = string;             // "08:30"
type CourseCycle = 0 | 1 | 2;   // 0 = préuniversitaire
type Cycle = 1 | 2;             // un programme n'est jamais préuniversitaire
type Semester = string;         // "A26" = automne 2026
```

### Cartes = objets nus

Toutes les cartes traversent la frontière en objets JavaScript nus, jamais en `Map` :

```ts
chosen?: Record<string, string[]>;     // entrée
pinned?: Record<string, number>;       // entrée
placement: Record<string, number>;     // sortie
seasons: Partial<Record<Season, SeasonOffering>>;  // snapshot
```

`Partial` sur `seasons` : un cours ne liste que les saisons où il est offert.
