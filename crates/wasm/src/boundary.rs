use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::organigramme::{self, OrganigrammeInput};
use crate::schedule::{self, ScheduleInput};

// The whole JS surface, and the only code in the crate that is not plain
// Rust: four exports, each one conversion in and one out. Everything worth
// testing lives on the other side of these calls (ADR
// `2026-08-module-wasm-quatre-fonctions-js`). The `unchecked_*` attributes
// and the Tsify derives only decorate the generated `.d.ts`; the runtime
// path is untouched (ADR `2026-08-types-typescript-tsify-declaratif`).

// The hand-written serde of these core types escapes the Tsify derive, and
// three shapes the derive would mistype are declared by hand: `Rule`
// flattens a union (`extends` would be invalid TS) and the two `valid`
// keys are absent-means-true (same ADR).
#[wasm_bindgen(typescript_custom_section)]
const TS_ALIASES: &str = r#"
export type Time = string;
export type CourseCycle = 0 | 1 | 2;
export type Cycle = 1 | 2;
export type Semester = string;

export type Rule = {
    title: string;
    constraint?: Constraint;
    notes?: string[];
    credits_in_addition?: boolean;
} & RuleCourses;

export interface CourseReport {
    code: string;
    /** Absente quand vraie : la clé n'est écrite que lorsqu'elle vaut false. */
    valid?: boolean;
    selected: Section[];
    alternatives: Alternative[];
}

export interface Alternative {
    sections: Section[];
    /** Absente quand vraie : la clé n'est écrite que lorsqu'elle vaut false. */
    valid?: boolean;
}
"#;

/// Construit l'horaire hebdomadaire d'une session : les options que
/// l'étudiant a déjà choisies (`chosen`) sont épinglées, chaque autre cours
/// prend la première combinaison sans conflit.
/// Lève une chaîne décrivant l'erreur si une entrée est invalide.
#[wasm_bindgen(unchecked_return_type = "ScheduleReport")]
pub fn generate_schedule(
    #[wasm_bindgen(unchecked_param_type = "ScheduleInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, schedule::generate)
}

/// Vérifie l'horaire assemblé par l'étudiant : chaque cours demandé doit
/// porter son option choisie dans `chosen`, sinon la question est incomplète
/// et une erreur est levée — jamais un faux verdict.
#[wasm_bindgen(unchecked_return_type = "ScheduleReport")]
pub fn verify_schedule(
    #[wasm_bindgen(unchecked_param_type = "ScheduleInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, schedule::verify)
}

/// Construit l'organigramme : chaque cours du programme (et les cours à
/// option retenus) est placé sur l'horizon de sessions, `pinned` fixant ce
/// que l'étudiant a déjà arrêté.
/// Lève une chaîne décrivant l'erreur si une entrée est invalide.
#[wasm_bindgen(unchecked_return_type = "OrganigrammeReport")]
pub fn generate_organigramme(
    #[wasm_bindgen(unchecked_param_type = "OrganigrammeInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<OrganigrammeInput, _>(input, organigramme::generate)
}

/// Vérifie le cheminement assemblé par l'étudiant : le placement est prouvé
/// avec tous les cours épinglés, puis les règles du programme sont comptées
/// (`coverage`).
/// Un cours laissé sans session est une question incomplète — erreur, jamais
/// un faux verdict.
#[wasm_bindgen(unchecked_return_type = "OrganigrammeReport")]
pub fn verify_organigramme(
    #[wasm_bindgen(unchecked_param_type = "OrganigrammeInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<OrganigrammeInput, _>(input, organigramme::verify)
}

fn run<I, O>(
    input: JsValue,
    solve: fn(&I) -> Result<O, String>,
) -> Result<JsValue, JsValue>
where
    I: serde::de::DeserializeOwned,
    O: Serialize,
{
    let input: I = serde_wasm_bindgen::from_value(input)?;
    let output = solve(&input).map_err(|e| JsValue::from_str(&e))?;
    // json_compatible: a map serializes to a plain object, not a JS `Map` —
    // what a caller reaching for `solution.placement["GEX-1000"]` expects
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(output.serialize(&serializer)?)
}
