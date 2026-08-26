use std::cell::RefCell;
use std::collections::BTreeMap;

use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use ulaval_scheduler_core::{
    apply_prereq_overrides, borrow_seasons_from_equivalents, Course,
    PrereqOverride,
};

use crate::catalogue;
use crate::merge::merge_manual;
use crate::organigramme::{self, OrganigrammeInput};
use crate::protocol;
use crate::questions::{
    self, CoverageInput, HorizonInput, PrerequisitesInput,
};
use crate::schedule::{self, ScheduleInput};

// The whole browser surface, and the only code in the crate that is not
// plain Rust. Two consumers share it (ADR
// `2026-08-fusion-des-crates-wasm-et-ui-calculations`):
//
// - the plain-JavaScript interface calls the eight exports below, one
//   conversion in and one out (ADRs `2026-08-module-wasm-quatre-fonctions-js`,
//   `2026-08-surface-wasm-etendue-a-huit-fonctions`);
// - the Dioxus app's Web Worker calls `init_snapshot` once, then funnels
//   every request through `handle_message` (ADR
//   `2026-08-crate-ui-calculations-et-worker`).
//
// Everything worth testing lives on the other side of these calls. The
// `unchecked_*` attributes and the Tsify derives only decorate the generated
// `.d.ts`; the runtime path is untouched (ADR
// `2026-08-types-typescript-tsify-declaratif`).

// The catalogue, loaded at most once per worker. Either consumer may fill
// it; a caller that still passes `courses` per call keeps working, its list
// winning over this one (ADR `2026-08-snapshot-en-cache-dans-le-module-wasm`).
thread_local! {
    static SNAPSHOT: RefCell<Vec<Course>> = const { RefCell::new(Vec::new()) };
}

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

/// Charge le catalogue une fois pour toutes : les appels suivants peuvent
/// alors omettre `courses`, au lieu de réexpédier tout le répertoire à
/// chaque question. `snapshot_json` est le contenu de `cours.json`,
/// `manual_json` la liste des cours maintenus à la main, `overrides_json`
/// les préalables réécrits — ceux du millésime de l'étudiant et les siens
/// propres, déjà fusionnés par l'appelant.
/// Répond le nombre de cours retenus, les sigles manuels éclipsés par un
/// cours scrapé, et ce qu'une correction n'a pas pu faire — des collisions
/// et des refus à afficher, jamais à taire.
#[wasm_bindgen]
pub fn init_snapshot(
    snapshot_json: &str,
    manual_json: &str,
    overrides_json: &str,
) -> Result<String, JsValue> {
    let snapshot: Snapshot = serde_json::from_str(snapshot_json)
        .map_err(|e| JsValue::from_str(&format!("snapshot : {e}")))?;
    let manual: Vec<Course> = serde_json::from_str(manual_json)
        .map_err(|e| JsValue::from_str(&format!("manual courses : {e}")))?;
    let overrides: BTreeMap<String, PrereqOverride> =
        serde_json::from_str(overrides_json).map_err(|e| {
            JsValue::from_str(&format!("prerequisite overrides : {e}"))
        })?;
    let mut merged = merge_manual(snapshot.courses, manual);
    // the catalogue is whole here: a new course's invented calendar defers
    // to the equivalent the répertoire dates, before any student
    // correction (ADR `2026-08-saisons-empruntees-a-lequivalent`)
    borrow_seasons_from_equivalents(&mut merged.courses);
    let notes = apply_prereq_overrides(&mut merged.courses, &overrides);
    let summary = serde_json::json!({
        "course_count": merged.courses.len(),
        "collisions": merged.collisions,
        "override_notes": notes,
    });
    SNAPSHOT.with(|cell| *cell.borrow_mut() = merged.courses);
    // expect over `?`: serializing a number and strings provably cannot fail
    Ok(serde_json::to_string(&summary)
        .expect("Summary serialization always succeeds"))
}

#[derive(serde::Deserialize)]
struct Snapshot {
    courses: Vec<Course>,
}

/// Le protocole du worker de l'app Dioxus : une requête JSON en chaîne,
/// toujours une réponse — y compris quand la question est refusée.
#[wasm_bindgen]
pub fn handle_message(request: &str) -> String {
    SNAPSHOT.with(|cell| match catalogue::resolve(None, &cell.borrow()) {
        Ok(courses) => protocol::handle(request, courses),
        Err(message) => protocol::refusal(message),
    })
}

/// Construit l'horaire hebdomadaire d'une session : les options que
/// l'étudiant a déjà choisies (`chosen`) sont épinglées, chaque autre cours
/// prend la première combinaison sans conflit.
/// Lève une chaîne décrivant l'erreur si une entrée est invalide.
#[wasm_bindgen(unchecked_return_type = "ScheduleReport")]
pub fn generate_schedule(
    #[wasm_bindgen(unchecked_param_type = "ScheduleInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            schedule::generate(input, courses)
        })
    })
}

/// Vérifie l'horaire assemblé par l'étudiant : chaque cours demandé doit
/// porter son option choisie dans `chosen`, sinon la question est incomplète
/// et une erreur est levée — jamais un faux verdict.
#[wasm_bindgen(unchecked_return_type = "ScheduleReport")]
pub fn verify_schedule(
    #[wasm_bindgen(unchecked_param_type = "ScheduleInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<ScheduleInput, _>(input, |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            schedule::verify(input, courses)
        })
    })
}

/// Construit l'organigramme : chaque cours du programme (et les cours à
/// option retenus) est placé sur l'horizon de sessions, `pinned` fixant ce
/// que l'étudiant a déjà arrêté.
/// Lève une chaîne décrivant l'erreur si une entrée est invalide.
#[wasm_bindgen(unchecked_return_type = "OrganigrammeReport")]
pub fn generate_organigramme(
    #[wasm_bindgen(unchecked_param_type = "OrganigrammeInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<OrganigrammeInput, _>(input, |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            organigramme::generate(input, courses)
        })
    })
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
    run::<OrganigrammeInput, _>(input, |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            organigramme::verify(input, courses)
        })
    })
}

/// Les sessions qui pourraient accueillir `code` sur l'horizon décrit par
/// `input` : une sonde de placement par session, numéros 1-based — la forme
/// que `pinned` parle.
/// Lève une chaîne décrivant l'erreur si une entrée est invalide.
#[wasm_bindgen(unchecked_return_type = "number[]")]
pub fn admissible_sessions(
    #[wasm_bindgen(unchecked_param_type = "OrganigrammeInput")] input: JsValue,
    code: String,
) -> Result<JsValue, JsValue> {
    run::<OrganigrammeInput, _>(input, move |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            organigramme::admissible(input, courses, &code)
        })
    })
}

/// La question statique des préalables d'un cours, contre ce que l'étudiant
/// tient déjà (`satisfied`, `credits`) : `met`, plus les opérandes que le
/// verdict a dû présumer (texte brut, cours préuniversitaires) — remontés,
/// jamais imposés. `same_session` (optionnel) porte ce qui est posé dans la
/// session jugée : il ne satisfait qu'une feuille étoilée au répertoire.
#[wasm_bindgen(unchecked_return_type = "PrerequisitesReport")]
pub fn prerequisites_met(
    #[wasm_bindgen(unchecked_param_type = "PrerequisitesInput")]
    input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<PrerequisitesInput, _>(input, questions::prerequisites)
}

/// Le bilan de couverture des règles du programme sur la sélection donnée —
/// une grille partielle suffit, contrairement à `verify_organigramme` qui
/// exige un placement complet.
#[wasm_bindgen(unchecked_return_type = "CoverageReport")]
pub fn coverage_report(
    #[wasm_bindgen(unchecked_param_type = "CoverageInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<CoverageInput, _>(input, |input| {
        with_catalogue(input.courses.as_deref(), |courses| {
            questions::coverage(input, courses)
        })
    })
}

/// L'horizon des sessions en codes de millésime (« A26 », « H27 », « E27 »,
/// …) : `study_sessions` compte la seule alternance A/H, les étés s'insèrent
/// après chaque hiver, le dernier inclus.
#[wasm_bindgen(unchecked_return_type = "Semester[]")]
pub fn horizon_sessions(
    #[wasm_bindgen(unchecked_param_type = "HorizonInput")] input: JsValue,
) -> Result<JsValue, JsValue> {
    run::<HorizonInput, _>(input, questions::horizon)
}

fn run<I, O>(
    input: JsValue,
    // `impl FnOnce`, not a fn pointer: `admissible_sessions` closes over its
    // second JS argument
    solve: impl FnOnce(&I) -> Result<O, String>,
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

// The call's own `courses` if it carries one, the loaded snapshot otherwise
// — and a refusal when there is neither, since answering on an empty
// catalogue would be a verdict, not an answer.
fn with_catalogue<T>(
    inline: Option<&[Course]>,
    ask: impl FnOnce(&[Course]) -> Result<T, String>,
) -> Result<T, String> {
    SNAPSHOT.with(|cell| ask(catalogue::resolve(inline, &cell.borrow())?))
}
