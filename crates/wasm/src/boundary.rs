use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::organigramme::{self, OrganigrammeInput};
use crate::schedule::{self, ScheduleInput};

// The whole JS surface, and the only code in the crate that is not plain
// Rust: four exports, each one conversion in and one out. Everything worth
// testing lives on the other side of these calls (ADR
// `2026-08-module-wasm-quatre-fonctions-js`).

#[wasm_bindgen]
pub fn generate_schedule(input: JsValue) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, schedule::generate)
}

#[wasm_bindgen]
pub fn verify_schedule(input: JsValue) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, schedule::verify)
}

#[wasm_bindgen]
pub fn generate_organigramme(input: JsValue) -> Result<JsValue, JsValue> {
    run::<OrganigrammeInput, _>(input, organigramme::generate)
}

#[wasm_bindgen]
pub fn verify_organigramme(input: JsValue) -> Result<JsValue, JsValue> {
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
