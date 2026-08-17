# La frontière WASM

Le crate `wasm` est le crate de frontière : une seule orchestration au-dessus de `core`, exposée à **deux** consommateurs navigateur, plus les fonctions pures que l'app Dioxus appelle nativement (ADR `2026-08-fusion-des-crates-wasm-et-ui-calculations`).

Il reste délibérément mince : chaque export est une conversion en entrée et une en sortie, et tout ce qui vaut la peine d'être testé vit de l'autre côté de ces appels, en Rust natif.

```text
JS ──(objet)──▶ serde_wasm_bindgen::from_value ──▶ fonction pure de core
JS ◀─(objet)── Serializer::json_compatible()  ◀── rapport sérialisable

worker Dioxus ──(chaîne JSON)──▶ protocol::handle ──▶ les mêmes fonctions
worker Dioxus ◀─(chaîne JSON)── protocol::Response ◀──
```

## Les huit fonctions de la surface JavaScript

| Export | Cœur | Rôle |
|---|---|---|
| `generate_schedule` | `schedule::generate` | construit l'horaire hebdomadaire |
| `verify_schedule` | `schedule::verify` | juge la combinaison de l'étudiant |
| `generate_organigramme` | `organigramme::generate` | place le programme sur l'horizon |
| `verify_organigramme` | `organigramme::verify` | prouve le cheminement épinglé + compte les règles |
| `admissible_sessions` | `organigramme::admissible` | les sessions qui pourraient accueillir un sigle |
| `prerequisites_met` | `questions::prerequisites` | les préalables d'un cours contre ce qui est acquis |
| `coverage_report` | `questions::coverage` | le bilan des règles sur une grille même partielle |
| `horizon_sessions` | `questions::horizon` | l'horizon en codes de millésime |

Les deux `verify` partagent le même principe : vérifier avec un cours laissé sans choix est une **erreur**, jamais un faux verdict (ADR `2026-08-module-wasm-quatre-fonctions-js`, `2026-08-surface-wasm-etendue-a-huit-fonctions`).

## La surface du worker Dioxus

`init_snapshot` puis `handle_message` : des chaînes JSON dans les deux sens, une requête (`place`, `verify`, `admissible-sessions`) sous son `id`, **toujours** une réponse — une requête illisible, ou un worker interrogé avant d'avoir reçu son catalogue, répond sous l'id réservé 0 plutôt que de disparaître (ADR `2026-08-crate-ui-calculations-et-worker`).

`protocol.rs` n'est qu'un aiguillage : il appelle les mêmes `organigramme::generate/verify/admissible` que la surface JavaScript.

## Le catalogue, chargé une fois

`courses` est **optionnel** dans les entrées : un appelant qui a fait `init_snapshot` cesse de réexpédier le répertoire à chaque question.
Le transport coûtait ~66 ms par appel — la totalité du temps d'une fonction courte : `generate_schedule` passait de 66,1 ms à 0,26 ms (ADR `2026-08-snapshot-en-cache-dans-le-module-wasm`).

`catalogue::resolve` tranche, une fois pour toutes les fonctions : l'appel l'emporte sur le cache, une liste explicitement vide reste celle de l'appelant, et n'avoir ni l'un ni l'autre est une **erreur** — un catalogue vide répondrait « rien n'est plaçable » à toute question, un verdict déguisé en réponse.

C'est aussi pourquoi `courses` est un **paramètre** des fonctions pures et jamais lu depuis l'entrée : c'est ce qui permet aux deux surfaces de partager la même orchestration.

## Choix de sérialisation

- `Serializer::json_compatible()` en sortie : une carte devient un objet nu, pas un `Map` JavaScript — ce qu'attend un appelant qui écrit `solution.placement["GEX-1000"]`.
- `deny_unknown_fields` sur les entrées : une faute de frappe dans l'objet JS est refusée, pas lue comme une valeur par défaut.
- La glue `#[wasm_bindgen]` vit dans `boundary.rs`, compilé sous `cfg(all(target_arch = "wasm32", feature = "boundary"))` seulement : un `cargo test` natif ne compile ni la glue ni ses dépendances, et `make static` la linte sur la cible wasm.
  La feature est active par défaut — un drapeau oublié ne peut pas publier un paquet sans exports — et le crate `ui`, qui ne lie ce crate que pour ses modules purs, s'en désabonne (`default-features = false`) : porter une glue qu'il n'appelle jamais lui coûtait ~800 Ko de wasm.

## Les types TypeScript

Les déclarations du `.d.ts` sont **dérivées des structures Rust** par `tsify`, en mode déclaratif seulement (ADR `2026-08-types-typescript-tsify-declaratif`) :

- `#[derive(Tsify)]` sans attribut ABI n'émet que la section TypeScript — le chemin d'exécution (`json_compatible`) est intouché ;
- les exports sont typés par `unchecked_param_type` / `unchecked_return_type`, la signature Rust restant `JsValue → Result<JsValue, JsValue>` ;
- dans `core`, les derives sont derrière la feature `tsify`, activée par le build navigateur seul — `core` reste pur et sans dépendance wasm en natif ;
- les types à serde manuel (`Time`, `Semester`, les cycles) et les formes que le derive déclarerait mal (`Rule` et son union aplatie, les `valid` absentes-quand-vraies) sont déclarés à la main dans une `typescript_custom_section` de `boundary.rs`.

Les commentaires rustdoc des exports sont propagés par wasm-pack en JSDoc : la documentation vue par l'éditeur du consommateur est écrite dans le code Rust, une seule source de vérité.
