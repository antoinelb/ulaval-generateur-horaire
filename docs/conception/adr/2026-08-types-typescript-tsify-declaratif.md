# Types TypeScript réels par tsify en mode déclaratif

Date : 2026-08-09

## Contexte

Le `.d.ts` généré par wasm-pack typait les quatre exports `(input: any): any` : le consommateur JS n'avait ni autocomplétion ni vérification, et le contrat JSON ne vivait que dans des commentaires Rust.
La frontière sérialise délibérément avec `Serializer::json_compatible()` (cartes en objets nus) et ce chemin d'exécution ne devait pas changer.

## Décision

`tsify` 0.5 (le crate d'origine, à nouveau maintenu — `tsify-next` était le fork intérimaire, retourné en amont) en **mode déclaratif seulement** :

- `#[derive(Tsify)]` sans attribut ABI n'émet que la section TypeScript (`export interface …`) — zéro changement d'exécution, la frontière garde `JsValue → Result<JsValue, JsValue>` et `json_compatible()` octet pour octet.
- Les quatre exports sont typés par `unchecked_param_type` / `unchecked_return_type` (wasm-bindgen ≥ 0.2.100), avec un commentaire rustdoc **en français** — propagé en JSDoc par wasm-pack, c'est de la documentation consommateur.
- Dans `core`, les derives sont derrière une feature cargo `tsify` (dépendances `tsify` + `wasm-bindgen` optionnelles), activée par le crate wasm sous `cfg(target_arch = "wasm32")` seulement : `core` reste pur, `make test` ne compile jamais la feature (couverture inchangée), et le build Dioxus du crate `ui` ne la voit pas.
- Les types à serde manuel — `Time` (`"08:30"`), `CourseCycle` (`0 | 1 | 2`), `Cycle` (`1 | 2`), `Semester` (`"A26"`) — ne sont pas dérivés : un derive syntaxique déclarerait la forme Rust, pas la forme sur le fil ; ils sont des alias d'une `typescript_custom_section` de `boundary.rs`.
- Trois formes que le derive déclarerait mal sont aussi écrites à la main dans cette section : `Rule` (le `flatten` d'une union sortirait `interface Rule extends RuleCourses`, TypeScript invalide) et `CourseReport.valid` / `Alternative.valid` (`skip_serializing_if = "is_true"` : clé absente quand vraie, que le derive déclarerait requise).
- Surcharges `#[tsify(type = "…")]` là où le derive mentirait : les `BTreeMap` en `Record<…>` (le `Map` émis par défaut contredit `json_compatible`), les `Option` des snapshots en `| null` (le `| undefined` émis rejetterait les `null` explicites que le consommateur relit des données), `possible_semester_start` en lettres `("A" | "H" | "E")[]`, `seasons` en `Partial<Record<…>>`.

Vérification : le `.d.ts` généré passe `tsc --strict --skipLibCheck false` sans erreur.

## Alternatives rejetées

- **`into_wasm_abi` / `from_wasm_abi` (tsify ABI)** : remplacerait le sérialiseur de la frontière — il faudrait reproduire `json_compatible()` type par type — et le mécanisme est déprécié en amont.
- **Types miroirs dans le crate wasm** : ~35 types dupliqués qui dérivent en silence.
- **Une `typescript_custom_section` entièrement manuelle** : aucun lien de compilation avec les structs ; les quelques déclarations manuelles restantes sont l'exception justifiée, pas la règle.
- **`cfg(target_arch = "wasm32")` dans core au lieu d'une feature** : les sections TypeScript seraient injectées aussi dans le build Dioxus du crate `ui`.
