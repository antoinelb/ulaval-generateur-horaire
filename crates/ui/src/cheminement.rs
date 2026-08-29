// Reading and writing one cheminement type as a file: the JSON a student
// is handed (`data/cheminements/{code}-{semester}[-{concentration}].json`)
// in, an organigramme out — and back out again to the same shape. Pure and
// testable; the file picker and the download are the view's business (the
// `capsule.rs` / `CapsuleDrawer` split is the model this module copies;
// ADR `2026-08-un-cheminement-par-fichier`).

use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    horizon_sessions, session_semesters, Cheminement, CheminementSession,
    Season, Semester, MAX_STUDY_SESSIONS,
};

use crate::export::provenance::ExportProvenance;
use crate::state::{self, Plan};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheminementError {
    #[error("this file is not a cheminement : {detail}")]
    Unreadable { detail: String },
    // a timeline opening on a summer names no admission session, and every
    // organigramme is built around one. A timeline that is *empty* is a
    // different matter: it says nothing about the calendar rather than
    // something impossible, and loads (ADR
    // `2026-08-cheminement-sans-session-charge-quand-meme`)
    #[error("the cheminement names no admission session")]
    NoAdmission,
    #[error("the cheminement runs over {max} study sessions")]
    TooLong { study_sessions: usize, max: usize },
    #[error("the cheminement holds no course this catalogue knows")]
    Empty,
    // raised by the view itself, before `load` is even callable: without
    // the snapshot there is no catalogue to hold the sigle gate against
    #[error("the course catalogue is not loaded yet")]
    CatalogueUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheminementLoad {
    pub application: CheminementApplication,
    pub summary: CheminementSummary,
}

// What the file asks the plan to become — every field already resolved
// against the horizon, so `apply_to_plan` decides nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheminementApplication {
    // `None` when the file carries no session at all — « des cours réussis,
    // rien de planifié ». The file then says nothing about the calendar, so
    // the plan keeps its own start and horizon rather than being handed an
    // invented one (ADR `2026-08-cheminement-sans-session-charge-quand-meme`)
    pub timeline: Option<CheminementTimeline>,
    pub summers_open: bool,
    // code → 1-based session index
    pub pinned: BTreeMap<String, usize>,
    pub credited: BTreeSet<String>,
}

// The two calendar facts a timeline carries, together: a start with no
// length, or a length with no start, is not a state any file can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheminementTimeline {
    pub start: Semester,
    pub study_sessions: usize,
}

// Every field a finished French sentence — `rsx!` only prints them (AP-5).
// The headline carries the counts; `ignored` names what the file asked for
// and the plan refused, one line each, because dropping it silently is the
// one thing this codebase never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheminementSummary {
    pub ignored: Vec<String>,
    pub headline: String,
}

// `known` is the catalogue's sigles (`Snapshot::by_code`'s keys). The
// official grids are full of placeholders that are not courses at all —
// « OPT-ION1 », « AUC-HOIX », « LAN-GUES » — so the gate is not an edge
// case here, it is the normal path (same discipline as ADR
// `2026-08-sigles-inconnus-du-releve-ignores`).
pub fn load(
    raw: &str,
    known: &BTreeSet<String>,
) -> Result<CheminementLoad, CheminementError> {
    let cheminement: Cheminement =
        serde_json::from_str(raw).map_err(|error| {
            CheminementError::Unreadable {
                detail: error.to_string(),
            }
        })?;
    apply(&cheminement, known)
}

fn apply(
    cheminement: &Cheminement,
    known: &BTreeSet<String>,
) -> Result<CheminementLoad, CheminementError> {
    let timeline = read_timeline(cheminement)?;

    // `Semester` is not `Ord`, and a horizon is at most 17 entries long:
    // the calendar itself is the index, walked rather than hashed. A file
    // with no session has no calendar and no course to look up in one.
    let semesters = timeline.map_or_else(Vec::new, |timeline| {
        let seasons =
            horizon_sessions(timeline.start.season, timeline.study_sessions);
        session_semesters(timeline.start, &seasons)
    });
    let index_of = |semester: Semester| {
        semesters
            .iter()
            .position(|held| *held == semester)
            .map(|offset| offset + 1)
    };

    let mut pinned = BTreeMap::new();
    // code → the session that first claimed it: the duplicate guard and
    // the sentence it prints are one map, so naming the winner needs no
    // lookup that could fail
    let mut placed_in: BTreeMap<String, Semester> = BTreeMap::new();
    let mut credited = BTreeSet::new();
    let mut ignored = Vec::new();
    let mut summers_open = false;

    for session in &cheminement.sessions {
        let semester = session.semester;
        let Some(index) = index_of(semester) else {
            for code in &session.courses {
                ignored.push(format!("{code} — {semester} hors de l'horizon"));
            }
            continue;
        };
        if semester.season == Season::Summer && !session.courses.is_empty() {
            summers_open = true;
        }
        for code in &session.courses {
            if !known.contains(code) {
                ignored
                    .push(format!("{code} — introuvable dans le catalogue"));
                continue;
            }
            // the same sigle twice in one grid is real data, not a typo:
            // `B-GEX-A26` schedules GMN-2902 in H28 *and* H29. The earlier
            // session wins and the later one is named, because a map keyed
            // by code can only hold one and the silent winner would be the
            // wrong one
            if let Some(first) = placed_in.get(code) {
                ignored.push(format!(
                    "{code} — déjà placé en {first}, seconde occurrence \
                     ignorée"
                ));
                continue;
            }
            placed_in.insert(code.clone(), semester);
            pinned.insert(code.clone(), index);
        }
    }

    for code in &cheminement.completed {
        if !known.contains(code) {
            ignored.push(format!("{code} — introuvable dans le catalogue"));
            continue;
        }
        // a course cannot be both credited on admission and scheduled: the
        // grid itself says which, and the session wins
        if pinned.contains_key(code) {
            ignored.push(format!(
                "{code} — déjà placé dans la grille, non crédité"
            ));
            continue;
        }
        credited.insert(code.clone());
    }

    if pinned.is_empty() && credited.is_empty() {
        return Err(CheminementError::Empty);
    }

    let headline = match timeline {
        Some(timeline) => format!(
            "{} {}, {} {}, {} {}, {} {}.",
            timeline.study_sessions,
            agree(timeline.study_sessions, "session", "sessions"),
            pinned.len(),
            agree(pinned.len(), "cours placé", "cours placés"),
            credited.len(),
            agree(credited.len(), "crédité", "crédités"),
            ignored.len(),
            agree(ignored.len(), "ignoré", "ignorés"),
        ),
        // the file named no session, so the calendar it did not describe is
        // the calendar that stays: saying so is the whole difference
        // between a load and something that looks like a no-op
        None => format!(
            "Aucune session dans le fichier : l'horizon n'a pas bougé, \
             {} {}, {} {}.",
            credited.len(),
            agree(credited.len(), "crédité", "crédités"),
            ignored.len(),
            agree(ignored.len(), "ignoré", "ignorés"),
        ),
    };

    Ok(CheminementLoad {
        application: CheminementApplication {
            timeline,
            summers_open,
            pinned,
            credited,
        },
        summary: CheminementSummary { ignored, headline },
    })
}

// The calendar the file describes, or `None` when it describes none. A
// timeline that opens on a summer is refused rather than defaulted: a
// summer is never an admission session, so guessing one would be inventing
// a fact the file never carried.
fn read_timeline(
    cheminement: &Cheminement,
) -> Result<Option<CheminementTimeline>, CheminementError> {
    let Some(first) = cheminement.sessions.first() else {
        return Ok(None);
    };
    let start = first.semester;
    if start.season == Season::Summer {
        return Err(CheminementError::NoAdmission);
    }
    let study_sessions = cheminement
        .sessions
        .iter()
        .filter(|session| session.semester.season != Season::Summer)
        .count();
    if study_sessions > MAX_STUDY_SESSIONS {
        return Err(CheminementError::TooLong {
            study_sessions,
            max: MAX_STUDY_SESSIONS,
        });
    }
    Ok(Some(CheminementTimeline {
        start,
        study_sessions,
    }))
}

// « 1 crédité » but « 0 ignoré » : French agreement singularizes both 0
// and 1, only 2 and above take the plural form.
fn agree(
    count: usize,
    singular: &'static str,
    plural: &'static str,
) -> &'static str {
    if count <= 1 {
        singular
    } else {
        plural
    }
}

// Loading a cheminement *replaces* the document, it does not merge into
// it — the student asked for this grid, not for this grid mixed with what
// was there. Everything naming a course or a session restarts empty:
// leaving `electives` standing was enough for `auto_propose` to seat them
// back on top of the freshly loaded grid, which reads as courses adding up.
//
// Rebuilt from `Plan::default()` rather than cleared field by field: a
// field added to `Plan` later is then wiped by default, which is the safe
// direction — the opposite forgets one and leaks it into the new grid.
// Safe because the whole call is wrapped in one labelled `edit_plan` act,
// so ctrl-z brings the old document back entire (ACT-2).
pub fn apply_to_plan(plan: &mut Plan, application: &CheminementApplication) {
    // the settings the file says nothing about are not the file's business
    let program = plan.program.clone();
    let credit_cap = plan.credit_cap;
    let concomitant = plan.concomitant;
    let preparatory_done = plan.preparatory_done;
    let prereq_overrides = std::mem::take(&mut plan.prereq_overrides);
    // an été carrying courses opens the summers; a grid with none never
    // closes a student's own choice to use them (same rule as the relevé)
    let summers_open = plan.summers_open || application.summers_open;
    // a file that named no session named no calendar either: the plan's own
    // start and horizon stand, exactly as `credit_cap` and the rest do
    let timeline = application.timeline.unwrap_or(CheminementTimeline {
        start: plan.start,
        study_sessions: plan.study_sessions,
    });

    *plan = Plan {
        program,
        credit_cap,
        concomitant,
        preparatory_done,
        prereq_overrides,
        summers_open,
        start: timeline.start,
        study_sessions: timeline.study_sessions,
        // a cheminement type is a plan, not a past: nothing in it is behind
        // the student, so the solver keeps every session open
        completed_sessions: 0,
        ..Plan::default()
    };

    for (code, &index) in &application.pinned {
        plan.pinned_sessions.insert(code.clone(), index);
        plan.displayed_placement.insert(code.clone(), index);
    }
    for code in &application.credited {
        state::credit_code(plan, code);
    }
}

// The gabarit the « ? » help prints and the « Copier le gabarit » button
// writes to the clipboard. It lives here, beside the reader, and is tested
// against it: the help must never show a shape `load` would refuse.
pub const TEMPLATE: &str = r#"{
  "completed": ["GMC-1024"],
  "sessions": [
    { "semester": "A26", "courses": ["GMC-1001", "MAT-1900"] },
    { "semester": "H27", "courses": ["GMC-2001"] },
    { "semester": "E27", "courses": [] }
  ]
}"#;

// --- writing one out -------------------------------------------------------

// The exported document: a `Cheminement` and nothing else, plus the
// provenance block EXP-1 demands. `Cheminement` has no
// `deny_unknown_fields`, so what this writes, `load` above reads back —
// the provenance riding along unread (ADR
// `2026-08-un-cheminement-par-fichier`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
struct Exported {
    #[serde(flatten)]
    cheminement: Cheminement,
    provenance: ExportedProvenance,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
struct ExportedProvenance {
    exported_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    program: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    semester: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concentration: Option<String>,
    app: String,
    data: String,
    version: String,
    repo: String,
}

// `generated_at` is handed in by the view (`browser::now_iso`), exactly as
// `export::provenance` takes it — this module never reads a clock, which
// is what lets it be tested against a fixed instant.
pub fn export(
    plan: &Plan,
    generated_at: &str,
    provenance: &ExportProvenance,
) -> String {
    let seasons = horizon_sessions(plan.start.season, plan.study_sessions);
    let sessions = session_semesters(plan.start, &seasons)
        .into_iter()
        .enumerate()
        .map(|(offset, semester)| CheminementSession {
            semester,
            // the whole timeline is written, empty summers included: it is
            // what makes `study_sessions` come back the same on re-reading
            courses: state::session_codes(plan, offset + 1),
        })
        .collect();
    let exported = Exported {
        cheminement: Cheminement {
            completed: plan.credited.iter().cloned().collect(),
            sessions,
        },
        provenance: ExportedProvenance {
            exported_at: generated_at.to_string(),
            program: plan.program.as_ref().map(|program| program.code.clone()),
            semester: plan
                .program
                .as_ref()
                .map(|program| program.semester.clone()),
            concentration: plan
                .program
                .as_ref()
                .and_then(|program| program.concentration.clone()),
            app: provenance.build.clone(),
            data: provenance.data.clone(),
            version: provenance.version.clone(),
            repo: provenance.repo.clone(),
        },
    };
    // expect over `?`: serializing strings and vectors provably cannot
    // fail — the same reasoning, and the same wording, as `persist::encode`
    serde_json::to_string_pretty(&exported)
        .expect("Cheminement serialization always succeeds")
}

// The name the browser suggests — the very name the same cheminement
// carries under `data/cheminements/`, so an exported correction is a drop-in
// replacement rather than a file someone has to rename.
pub fn export_file_name(plan: &Plan) -> String {
    let Some(program) = plan.program.as_ref() else {
        return "cheminement.json".to_string();
    };
    let stem = format!("{}-{}", program.code, program.semester);
    match program.concentration.as_deref().map(snake_case) {
        Some(concentration) if !concentration.is_empty() => {
            format!("{stem}-{concentration}.json")
        }
        _ => format!("{stem}.json"),
    }
}

// « Sciences de la nature - Profil international » →
// `sciences_de_la_nature_profil_international`. Accents are folded rather
// than kept: the name travels through URLs, shells and file pickers on
// three operating systems.
fn snake_case(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    for character in label.chars() {
        let folded = fold_accent(character);
        if folded.is_ascii_alphanumeric() {
            out.extend(folded.to_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn fold_accent(character: char) -> char {
    match character {
        'à' | 'â' | 'ä' | 'À' | 'Â' | 'Ä' => 'a',
        'ç' | 'Ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'î' | 'ï' | 'Î' | 'Ï' => 'i',
        'ô' | 'ö' | 'Ô' | 'Ö' => 'o',
        'ù' | 'û' | 'ü' | 'Ù' | 'Û' | 'Ü' => 'u',
        other => other,
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::state::ProgramChoice;

    fn known(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| (*code).to_string()).collect()
    }

    const GRID: &str = r#"{
  "completed": ["GMC-1024"],
  "sessions": [
    { "semester": "A26", "courses": ["GMC-1001", "OPT-ION1"] },
    { "semester": "H27", "courses": ["MAT-1900"] },
    { "semester": "E27", "courses": [] }
  ]
}"#;

    #[test]
    fn a_grid_becomes_an_application_and_a_bilan() {
        let loaded = load(GRID, &known(&["GMC-1001", "MAT-1900", "GMC-1024"]))
            .expect("the grid loads");
        let timeline = loaded
            .application
            .timeline
            .expect("the grid names a calendar");
        assert_eq!(timeline.start.to_string(), "A26");
        assert_eq!(timeline.study_sessions, 2);
        assert!(!loaded.application.summers_open);
        assert_eq!(loaded.application.pinned["GMC-1001"], 1);
        assert_eq!(loaded.application.pinned["MAT-1900"], 2);
        assert!(loaded.application.credited.contains("GMC-1024"));
        // the placeholder is named, never placed and never silent
        assert_eq!(loaded.summary.ignored.len(), 1);
        assert!(
            loaded.summary.ignored[0].contains("OPT-ION1"),
            "{:?}",
            loaded.summary.ignored
        );
        assert!(
            loaded
                .summary
                .headline
                .starts_with("2 sessions, 2 cours placés"),
            "{}",
            loaded.summary.headline
        );
    }

    #[test]
    fn a_summer_holding_courses_opens_the_summers() {
        let raw = r#"{ "completed": [], "sessions": [
            { "semester": "A26", "courses": ["GMC-1001"] },
            { "semester": "H27", "courses": [] },
            { "semester": "E27", "courses": ["MAT-1900"] } ] }"#;
        let loaded = load(raw, &known(&["GMC-1001", "MAT-1900"]))
            .expect("the grid loads");
        assert!(loaded.application.summers_open);
        assert_eq!(loaded.application.pinned["MAT-1900"], 3);
    }

    #[test]
    fn the_same_sigle_twice_keeps_the_earlier_session_and_says_so() {
        let raw = r#"{ "completed": [], "sessions": [
            { "semester": "A26", "courses": ["GMN-2902"] },
            { "semester": "H27", "courses": ["GMN-2902"] } ] }"#;
        let loaded = load(raw, &known(&["GMN-2902"])).expect("the grid loads");
        assert_eq!(loaded.application.pinned["GMN-2902"], 1);
        assert!(
            loaded.summary.ignored[0].contains("déjà placé en A26"),
            "{:?}",
            loaded.summary.ignored
        );
    }

    #[test]
    fn a_completed_course_also_scheduled_stays_in_its_session() {
        let raw = r#"{ "completed": ["GMC-1001"], "sessions": [
            { "semester": "A26", "courses": ["GMC-1001"] } ] }"#;
        let loaded = load(raw, &known(&["GMC-1001"])).expect("the grid loads");
        assert!(loaded.application.credited.is_empty());
        assert!(
            loaded.summary.ignored[0].contains("non crédité"),
            "{:?}",
            loaded.summary.ignored
        );
    }

    #[test]
    fn a_session_outside_the_horizon_names_its_courses() {
        // H31 belongs to no session of a two-session horizon opening in A26
        let raw = r#"{ "completed": [], "sessions": [
            { "semester": "A26", "courses": ["GMC-1001"] },
            { "semester": "H31", "courses": ["MAT-1900"] } ] }"#;
        let loaded = load(raw, &known(&["GMC-1001", "MAT-1900"]))
            .expect("the grid loads");
        assert!(!loaded.application.pinned.contains_key("MAT-1900"));
        assert!(
            loaded.summary.ignored[0].contains("H31 hors de l'horizon"),
            "{:?}",
            loaded.summary.ignored
        );
    }

    #[test]
    fn an_unknown_completed_sigle_is_named_not_credited() {
        let raw = r#"{ "completed": ["OPT-ION3"], "sessions": [
            { "semester": "A26", "courses": ["GMC-1001"] } ] }"#;
        let loaded = load(raw, &known(&["GMC-1001"])).expect("the grid loads");
        assert!(loaded.application.credited.is_empty());
        assert!(
            loaded.summary.ignored[0].contains("introuvable"),
            "{:?}",
            loaded.summary.ignored
        );
    }

    #[test]
    fn every_refusal_is_typed() {
        assert!(matches!(
            load("pas du json", &known(&[])),
            Err(CheminementError::Unreadable { .. })
        ));
        // a timeline that opens on a summer names no admission; an *empty*
        // one names no calendar at all, which is a different verdict and
        // has its own test above
        assert_eq!(
            load(
                r#"{ "completed": [], "sessions": [
                    { "semester": "E27", "courses": [] } ] }"#,
                &known(&[])
            ),
            Err(CheminementError::NoAdmission)
        );
        assert_eq!(
            load(
                r#"{ "completed": [], "sessions": [
                    { "semester": "A26", "courses": ["OPT-ION1"] } ] }"#,
                &known(&[])
            ),
            Err(CheminementError::Empty)
        );
        let long: String = (0..=MAX_STUDY_SESSIONS)
            .map(|offset| {
                format!(
                    r#"{{ "semester": "{}{:02}", "courses": [] }},"#,
                    if offset % 2 == 0 { "A" } else { "H" },
                    26 + offset / 2
                )
            })
            .collect();
        assert!(matches!(
            load(
                &format!(
                    r#"{{ "completed": [], "sessions": [{}] }}"#,
                    long.trim_end_matches(',')
                ),
                &known(&[])
            ),
            Err(CheminementError::TooLong { .. })
        ));
        // the view raises this one itself; naming it here keeps the
        // Display string honest
        assert!(CheminementError::CatalogueUnavailable
            .to_string()
            .contains("catalogue"));
    }

    #[test]
    fn the_help_gabarit_is_a_file_this_reader_accepts() {
        let loaded = load(
            TEMPLATE,
            &known(&["GMC-1001", "MAT-1900", "GMC-2001", "GMC-1024"]),
        )
        .expect("the gabarit shown in the help must load");
        assert!(loaded.summary.ignored.is_empty());
        assert_eq!(
            loaded
                .application
                .timeline
                .map(|timeline| timeline.study_sessions),
            Some(2)
        );
    }

    // The bug this test was written for: a file that is only « des cours
    // réussis, rien de planifié » used to be refused as `NoAdmission`, and
    // the refusal read as nothing happening at all. It loads (ADR
    // `2026-08-cheminement-sans-session-charge-quand-meme`).
    #[test]
    fn a_cheminement_with_no_session_credits_its_courses_and_says_so() {
        let raw = r#"{
  "completed": ["GCI-1000", "GCI-1011", "GLG-1000"],
  "sessions": []
}"#;
        let loaded = load(raw, &known(&["GCI-1000", "GCI-1011", "GLG-1000"]))
            .expect("a cheminement of credits alone loads");
        assert_eq!(loaded.application.timeline, None);
        assert_eq!(loaded.application.credited.len(), 3);
        assert!(loaded.application.pinned.is_empty());
        assert!(loaded.summary.ignored.is_empty());
        // the bilan says what did *not* move, or the load reads as a no-op
        assert_eq!(
            loaded.summary.headline,
            "Aucune session dans le fichier : l'horizon n'a pas bougé, \
             3 crédités, 0 ignoré."
        );
    }

    // the same file, still not silent about what it could not use
    #[test]
    fn a_cheminement_with_no_session_still_names_the_sigles_it_refuses() {
        let raw = r#"{ "completed": ["GCI-1000", "OPT-ION1"],
                       "sessions": [] }"#;
        let loaded =
            load(raw, &known(&["GCI-1000"])).expect("the file still loads");
        assert_eq!(loaded.summary.ignored.len(), 1);
        assert!(
            loaded.summary.ignored[0].contains("OPT-ION1"),
            "{:?}",
            loaded.summary.ignored
        );
        assert_eq!(
            loaded.summary.headline,
            "Aucune session dans le fichier : l'horizon n'a pas bougé, \
             1 crédité, 1 ignoré."
        );
    }

    // the neighbours of the bug: nothing to load, and nothing at all — both
    // refused by name, neither in silence
    #[test]
    fn an_empty_document_and_a_missing_field_are_both_named() {
        assert_eq!(
            load(r#"{ "completed": [], "sessions": [] }"#, &known(&[])),
            Err(CheminementError::Empty)
        );
        let missing = load(r#"{ "completed": ["GCI-1000"] }"#, &known(&[]))
            .expect_err("a document without « sessions » is refused");
        let CheminementError::Unreadable { detail } = missing else {
            panic!("the refusal must name the missing field: {missing:?}");
        };
        assert!(detail.contains("sessions"), "{detail}");
    }

    #[test]
    fn a_cheminement_with_no_session_leaves_the_horizon_where_it_was() {
        let mut plan = Plan {
            start: Semester {
                season: Season::Winter,
                year: 2025,
            },
            study_sessions: 9,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GMC-1001".to_string(), 3);
        let loaded = load(
            r#"{ "completed": ["GCI-1000"], "sessions": [] }"#,
            &known(&["GCI-1000"]),
        )
        .expect("a cheminement of credits alone loads");
        apply_to_plan(&mut plan, &loaded.application);

        assert_eq!(plan.start.to_string(), "H25");
        assert_eq!(plan.study_sessions, 9);
        assert!(plan.credited.contains("GCI-1000"));
        // the load still replaces the document rather than merging into it
        assert!(plan.displayed_placement.is_empty());
    }

    // --- apply_to_plan -----------------------------------------------------

    #[test]
    fn applying_replaces_the_organigramme_rather_than_merging() {
        let mut plan = Plan {
            study_sessions: 8,
            concomitant: true,
            preparatory_done: false,
            program: Some(ProgramChoice {
                code: "B-GEX".to_string(),
                semester: "A26".to_string(),
                concentration: None,
                profile: None,
            }),
            ..Plan::default()
        };
        plan.prereq_overrides.insert(
            "GCI-2010".to_string(),
            ulaval_scheduler_core::PrereqOverride {
                text: "GCI-1000".to_string(),
                official: None,
            },
        );
        plan.displayed_placement.insert("GCI-1000".to_string(), 4);
        plan.pinned_sessions.insert("GCI-1000".to_string(), 4);
        plan.manual.insert(2, vec!["GCI-1003".to_string()]);
        plan.credited.insert("GSC-1000".to_string());
        // the leftovers that used to survive and get re-seated by the
        // automatic placement, on top of the grid just loaded
        plan.electives.push("GCI-2010".to_string());
        plan.elective_origins
            .insert("GCI-2010".to_string(), "c".to_string());
        plan.rule_grants
            .insert("GCI-2010".to_string(), "p/Règle 2".to_string());
        plan.special.insert(3, "à l'étranger".to_string());
        plan.chosen.insert(1, Default::default());
        plan.completed_sessions = 2;
        plan.credit_cap = 21;

        let loaded = load(GRID, &known(&["GMC-1001", "MAT-1900", "GMC-1024"]))
            .expect("the grid loads");
        apply_to_plan(&mut plan, &loaded.application);

        assert_eq!(plan.start.to_string(), "A26");
        assert_eq!(plan.study_sessions, 2);
        assert!(!plan.displayed_placement.contains_key("GCI-1000"));
        assert!(plan.manual.is_empty());
        // nothing naming a course or a session survives the load
        assert!(plan.electives.is_empty(), "{:?}", plan.electives);
        assert!(plan.elective_origins.is_empty());
        assert!(plan.rule_grants.is_empty());
        assert!(plan.special.is_empty());
        assert!(plan.chosen.is_empty());
        assert_eq!(plan.displayed_placement["GMC-1001"], 1);
        assert_eq!(plan.pinned_sessions["MAT-1900"], 2);
        assert!(plan.credited.contains("GMC-1024"));
        assert!(!plan.credited.contains("GSC-1000"));
        // the settings the file says nothing about are not its business
        assert_eq!(plan.credit_cap, 21);
        assert!(plan.concomitant);
        assert!(!plan.preparatory_done);
        assert!(plan.prereq_overrides.contains_key("GCI-2010"));
        assert_eq!(
            plan.program.as_ref().map(|program| program.code.as_str()),
            Some("B-GEX")
        );
        assert_eq!(plan.completed_sessions, 0);
    }

    #[test]
    fn a_grid_with_summer_courses_opens_the_summers_and_never_closes_them() {
        let raw = r#"{ "completed": [], "sessions": [
            { "semester": "A26", "courses": ["GMC-1001"] },
            { "semester": "H27", "courses": [] },
            { "semester": "E27", "courses": ["MAT-1900"] } ] }"#;
        let mut plan = Plan::default();
        let loaded = load(raw, &known(&["GMC-1001", "MAT-1900"]))
            .expect("the grid loads");
        apply_to_plan(&mut plan, &loaded.application);
        assert!(plan.summers_open);

        // an été-less grid leaves a student's own choice standing
        let mut plan = Plan {
            summers_open: true,
            ..Plan::default()
        };
        let loaded = load(GRID, &known(&["GMC-1001", "MAT-1900", "GMC-1024"]))
            .expect("the grid loads");
        apply_to_plan(&mut plan, &loaded.application);
        assert!(plan.summers_open);
    }

    // --- export ------------------------------------------------------------

    fn provenance() -> ExportProvenance {
        crate::export::provenance::export_provenance(
            "2026-08-29T18:32:00Z",
            Some("2026-08-28"),
        )
    }

    #[test]
    fn an_exported_organigramme_reads_back_as_the_same_organigramme() {
        let mut plan = Plan {
            study_sessions: 2,
            ..Plan::default()
        };
        plan.displayed_placement.insert("GMC-1001".to_string(), 1);
        plan.displayed_placement.insert("MAT-1900".to_string(), 2);
        plan.credited.insert("GMC-1024".to_string());

        let raw = export(&plan, "2026-08-29T18:32:00Z", &provenance());
        let reread = load(&raw, &known(&["GMC-1001", "MAT-1900", "GMC-1024"]))
            .expect("an exported cheminement reads back");
        let mut round_tripped = Plan::default();
        apply_to_plan(&mut round_tripped, &reread.application);

        assert_eq!(round_tripped.start, plan.start);
        assert_eq!(round_tripped.study_sessions, plan.study_sessions);
        assert_eq!(
            round_tripped.displayed_placement,
            plan.displayed_placement
        );
        assert_eq!(round_tripped.credited, plan.credited);
        assert!(reread.summary.ignored.is_empty());
    }

    #[test]
    fn the_exported_file_carries_its_provenance_and_the_whole_timeline() {
        let plan = Plan {
            study_sessions: 2,
            program: Some(ProgramChoice {
                code: "B-GMC".to_string(),
                semester: "A26".to_string(),
                concentration: Some(
                    "Sciences de la nature - Profil international".to_string(),
                ),
                profile: None,
            }),
            ..Plan::default()
        };
        let raw = export(&plan, "2026-08-29T18:32:00Z", &provenance());
        let value: serde_json::Value =
            serde_json::from_str(&raw).expect("the export is JSON");
        assert_eq!(value["provenance"]["program"], "B-GMC");
        assert_eq!(value["provenance"]["semester"], "A26");
        assert_eq!(value["provenance"]["exported_at"], "2026-08-29T18:32:00Z");
        assert_eq!(value["provenance"]["repo"], provenance().repo);
        // two study sessions carry one été between them: the empty slot is
        // written, or the timeline would not come back the same
        let sessions = value["sessions"]
            .as_array()
            .expect("the sessions are an array");
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[2]["semester"], "E27");
        assert!(sessions[2]["courses"]
            .as_array()
            .is_some_and(|courses| courses.is_empty()));
    }

    #[test]
    fn a_plan_with_no_program_still_exports_under_a_plain_name() {
        let plan = Plan::default();
        assert_eq!(export_file_name(&plan), "cheminement.json");
        let raw = export(&plan, "2026-08-29T18:32:00Z", &provenance());
        let value: serde_json::Value =
            serde_json::from_str(&raw).expect("the export is JSON");
        assert!(value["provenance"].get("program").is_none());
        assert!(value["provenance"].get("concentration").is_none());
    }

    #[test]
    fn the_file_name_matches_the_repository_convention() {
        let named = |concentration: Option<&str>| {
            export_file_name(&Plan {
                program: Some(ProgramChoice {
                    code: "B-GMC".to_string(),
                    semester: "A26".to_string(),
                    concentration: concentration.map(str::to_string),
                    profile: None,
                }),
                ..Plan::default()
            })
        };
        assert_eq!(named(None), "B-GMC-A26.json");
        assert_eq!(
            named(Some(
                "Technique de génie mécanique - Scolarité \
                        préparatoire non-complétée"
            )),
            "B-GMC-A26-technique_de_genie_mecanique_scolarite_preparatoire_\
             non_completee.json"
        );
        // a label that folds to nothing leaves the plain name standing
        assert_eq!(named(Some("—")), "B-GMC-A26.json");
    }

    // the fold table is hand-written (no Unicode crate for eight lines):
    // every arm of it has to be exercised, or a French label would reach a
    // file name with an accent still in it
    #[test]
    fn every_accent_folds_to_its_bare_letter() {
        assert_eq!(
            snake_case("À côté d'un Îlot, où Ça mûrit"),
            "a_cote_d_un_ilot_ou_ca_murit"
        );
        assert_eq!(snake_case("äëïöü"), "aeiou");
    }
}
