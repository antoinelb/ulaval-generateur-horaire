use std::collections::BTreeMap;

use ulaval_scheduler_core::Course;

use crate::import::LocalProgram;
use crate::state::{Plan, View};

pub const PLAN_KEY: &str = "gh.v1.plan";
pub const VIEW_KEY: &str = "gh.v1.view";
pub const LOG_KEY: &str = "gh.v1.log";
pub const MANUAL_KEY: &str = "gh.v1.cours-manuels";
pub const LOCAL_PROGRAMS_KEY: &str = "gh.v1.programmes-locaux";
pub const LOG_CAP: usize = 200;
const VERSION: u32 = 1;

// One stored value: a versioned envelope around the state. Restoring is
// best-effort and *loud*: anything tolerated becomes a note for the status
// region, anything at risk of loss comes back as `backup` for the caller
// to stash under a fresh key before the next save overwrites it — the
// opposite discipline of the solver inputs (`deny_unknown_fields`).
#[derive(serde::Serialize, serde::Deserialize)]
struct Envelope<T> {
    version: u32,
    state: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored<T> {
    pub state: T,
    pub notes: Vec<String>,
    pub backup: Option<String>,
}

pub fn encode_plan(plan: &Plan) -> String {
    encode(plan)
}

pub fn restore_plan(stored: Option<&str>) -> Restored<Plan> {
    let mut restored: Restored<Plan> = restore(stored, "du plan");
    // heal saves from before the horizon rule: a stray seat beyond the
    // horizon made verify refuse the whole plan (« GEX-2001 is pinned to
    // session 11, outside 1..=9 », 2026-08-26) — re-asserting the saved
    // horizon evicts it and re-floors under the pins
    let saved_horizon = restored.state.study_sessions;
    crate::state::set_horizon(&mut restored.state, saved_horizon);
    restored
}

pub fn encode_view(view: &View) -> String {
    encode(view)
}

pub fn restore_view(stored: Option<&str>) -> Restored<View> {
    restore(stored, "de l'affichage")
}

// the student's hand-entered Courses (ADR
// `2026-07-contribution-de-cours-manuels`) — same envelope, same loud
// tolerance: a damaged list restarts empty with a kept copy
pub fn encode_manual(manual: &Vec<Course>) -> String {
    encode(manual)
}

pub fn restore_manual(stored: Option<&str>) -> Restored<Vec<Course>> {
    restore(stored, "des cours manuels")
}

// the student's programs imported by URL (plan item 5) — same envelope,
// same loud tolerance. The stored value is a JSON array, not an object, so
// `unknown_keys` never finds anything to name here — the tolerance plays out
// at the whole-value level instead, exactly as for the manual course list.
pub fn encode_local_programs(programs: &Vec<LocalProgram>) -> String {
    encode(programs)
}

pub fn restore_local_programs(
    stored: Option<&str>,
) -> Restored<Vec<LocalProgram>> {
    restore(stored, "des programmes locaux")
}

fn encode<T: serde::Serialize>(state: &T) -> String {
    // expect over `?`: serializing maps, sets and strings provably
    // cannot fail
    serde_json::to_string(&Envelope {
        version: VERSION,
        state,
    })
    .expect("State serialization always succeeds")
}

fn restore<T>(stored: Option<&str>, what: &str) -> Restored<T>
where
    T: serde::de::DeserializeOwned + serde::Serialize + Default,
{
    let Some(raw) = stored else {
        // a first visit: nothing to restore, nothing to say
        return Restored {
            state: T::default(),
            notes: Vec::new(),
            backup: None,
        };
    };
    let keep = |note: String| Restored {
        state: T::default(),
        notes: vec![note],
        backup: Some(raw.to_string()),
    };
    let value: serde_json::Value = match serde_json::from_str(raw) {
        Ok(value) => value,
        Err(error) => {
            return keep(format!(
                "Sauvegarde {what} illisible ({error}) — reprise à neuf, \
                 copie conservée."
            ))
        }
    };
    let Some(version) = value.get("version").and_then(|v| v.as_u64()) else {
        return keep(format!(
            "Sauvegarde {what} sans version — reprise à neuf, copie \
             conservée."
        ));
    };
    if version > u64::from(VERSION) {
        return keep(format!(
            "Sauvegarde {what} écrite par une version plus récente \
             (v{version}) — reprise à neuf, copie conservée."
        ));
    }
    let Some(state_value) = value.get("state") else {
        return keep(format!(
            "Sauvegarde {what} sans contenu — reprise à neuf, copie \
             conservée."
        ));
    };
    let state: T = match serde_json::from_value(state_value.clone()) {
        Ok(state) => state,
        Err(error) => {
            return keep(format!(
                "Sauvegarde {what} incompatible ({error}) — reprise à \
                 neuf, copie conservée."
            ))
        }
    };
    // fields the current version does not know: named and backed up, so an
    // accidental downgrade loses nothing silently
    let unknown = unknown_keys(state_value, &state);
    if unknown.is_empty() {
        Restored {
            state,
            notes: Vec::new(),
            backup: None,
        }
    } else {
        Restored {
            notes: vec![format!(
                "Sauvegarde {what} : champs inconnus ignorés ({}) — copie \
                 conservée.",
                unknown.join(", ")
            )],
            backup: Some(raw.to_string()),
            state,
        }
    }
}

fn unknown_keys<T: serde::Serialize>(
    stored: &serde_json::Value,
    parsed: &T,
) -> Vec<String> {
    // expect over `?`: the value was just deserialized from JSON
    let known = serde_json::to_value(parsed)
        .expect("State serialization always succeeds");
    match (stored.as_object(), known.as_object()) {
        (Some(stored), Some(known)) => stored
            .keys()
            .filter(|key| !known.contains_key(*key))
            .cloned()
            .collect(),
        _ => Vec::new(),
    }
}

// --- the per-(program, vintage) shelf ---------------------------------------
// `gh.v1.plan` stays the living document, picker state included — it *is*
// the pointer, no « dernier » key needed. Leaving a program shelves the
// document whole under its own key, synchronously (never behind the save
// debounce); entering one restores that snapshot exactly, or starts fresh.
// No migration: nothing is deployed (ADR
// `2026-08-instantane-de-plan-par-programme-et-millesime`).

// « gh.v1.plan/B-GEX-A26 » — the same naming as `data/programmes/`
pub fn snapshot_key(choice: &crate::state::ProgramChoice) -> String {
    format!("{PLAN_KEY}/{}-{}", choice.code, choice.semester)
}

// Everything one document swap decides, computed pure — the component
// only writes localStorage and the signals around it.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentSwap {
    // (shelf key, encoded envelope) of the document being left
    pub stash: Option<(String, String)>,
    // the living document afterwards
    pub next: Plan,
    // the loud tolerance of a damaged shelf snapshot, as at boot
    pub notes: Vec<String>,
    pub backup: Option<String>,
}

// « changer » : shelve the current document, hand back the picker — the
// placements leave with it, so nothing is counted under no program (US-10 :
// « la grille est vidée au passage »)
pub fn leave_document(current: &Plan) -> DocumentSwap {
    DocumentSwap {
        stash: stash_of(current),
        next: Plan {
            start: current.start,
            ..Plan::default()
        },
        notes: Vec::new(),
        backup: None,
    }
}

// « Choisir » : shelve whatever is being left (normally the picker —
// nothing), then restore the shelf snapshot exactly, or start fresh. The
// key is the identity, so its code and semester are forced back onto the
// restored document; the snapshot's concentration and profile win over the
// click's defaults — they are the student's own last state.
pub fn enter_document(
    current: &Plan,
    choice: crate::state::ProgramChoice,
    stored: Option<&str>,
    study_sessions: usize,
) -> DocumentSwap {
    let stash = stash_of(current);
    let restored = restore_plan(stored);
    let mut next = restored.state;
    let mut notes = restored.notes;
    match next.program.take() {
        Some(held) => {
            next.program = Some(crate::state::ProgramChoice {
                code: choice.code,
                semester: choice.semester,
                concentration: held.concentration,
                profile: held.profile,
            });
        }
        // nothing on the shelf, or a damaged snapshot restarted fresh
        // (and kept): a new document, only the calendar identity and the
        // credit-derived horizon carried
        None => {
            // the caller resolves `choice.concentration` to this vintage's
            // default (`panel::default_concentration`) before ever calling
            // here — announced so a first-contact student knows the choice
            // was not her own (rapport étudiante-cegep 2026-08-27); a
            // restored document below says nothing, since nothing was
            // picked on its behalf
            if let Some(title) = choice.concentration.clone() {
                notes.push(format!(
                    "Concentration « {title} » sélectionnée par défaut — \
                     changez-la au besoin dans le panneau de gauche."
                ));
            }
            next = crate::state::fresh_plan(
                current.start,
                choice,
                study_sessions,
            );
        }
    }
    DocumentSwap {
        stash,
        next,
        notes,
        backup: restored.backup,
    }
}

// The fragment import stashes the current document only when the shared
// plan belongs to another (program, vintage): stashing the same key would
// let a later « changer » overwrite the shelf with the shared version.
pub fn import_stash(
    current: &Plan,
    shared: &Plan,
) -> Option<(String, String)> {
    let (key, encoded) = stash_of(current)?;
    if shared.program.as_ref().map(snapshot_key) == Some(key.clone()) {
        return None;
    }
    Some((key, encoded))
}

// a picker document (no program) has no shelf to go to
fn stash_of(plan: &Plan) -> Option<(String, String)> {
    plan.program
        .as_ref()
        .map(|choice| (snapshot_key(choice), encode_plan(plan)))
}

// --- the whole-organigramme share (fragment codec) --------------------------
// The link must carry *everything* — the recipient pastes it and sees the
// organigramme whole, no adjustment (note 9, 2026-08-13). Pipeline:
// state → postcard → raw deflate (kept only if smaller) → base64url, with
// one version|flag header byte in front (`docs/conception/shareable_link.md`).

// One frame, one version: nothing is deployed and no link has ever
// circulated, so nesting formats to stay backward-compatible protected
// links that do not exist (ADR
// `2026-08-trame-de-partage-unique-avant-deploiement`). The header byte
// stays: the day the app ships, the frame freezes and version 2 is how a
// future addition announces itself.
const SHARE_VERSION: u8 = 1;
const FLAG_DEFLATE: u8 = 0x80;
// a hostile link must not be a zip bomb against someone else's tab
const MAX_DECOMPRESSED: usize = 256 * 1024;

// Frozen at the first deployment, not before: postcard encodes
// positionally, so from that day on a field is never reordered, removed or
// inserted here — a later addition becomes version 2 with its migration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Share {
    program_code: Option<String>,
    program_semester: Option<String>,
    concentration: Option<String>,
    profile: Option<String>,
    // 0 = automne, 1 = hiver, 2 = été
    start_season: u8,
    start_year: u16,
    study_sessions: u32,
    summers_open: bool,
    credit_cap: u32,
    concomitant: bool,
    preparatory_done: bool,
    electives: Vec<String>,
    pinned_sessions: Vec<(String, u32)>,
    displayed_placement: Vec<(String, u32)>,
    chosen: Vec<(u32, SessionChosen)>,
    manual: Vec<(u32, Vec<String>)>,
    special: Vec<(u32, String)>,
    rule_grants: Vec<(String, String)>,
    credited: Vec<String>,
    // (code, the rewritten expression, the official text it replaced)
    prereq_overrides: Vec<(String, String, Option<String>)>,
    // each hand-entered Course as its own JSON — self-describing on
    // purpose, so core's Course may evolve without corrupting old links
    manual_courses: Vec<String>,
}

// one session's pinned sections: (code, sorted NRC set)
type SessionChosen = Vec<(String, Vec<String>)>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrganigrammeShareError {
    #[error("lien vide")]
    Empty,
    #[error("encodage base64 invalide")]
    Base64,
    #[error("décompression impossible — lien corrompu ou déraisonnable")]
    Inflate,
    #[error("version de lien inconnue ({0}) — l'application est trop vieille pour le lire")]
    UnknownVersion(u8),
    #[error("contenu du lien illisible")]
    Payload,
    #[error("saison de départ inconnue ({0})")]
    UnknownSeason(u8),
    #[error("cours manuel du lien illisible : {0}")]
    ManualCourse(String),
}

pub fn encode_organigramme(plan: &Plan, manual_courses: &[Course]) -> String {
    use base64::Engine;
    let state = share_from(plan, manual_courses);
    // expect over `?`: strings, vecs and integers provably cannot fail
    let raw = postcard::to_allocvec(&state)
        .expect("postcard serialization of plain data always succeeds");
    let (payload, flag) = pack(raw);
    let mut bytes = Vec::with_capacity(payload.len() + 1);
    bytes.push(SHARE_VERSION | flag);
    bytes.extend_from_slice(&payload);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// deflate has fixed overhead: it is kept only when it actually shrinks
fn pack(raw: Vec<u8>) -> (Vec<u8>, u8) {
    let deflated = miniz_oxide::deflate::compress_to_vec(&raw, 10);
    if deflated.len() < raw.len() {
        (deflated, FLAG_DEFLATE)
    } else {
        (raw, 0)
    }
}

pub fn decode_organigramme(
    fragment: &str,
) -> Result<(Plan, Vec<Course>), OrganigrammeShareError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(fragment)
        .map_err(|_| OrganigrammeShareError::Base64)?;
    let (header, body) =
        bytes.split_first().ok_or(OrganigrammeShareError::Empty)?;
    let raw = if header & FLAG_DEFLATE != 0 {
        miniz_oxide::inflate::decompress_to_vec_with_limit(
            body,
            MAX_DECOMPRESSED,
        )
        .map_err(|_| OrganigrammeShareError::Inflate)?
    } else {
        body.to_vec()
    };
    match header & !FLAG_DEFLATE {
        SHARE_VERSION => {
            let state: Share = postcard::from_bytes(&raw)
                .map_err(|_| OrganigrammeShareError::Payload)?;
            share_into(state)
        }
        version => Err(OrganigrammeShareError::UnknownVersion(version)),
    }
}

fn share_from(plan: &Plan, manual_courses: &[Course]) -> Share {
    let session_map = |map: &BTreeMap<String, usize>| {
        map.iter()
            .map(|(code, &session)| (code.clone(), session as u32))
            .collect()
    };
    Share {
        program_code: plan.program.as_ref().map(|choice| choice.code.clone()),
        program_semester: plan
            .program
            .as_ref()
            .map(|choice| choice.semester.clone()),
        concentration: plan
            .program
            .as_ref()
            .and_then(|choice| choice.concentration.clone()),
        profile: plan
            .program
            .as_ref()
            .and_then(|choice| choice.profile.clone()),
        start_season: match plan.start.season {
            ulaval_scheduler_core::Season::Fall => 0,
            ulaval_scheduler_core::Season::Winter => 1,
            ulaval_scheduler_core::Season::Summer => 2,
        },
        start_year: plan.start.year,
        study_sessions: plan.study_sessions as u32,
        summers_open: plan.summers_open,
        credit_cap: plan.credit_cap,
        concomitant: plan.concomitant,
        preparatory_done: plan.preparatory_done,
        electives: plan.electives.clone(),
        pinned_sessions: session_map(&plan.pinned_sessions),
        displayed_placement: session_map(&plan.displayed_placement),
        chosen: plan
            .chosen
            .iter()
            .map(|(&session, courses)| {
                (
                    session as u32,
                    courses
                        .iter()
                        .map(|(code, nrcs)| {
                            (code.clone(), nrcs.iter().cloned().collect())
                        })
                        .collect(),
                )
            })
            .collect(),
        manual: plan
            .manual
            .iter()
            .map(|(&session, codes)| (session as u32, codes.clone()))
            .collect(),
        special: plan
            .special
            .iter()
            .map(|(&session, label)| (session as u32, label.clone()))
            .collect(),
        rule_grants: plan
            .rule_grants
            .iter()
            .map(|(code, key)| (code.clone(), key.clone()))
            .collect(),
        credited: plan.credited.iter().cloned().collect(),
        prereq_overrides: plan
            .prereq_overrides
            .iter()
            .map(|(code, value)| {
                (code.clone(), value.text.clone(), value.official.clone())
            })
            .collect(),
        manual_courses: manual_courses
            .iter()
            .map(|course| {
                // expect over `?`: serializing a Course provably cannot fail
                serde_json::to_string(course)
                    .expect("Course serialization always succeeds")
            })
            .collect(),
    }
}

fn share_into(
    state: Share,
) -> Result<(Plan, Vec<Course>), OrganigrammeShareError> {
    let season = match state.start_season {
        0 => ulaval_scheduler_core::Season::Fall,
        1 => ulaval_scheduler_core::Season::Winter,
        2 => ulaval_scheduler_core::Season::Summer,
        other => {
            return Err(OrganigrammeShareError::UnknownSeason(other));
        }
    };
    let courses = state
        .manual_courses
        .iter()
        .map(|json| {
            serde_json::from_str::<Course>(json).map_err(|error| {
                OrganigrammeShareError::ManualCourse(error.to_string())
            })
        })
        .collect::<Result<Vec<Course>, _>>()?;
    let plan = Plan {
        prereq_overrides: state
            .prereq_overrides
            .into_iter()
            .map(|(code, text, official)| {
                (
                    code,
                    ulaval_scheduler_core::PrereqOverride { text, official },
                )
            })
            .collect(),
        program: state.program_code.map(|code| crate::state::ProgramChoice {
            code,
            semester: state.program_semester.unwrap_or_default(),
            concentration: state.concentration,
            profile: state.profile,
        }),
        start: ulaval_scheduler_core::Semester {
            season,
            year: state.start_year,
        },
        study_sessions: state.study_sessions as usize,
        // ponytail: the share URL does not carry the relevé's completed
        // sessions — a shared organigramme reopens fully editable; add the
        // field to the share state if that ever misleads
        completed_sessions: 0,
        summers_open: state.summers_open,
        credit_cap: state.credit_cap,
        concomitant: state.concomitant,
        preparatory_done: state.preparatory_done,
        electives: state.electives,
        pinned_sessions: state
            .pinned_sessions
            .into_iter()
            .map(|(code, session)| (code, session as usize))
            .collect(),
        displayed_placement: state
            .displayed_placement
            .into_iter()
            .map(|(code, session)| (code, session as usize))
            .collect(),
        chosen: state
            .chosen
            .into_iter()
            .map(|(session, courses)| {
                (
                    session as usize,
                    courses
                        .into_iter()
                        .map(|(code, nrcs)| (code, nrcs.into_iter().collect()))
                        .collect(),
                )
            })
            .collect(),
        manual: state
            .manual
            .into_iter()
            .map(|(session, codes)| (session as usize, codes))
            .collect(),
        special: state
            .special
            .into_iter()
            .map(|(session, label)| (session as usize, label))
            .collect(),
        rule_grants: state.rule_grants.into_iter().collect(),
        credited: state.credited.into_iter().collect(),
    };
    Ok((plan, courses))
}

// --- the correction/error log (OBS-2) --------------------------------------

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct LogEntry {
    // an ISO instant the browser provides; pure code never invents time
    pub at: String,
    pub kind: LogKind,
    pub detail: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum LogKind {
    // the interface guessed wrong and the student corrected it — the
    // highest-value telemetry (OBS-2)
    Correction,
    Error,
    Latency,
    Restore,
}

// bounded ring: the newest entries survive, the excess drains from the
// front — telemetry never grows without bound (OBS-6)
pub fn push_log(log: &mut Vec<LogEntry>, entry: LogEntry) {
    log.push(entry);
    if log.len() > LOG_CAP {
        let excess = log.len() - LOG_CAP;
        log.drain(..excess);
    }
}

pub fn encode_log(log: &[LogEntry]) -> String {
    // expect over `?`: serializing strings provably cannot fail
    serde_json::to_string(log).expect("Log serialization always succeeds")
}

// a corrupt log is telemetry, not user work: dropped to empty, no ceremony
pub fn decode_log(stored: Option<&str>) -> Vec<LogEntry> {
    stored
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn a_plan_and_a_view_round_trip_through_their_envelopes() {
        let mut plan = Plan::default();
        plan.electives.push("GEX-1000".to_string());
        let restored = restore_plan(Some(&encode_plan(&plan)));
        assert_eq!(restored.state, plan);
        assert!(restored.notes.is_empty());
        assert!(restored.backup.is_none());

        let view = View {
            session: 3,
            ..View::default()
        };
        let restored = restore_view(Some(&encode_view(&view)));
        assert_eq!(restored.state, view);
    }

    #[test]
    fn a_first_visit_restores_fresh_and_silent() {
        let restored = restore_plan(None);
        assert_eq!(restored.state, Plan::default());
        assert!(restored.notes.is_empty());
        assert!(restored.backup.is_none());
    }

    #[test]
    fn every_damaged_save_keeps_a_backup_and_says_why() {
        for (name, raw) in [
            ("unreadable", "pas du json"),
            ("versionless", r#"{"state":{}}"#),
            ("newer", r#"{"version":99,"state":{}}"#),
            ("contentless", r#"{"version":1}"#),
            ("incompatible", r#"{"version":1,"state":{"electives":42}}"#),
        ] {
            let restored = restore_plan(Some(raw));
            assert_eq!(restored.state, Plan::default(), "{name}");
            assert_eq!(restored.notes.len(), 1, "{name}");
            assert!(restored.notes[0].contains("conservée"), "{name}");
            assert_eq!(restored.backup.as_deref(), Some(raw), "{name}");
            // the view save walks the same tolerance
            let restored = restore_view(Some(raw));
            assert_eq!(restored.state, View::default(), "{name} (view)");
            assert_eq!(restored.backup.as_deref(), Some(raw), "{name} (view)");
        }
        let unknown = r#"{"version":1,"state":{"futur":1}}"#;
        assert_eq!(restore_view(Some(unknown)).notes.len(), 1, "view unknown");
    }

    #[test]
    fn a_non_object_state_has_no_unknown_keys_to_name() {
        // a non-object stored value: the object diff simply abstains
        assert!(
            unknown_keys(&serde_json::json!(42), &Plan::default()).is_empty()
        );
        assert!(
            unknown_keys(&serde_json::json!(42), &View::default()).is_empty()
        );
    }

    #[test]
    fn unknown_fields_are_named_and_the_save_backed_up() {
        let raw = r#"{"version":1,"state":{"credit_cap":12,
                       "futur_champ":true,"autre":1}}"#;
        let restored = restore_plan(Some(raw));
        assert_eq!(restored.state.credit_cap, 12, "known fields restored");
        assert!(restored.notes[0].contains("autre, futur_champ"));
        assert!(restored.backup.is_some());
    }

    #[test]
    fn an_absent_field_restores_its_default_without_noise() {
        let raw = r#"{"version":1,"state":{"credit_cap":12}}"#;
        let restored = restore_plan(Some(raw));
        assert_eq!(restored.state.credit_cap, 12);
        assert_eq!(restored.state.study_sessions, 8, "defaulted");
        assert!(restored.notes.is_empty(), "normal evolution, no ceremony");
    }

    #[test]
    fn a_restored_plan_heals_a_seat_beyond_its_horizon() {
        // a save from before the horizon rule: GEX-2001 seated at 11 by an
        // old proposal, horizon since shrunk to six study sessions — nine
        // slots, étés included — and verify used to refuse the whole plan
        // for it (« GEX-2001 is pinned to session 11, outside 1..=9 »)
        let stale = Plan {
            study_sessions: 6,
            displayed_placement: BTreeMap::from([
                ("GEX-2001".to_string(), 11),
                ("GEX-1000".to_string(), 2),
            ]),
            pinned_sessions: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            ..Plan::default()
        };
        let restored = restore_plan(Some(&encode_plan(&stale)));
        assert!(
            !restored.state.displayed_placement.contains_key("GEX-2001"),
            "the stray seat falls back to « automatique »"
        );
        assert_eq!(
            restored.state.displayed_placement["GEX-1000"], 2,
            "the pinned seat survives"
        );
        assert_eq!(restored.state.study_sessions, 6);
    }

    #[test]
    fn the_manual_course_list_round_trips_and_survives_damage() {
        let course: Course = serde_json::from_str(
            r#"{"code":"GEX-1234","title":"Maison","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],"seasons":{}}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let manual = vec![course];
        let restored = restore_manual(Some(&encode_manual(&manual)));
        assert_eq!(restored.state, manual);
        assert!(restored.notes.is_empty());

        let damaged = restore_manual(Some("pas du json"));
        assert!(damaged.state.is_empty());
        assert_eq!(damaged.backup.as_deref(), Some("pas du json"));
        let fresh = restore_manual(None);
        assert!(fresh.state.is_empty());
        assert!(fresh.backup.is_none());
    }

    #[test]
    fn the_local_program_list_round_trips_and_survives_damage() {
        let program: ulaval_scheduler_core::Program = serde_json::from_str(
            r#"{"code":"B-GLO","slug":"genie-logiciel","semester":"A26",
                "title":"Génie logiciel","cycle":1,"credits_required":120,
                "mandatory":[],"rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let local = LocalProgram {
            program,
            source_url:
                "https://www.ulaval.ca/etudes/programmes/genie-logiciel"
                    .to_string(),
            imported_at: "2026-08-24T12:00:00Z".to_string(),
            proxy: "corsproxy.io".to_string(),
            anomalies: vec!["règle non reconnue".to_string()],
            origin: crate::import::ProgramOrigin::Url,
        };
        let programs = vec![local];
        let restored =
            restore_local_programs(Some(&encode_local_programs(&programs)));
        assert_eq!(restored.state, programs);
        assert!(restored.notes.is_empty());
        assert!(restored.backup.is_none());

        let damaged = restore_local_programs(Some("pas du json"));
        assert!(damaged.state.is_empty());
        assert_eq!(damaged.notes.len(), 1);
        assert_eq!(damaged.backup.as_deref(), Some("pas du json"));
        let fresh = restore_local_programs(None);
        assert!(fresh.state.is_empty());
        assert!(fresh.backup.is_none());
    }

    // An envelope written before `origin`/`manual` existed: `#[serde(default)]`
    // must restore it as a `Url` import with no manual, not refuse it as
    // damaged (plan item 6's `LocalProgram` grows two fields on a type
    // already living in `localStorage`).
    #[test]
    fn an_envelope_without_origin_or_manual_still_restores() {
        let raw = r#"{"version":1,"state":[{
            "program":{"code":"B-GLO","slug":"genie-logiciel","semester":"A26",
                "title":"Génie logiciel","cycle":1,"credits_required":120,
                "mandatory":[],"rules":[],"concentrations":[],"profiles":[]},
            "source_url":"https://www.ulaval.ca/etudes/programmes/genie-logiciel",
            "imported_at":"2026-08-24T12:00:00Z",
            "proxy":"corsproxy.io",
            "anomalies":[]
        }]}"#;
        let restored = restore_local_programs(Some(raw));
        assert!(restored.notes.is_empty(), "{:?}", restored.notes);
        assert!(restored.backup.is_none());
        assert_eq!(restored.state.len(), 1);
        assert_eq!(
            restored.state[0].origin,
            crate::import::ProgramOrigin::Url,
            "an envelope predating the field defaults to a URL import"
        );
    }

    fn choice(code: &str, semester: &str) -> crate::state::ProgramChoice {
        crate::state::ProgramChoice {
            code: code.to_string(),
            semester: semester.to_string(),
            concentration: None,
            profile: None,
        }
    }

    #[test]
    fn the_shelf_key_names_the_program_and_the_vintage() {
        assert_eq!(
            snapshot_key(&choice("B-GEX", "A26")),
            "gh.v1.plan/B-GEX-A26"
        );
    }

    #[test]
    fn leaving_a_document_shelves_it_and_hands_back_the_picker() {
        let (plan, _) = shared_plan();
        let swap = leave_document(&plan);
        let (key, encoded) = swap.stash.expect("a program was open");
        assert_eq!(key, "gh.v1.plan/B-GEX-A26");
        assert_eq!(restore_plan(Some(&encoded)).state, plan, "shelved whole");
        // the picker document: nothing placed, nothing counted — only the
        // calendar identity survives (the header-count bug's fix)
        assert_eq!(
            swap.next,
            Plan {
                start: plan.start,
                ..Plan::default()
            }
        );
        assert!(swap.notes.is_empty());
        assert!(swap.backup.is_none());
        // a picker document has no shelf to go to
        assert!(leave_document(&Plan::default()).stash.is_none());
    }

    #[test]
    fn entering_a_program_restores_its_shelf_snapshot_exactly() {
        let (shelved, _) = shared_plan();
        let encoded = encode_plan(&shelved);
        let picker = Plan::default();
        // the click's default concentration differs: the snapshot's own
        // choice must win — it is the student's last state
        let click = crate::state::ProgramChoice {
            concentration: Some("Autre".to_string()),
            ..choice("B-GEX", "A26")
        };
        // a session count unrelated to the shelf's own 6 : the restore arm
        // must ignore it entirely, the shelved value is the truth
        let swap = enter_document(&picker, click, Some(&encoded), 99);
        assert_eq!(swap.next, shelved, "restored exactly, the 99 ignored");
        assert!(swap.notes.is_empty());
        assert!(swap.stash.is_none(), "the picker shelves nothing");
    }

    #[test]
    fn the_shelf_key_is_the_identity_of_what_it_restores() {
        // a snapshot whose ProgramChoice diverges from its key (a moved
        // save): code and semester are forced back to the click's
        let (mut divergent, _) = shared_plan();
        if let Some(held) = divergent.program.as_mut() {
            held.code = "AUTRE".to_string();
            held.semester = "H99".to_string();
        }
        let encoded = encode_plan(&divergent);
        let swap = enter_document(
            &Plan::default(),
            choice("B-GEX", "A26"),
            Some(&encoded),
            8,
        );
        let restored = swap.next.program.expect("a program");
        assert_eq!(restored.code, "B-GEX");
        assert_eq!(restored.semester, "A26");
        assert_eq!(
            restored.concentration.as_deref(),
            Some("Génie urbain"),
            "the snapshot's own scope survives"
        );
    }

    #[test]
    fn entering_without_a_shelf_starts_fresh_with_the_start_kept() {
        let picker = Plan {
            start: "H27"
                .parse::<ulaval_scheduler_core::Semester>()
                .unwrap_or_else(|e| panic!("{e}")),
            ..Plan::default()
        };
        let click = choice("B-GIN", "H27");
        let swap = enter_document(&picker, click.clone(), None, 6);
        assert_eq!(
            swap.next,
            crate::state::fresh_plan(picker.start, click, 6),
            "defaults plus the click, calendar identity and sessions carried"
        );
        assert_eq!(swap.next.study_sessions, 6, "the passed count lands");
        assert!(swap.notes.is_empty());
        assert!(swap.backup.is_none());
    }

    #[test]
    fn a_fresh_document_announces_its_default_concentration() {
        // F7: a first contact must not silently compare a program under a
        // concentration the student never picked (rapport
        // étudiante-cegep 2026-08-27)
        let picker = Plan::default();
        let click = crate::state::ProgramChoice {
            concentration: Some("Aéronautique et aérospatiale".to_string()),
            ..choice("B-GPH", "A26")
        };
        let swap = enter_document(&picker, click, None, 8);
        assert_eq!(swap.notes.len(), 1);
        assert!(
            swap.notes[0].contains("Aéronautique et aérospatiale"),
            "{:?}",
            swap.notes
        );
        assert!(swap.notes[0].contains("par défaut"), "{:?}", swap.notes);
    }

    #[test]
    fn a_damaged_shelf_starts_fresh_loudly_and_keeps_the_copy() {
        let picker = Plan::default();
        let click = choice("B-GIN", "H27");
        let swap =
            enter_document(&picker, click.clone(), Some("pas du json"), 6);
        assert_eq!(
            swap.next,
            crate::state::fresh_plan(picker.start, click, 6)
        );
        assert_eq!(swap.notes.len(), 1);
        assert!(swap.notes[0].contains("conservée"), "{:?}", swap.notes);
        assert_eq!(swap.backup.as_deref(), Some("pas du json"));
    }

    #[test]
    fn the_import_stashes_only_across_documents() {
        let (current, _) = shared_plan();
        // same (program, vintage): no stash — a later « changer » must
        // not overwrite the shelf with the shared version
        assert!(import_stash(&current, &current).is_none());
        let mut other = current.clone();
        if let Some(held) = other.program.as_mut() {
            held.semester = "A24".to_string();
        }
        let (key, encoded) =
            import_stash(&current, &other).expect("another vintage");
        assert_eq!(key, "gh.v1.plan/B-GEX-A26");
        assert_eq!(restore_plan(Some(&encoded)).state, current);
        // a shared plan with no program still stashes the current one
        assert!(import_stash(&current, &Plan::default()).is_some());
        // a picker current has nothing to stash
        assert!(import_stash(&Plan::default(), &other).is_none());
    }

    // user story 10, the contract: A rempli → changer → B rempli →
    // changer → re-choisir A redonne exactement le même document
    #[test]
    fn switching_programs_and_back_restores_the_same_document() {
        let (plan_a, _) = shared_plan();
        let left_a = leave_document(&plan_a);
        let (key_a, stash_a) = left_a.stash.clone().expect("A shelved");
        let entered_b =
            enter_document(&left_a.next, choice("B-GIN", "A26"), None, 8);
        let mut plan_b = entered_b.next;
        plan_b.displayed_placement.insert("IFT-1000".to_string(), 1);
        let left_b = leave_document(&plan_b);
        let (key_b, _) = left_b.stash.clone().expect("B shelved");
        assert_ne!(key_a, key_b, "two documents, two shelves");
        let choice_a = plan_a.program.clone().expect("A has a program");
        let back = enter_document(&left_b.next, choice_a, Some(&stash_a), 8);
        assert_eq!(back.next, plan_a, "byte-identical document");
    }

    // a plan touching every field, deterministic — the frozen-string test
    // depends on it never changing
    fn shared_plan() -> (Plan, Vec<Course>) {
        let mut plan = Plan {
            program: Some(crate::state::ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: Some("Génie urbain".to_string()),
                profile: None,
            }),
            study_sessions: 6,
            summers_open: true,
            credit_cap: 17,
            concomitant: true,
            preparatory_done: false,
            ..Plan::default()
        };
        plan.electives.push("ANL-2020".to_string());
        plan.pinned_sessions.insert("GEX-1000".to_string(), 1);
        plan.displayed_placement.insert("GEX-1000".to_string(), 1);
        plan.displayed_placement.insert("GCI-1007".to_string(), 2);
        plan.chosen.insert(
            1,
            BTreeMap::from([(
                "GEX-1000".to_string(),
                BTreeSet::from(["12345".to_string(), "12346".to_string()]),
            )]),
        );
        plan.manual.insert(2, vec!["ZZZ-9000".to_string()]);
        plan.special.insert(3, "à l'étranger".to_string());
        plan.rule_grants
            .insert("ZZZ-9000".to_string(), "p/Règle 2".to_string());
        plan.credited.insert("GAE-1000".to_string());
        let course: Course = serde_json::from_str(
            r#"{"code":"ZZZ-9000","title":"Maison","credits":3,"cycle":1,
                "prerequisites":null,"equivalents":[],
                "seasons":{"winter":{"last_offered":2026,"options":null}}}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        (plan, vec![course])
    }

    #[test]
    fn the_organigramme_link_round_trips_whole() {
        let (plan, courses) = shared_plan();
        let encoded = encode_organigramme(&plan, &courses);
        let (back, back_courses) =
            decode_organigramme(&encoded).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, plan, "every field survives");
        assert_eq!(back_courses, courses);
        // a bare default plan round-trips too (no program, empty maps)
        let bare = Plan::default();
        let encoded = encode_organigramme(&bare, &[]);
        let (back, back_courses) =
            decode_organigramme(&encoded).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, bare);
        assert!(back_courses.is_empty());
        // every start season survives the discriminant round trip
        for raw in ["H27", "E27"] {
            let seasonal = Plan {
                start: raw
                    .parse::<ulaval_scheduler_core::Semester>()
                    .unwrap_or_else(|e| panic!("{e}")),
                ..Plan::default()
            };
            let encoded = encode_organigramme(&seasonal, &[]);
            let (back, _) = decode_organigramme(&encoded)
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(back, seasonal, "{raw}");
        }
    }

    #[test]
    fn deflate_is_kept_only_when_it_helps() {
        use base64::Engine;
        let header = |encoded: &str| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(encoded)
                .unwrap_or_else(|e| panic!("{e}"))[0]
        };
        // incompressible bytes (a limited-alphabet state always shrinks a
        // little, so the raw branch is proven on `pack` directly): deflate
        // overhead loses and the raw bytes ride
        let noise: Vec<u8> = (0..64u32).map(|i| (i * 37 + 13) as u8).collect();
        let (payload, flag) = pack(noise.clone());
        assert_eq!(flag, 0, "raw kept");
        assert_eq!(payload, noise);
        // a redundant state (many similar codes): deflate wins
        let mut big = Plan::default();
        for i in 0..200 {
            big.displayed_placement.insert(format!("GEX-{i:04}"), 1);
        }
        let encoded = encode_organigramme(&big, &[]);
        assert_eq!(header(&encoded) & FLAG_DEFLATE, FLAG_DEFLATE);
        let (back, _) =
            decode_organigramme(&encoded).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, big);
    }

    #[test]
    fn the_frozen_string_still_encodes_byte_for_byte() {
        // The lock on the frame: postcard encodes positionally, so a field
        // reordered, removed or inserted changes this string. Until the
        // first deployment, updating it here is the whole migration; after
        // it, a change means a version 2 instead.
        let (plan, courses) = shared_plan();
        assert_eq!(encode_organigramme(&plan, &courses), FROZEN);
        let (back, back_courses) =
            decode_organigramme(FROZEN).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, plan);
        assert_eq!(back_courses, courses);
    }

    #[test]
    fn the_link_carries_the_prerequisite_corrections() {
        // without them the recipient would see another verdict than the
        // sender's, which is the one thing the link must never do
        let (mut plan, courses) = shared_plan();
        plan.prereq_overrides.insert(
            "GCI-2000".to_string(),
            ulaval_scheduler_core::PrereqOverride {
                text: "GCI-1000 ET MAT-1902".to_string(),
                official: Some("GCI-1005".to_string()),
            },
        );
        let link = encode_organigramme(&plan, &courses);
        let (back, _) =
            decode_organigramme(&link).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back.prereq_overrides, plan.prereq_overrides);
    }

    const FROZEN: &str = "gUWQwUrDQBCGMymVIKj4BmUuXhJMUo2YW5QSBPXgSSIiazItC8smbjaKlLyIJ4_2OfI2PoW7Ads5DP_8_893GJheBfniESZZnMBBPmwkp1mnXhmXjvN7tAfH4ICX3d8GcRiH4JluEIVhCK6XX99YeeHuTDPbw51G8fzsfNwJuOAVRRFc2tbkcPieiZNhoxWTK1K7aL85fRh-VoJmsQFlixHkwBessawrwhT_m-ij5lpY647xtpbGKBVVXLeYzo3-LG0Y-dgoUvTW8ZZrMpnshPDRGu9MkLT1p2cfW2IGYo41fnCpSVklWKtf6uXSACpMzQMSH-tG87FoQX3f_wE";

    #[test]
    fn every_broken_link_is_a_typed_french_error() {
        use base64::Engine;
        let engine = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let of_bytes = |bytes: &[u8]| engine.encode(bytes);
        assert_eq!(
            decode_organigramme(""),
            Err(OrganigrammeShareError::Empty)
        );
        assert_eq!(
            decode_organigramme("!!!"),
            Err(OrganigrammeShareError::Base64)
        );
        assert_eq!(
            decode_organigramme(&of_bytes(&[0x02])),
            Err(OrganigrammeShareError::UnknownVersion(2))
        );
        assert_eq!(
            decode_organigramme(&of_bytes(&[SHARE_VERSION, 0xFF, 0xFF])),
            Err(OrganigrammeShareError::Payload)
        );
        // a link inflating past the cap is refused, not obeyed
        let bomb = miniz_oxide::deflate::compress_to_vec(
            &vec![0u8; MAX_DECOMPRESSED + 1],
            10,
        );
        let mut bytes = vec![SHARE_VERSION | FLAG_DEFLATE];
        bytes.extend_from_slice(&bomb);
        assert_eq!(
            decode_organigramme(&of_bytes(&bytes)),
            Err(OrganigrammeShareError::Inflate)
        );
        // a payload with an unknown season or an unreadable manual course
        let broken_season = Share {
            start_season: 9,
            ..share_from(&Plan::default(), &[])
        };
        let raw = postcard::to_allocvec(&broken_season)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut bytes = vec![SHARE_VERSION];
        bytes.extend_from_slice(&raw);
        assert_eq!(
            decode_organigramme(&of_bytes(&bytes)),
            Err(OrganigrammeShareError::UnknownSeason(9))
        );
        let broken_course = Share {
            manual_courses: vec!["pas du json".to_string()],
            ..share_from(&Plan::default(), &[])
        };
        let raw = postcard::to_allocvec(&broken_course)
            .unwrap_or_else(|e| panic!("{e}"));
        let mut bytes = vec![SHARE_VERSION];
        bytes.extend_from_slice(&raw);
        assert!(matches!(
            decode_organigramme(&of_bytes(&bytes)),
            Err(OrganigrammeShareError::ManualCourse(_))
        ));
        // every message reads in French, never empty
        for error in [
            OrganigrammeShareError::Empty,
            OrganigrammeShareError::Base64,
            OrganigrammeShareError::Inflate,
            OrganigrammeShareError::UnknownVersion(2),
            OrganigrammeShareError::Payload,
            OrganigrammeShareError::UnknownSeason(9),
            OrganigrammeShareError::ManualCourse("x".to_string()),
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn the_log_is_a_bounded_ring_that_keeps_the_newest() {
        let mut log = Vec::new();
        for i in 0..(LOG_CAP + 5) {
            push_log(
                &mut log,
                LogEntry {
                    at: format!("t{i}"),
                    kind: LogKind::Correction,
                    detail: format!("entrée {i}"),
                },
            );
        }
        assert_eq!(log.len(), LOG_CAP);
        assert_eq!(log[0].detail, "entrée 5", "the oldest five drained");

        let decoded = decode_log(Some(&encode_log(&log)));
        assert_eq!(decoded, log);
        assert!(decode_log(None).is_empty());
        assert!(decode_log(Some("garbage")).is_empty());
    }
}
