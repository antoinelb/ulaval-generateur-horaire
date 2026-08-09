# La frontière WASM

Le crate `wasm` est délibérément mince : quatre exports, chacun une conversion en entrée et une en sortie, et tout ce qui vaut la peine d'être testé vit de l'autre côté de ces appels, en Rust natif.

```text
JS ──(objet)──▶ serde_wasm_bindgen::from_value ──▶ fonction pure de core
JS ◀─(objet)── Serializer::json_compatible()  ◀── rapport sérialisable
```

## Les quatre fonctions

| Export | Cœur | Rôle |
|---|---|---|
| `generate_schedule` | `schedule::generate` | construit l'horaire hebdomadaire |
| `verify_schedule` | `schedule::verify` | juge la combinaison de l'étudiant |
| `generate_organigramme` | `organigramme::generate` | place le programme sur l'horizon |
| `verify_organigramme` | `organigramme::verify` | prouve le cheminement épinglé + compte les règles |

Les deux `verify` partagent le même principe : vérifier avec un cours laissé sans choix est une **erreur**, jamais un faux verdict (ADR `2026-08-module-wasm-quatre-fonctions-js`).

## Choix de sérialisation

- `Serializer::json_compatible()` en sortie : une carte devient un objet nu, pas un `Map` JavaScript — ce qu'attend un appelant qui écrit `solution.placement["GEX-1000"]`.
- `deny_unknown_fields` sur les entrées : une faute de frappe dans l'objet JS est refusée, pas lue comme une valeur par défaut.
- La glue `#[wasm_bindgen]` vit dans `boundary.rs`, compilé sous `cfg(target_arch = "wasm32")` seulement : un `cargo test` natif ne compile ni la glue ni ses dépendances, et `make static` la linte sur la cible wasm.

## Les types TypeScript

Les déclarations du `.d.ts` sont **dérivées des structures Rust** par `tsify`, en mode déclaratif seulement (ADR `2026-08-types-typescript-tsify-declaratif`) :

- `#[derive(Tsify)]` sans attribut ABI n'émet que la section TypeScript — le chemin d'exécution (`json_compatible`) est intouché ;
- les quatre exports sont typés par `unchecked_param_type` / `unchecked_return_type`, la signature Rust restant `JsValue → Result<JsValue, JsValue>` ;
- dans `core`, les derives sont derrière la feature `tsify`, activée par le build navigateur seul — `core` reste pur et sans dépendance wasm en natif ;
- les types à serde manuel (`Time`, `Semester`, les cycles) et les formes que le derive déclarerait mal (`Rule` et son union aplatie, les `valid` absentes-quand-vraies) sont déclarés à la main dans une `typescript_custom_section` de `boundary.rs`.

Les commentaires rustdoc des quatre exports sont propagés par wasm-pack en JSDoc : la documentation vue par l'éditeur du consommateur est écrite dans le code Rust, une seule source de vérité.
