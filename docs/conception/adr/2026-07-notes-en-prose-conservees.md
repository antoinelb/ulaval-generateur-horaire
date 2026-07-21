# Notes en prose conservées sur les règles et les blocs

Date : 2026-07-20

## Contexte

Deux sortes de prose n'avaient aucune place dans le modèle.

**Attachée à une règle qui a déjà des cours.**
`RuleCourses::List` ne porte pas de `raw`, donc les `p.fe-bloc-regle--ligne` d'une règle énumérant des cartes étaient jetés : les étiquettes de sous-groupes thématiques (« Programmation », « Langue et communication », « Biologie »…) et les notes (« Pour la personne francophone, la réussite du cours ANL-2020 … est requise pour diplômer »).

**Attachée à un bloc.**
`div.fe-bloc-section--paragraphe` porte, sur quatre des six pages gelées, une **exigence de diplômation** qui n'apparaît dans aucune règle :

> l'étudiant doit réussir le stage de formation pratique GMC-2580 pour obtenir son diplôme

Idem GCI-2580 (civil), GEX-1580 (eaux), GIN-3580 (industriel).
Ces cours disparaissaient entièrement du snapshot.

`docs/conception/initial/CONCEPTION.md` § « Règles de programme » prévoyait déjà un champ `notes` : « les prescriptions en prose (stages, exigences d'anglais de la règle 4) sont conservées telles quelles dans `notes` et affichées, pas interprétées ».
Les fixtures écrites à la main l'avaient perdu.

## Décision

1. `notes: Vec<String>` sur `Rule`, `Program`, `Concentration` et `Profile`, avec `#[serde(default, skip_serializing_if = "Vec::is_empty")]` : aucune clé parasite là où il n'y a pas de prose.
2. Ce qui y va :
   - règle énumérant des cours → **toutes** ses lignes de prose ;
   - règle en prose → les lignes **après** la première (la première est `raw`) ;
   - bloc → ses `div.fe-bloc-section--paragraphe`, plus les lignes d'un accordéon « Cours obligatoires ».
3. Rien n'y est interprété. Une note est du texte à afficher ; le solveur ne la lit pas.
4. Le modèle n'a **pas** de sous-groupes : les cours de tous les `ul.fe--liste-cours` d'une règle sont aplatis dans l'ordre du document, et les étiquettes deviennent des notes dans ce même ordre. Une interface peut donc les réafficher, mais aucune logique n'en dépend.

## Alternatives rejetées

- **`notes` sur `Rule` seulement** : ne couvre pas les paragraphes de bloc, donc perd les stages obligatoires — le cas le plus coûteux des deux.
- **Tout jeter et documenter que la prose est hors périmètre** : garde le modèle minimal mais fait disparaître GMC-2580, un cours exigé pour diplômer, en contradiction directe avec l'invariant « ne jamais perdre une entrée non reconnue » de `docs/project_plan.md`.
- **Modéliser les sous-groupes thématiques** (`subgroups`) : aucune règle métier n'en dépend — la contrainte porte sur la règle entière, jamais sur un sous-groupe — et cela ajouterait un niveau à `Rule` pour un usage purement décoratif.
