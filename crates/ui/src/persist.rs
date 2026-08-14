use std::collections::BTreeMap;

use ulaval_scheduler_core::Course;

use crate::state::{Plan, View};

pub const PLAN_KEY: &str = "gh.v1.plan";
pub const VIEW_KEY: &str = "gh.v1.view";
pub const LOG_KEY: &str = "gh.v1.log";
pub const MANUAL_KEY: &str = "gh.v1.cours-manuels";
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
    restore(stored, "du plan")
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

// --- the whole-organigramme share (fragment codec) --------------------------
// The link must carry *everything* — the recipient pastes it and sees the
// organigramme whole, no adjustment (note 9, 2026-08-13). Pipeline:
// state → postcard → raw deflate (kept only if smaller) → base64url, with
// one version|flag header byte in front (`docs/conception/shareable_link.md`).

const SHARE_VERSION: u8 = 1;
const FLAG_DEFLATE: u8 = 0x80;
// a hostile link must not be a zip bomb against someone else's tab
const MAX_DECOMPRESSED: usize = 256 * 1024;

// FROZEN — postcard encodes positionally: never reorder, remove, or
// insert a field here. A future format is a `ShareV2` with its own
// version byte and a migration, not an edit of this struct.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct ShareV1 {
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
    chosen: Vec<(u32, SessionChosenV1)>,
    manual: Vec<(u32, Vec<String>)>,
    special: Vec<(u32, String)>,
    rule_grants: Vec<(String, String)>,
    // each hand-entered Course as its own JSON — self-describing on
    // purpose, so core's Course may evolve without corrupting old links
    manual_courses: Vec<String>,
}

// one session's pinned sections: (code, sorted NRC set) — frozen too
type SessionChosenV1 = Vec<(String, Vec<String>)>;

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
            let state: ShareV1 = postcard::from_bytes(&raw)
                .map_err(|_| OrganigrammeShareError::Payload)?;
            share_into(state)
        }
        version => Err(OrganigrammeShareError::UnknownVersion(version)),
    }
}

fn share_from(plan: &Plan, manual_courses: &[Course]) -> ShareV1 {
    let session_map = |map: &BTreeMap<String, usize>| {
        map.iter()
            .map(|(code, &session)| (code.clone(), session as u32))
            .collect()
    };
    ShareV1 {
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
    state: ShareV1,
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
    fn the_frozen_v1_string_still_decodes_byte_for_byte() {
        // the compatibility lock: if this fails, ShareV1 changed shape and
        // every link ever shared is broken — write a ShareV2 instead
        let (plan, courses) = shared_plan();
        assert_eq!(encode_organigramme(&plan, &courses), FROZEN_V1);
        let (back, back_courses) =
            decode_organigramme(FROZEN_V1).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, plan);
        assert_eq!(back_courses, courses);
    }

    const FROZEN_V1: &str = "gUWPQUrEQBBFUxlGgqDiDUJt3CSYZDRidqNIENSFK4mItEnN0NB0YndHkSEXceXSOUdu4ynsHtCpRfHr_8eHgulFXF49wGSe5bBXjmvJKezVC-PS834OduAQPAjmdzdxlmQJBJaN0yRJwA_Ky2snz_ytaef_8KdpNjs53ewcfAiqqorPHTXZH79CcTSujWJySWob7XbH9-P3UlCYwSessG4bwgL_YozQcCOcdcu4bqU1akUNNxqLmdUftQvTCDtFil57rrkhm8leiAid8cYESYc_PkWoidkSe6zwnUtDyinBtHluFwtb0GBhv84jbDvDN6ArGobhFw";

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
            decode_organigramme(&of_bytes(&[0x01, 0xFF, 0xFF])),
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
        let broken_season = ShareV1 {
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
        let broken_course = ShareV1 {
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
