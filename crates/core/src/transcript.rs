use std::collections::{BTreeMap, BTreeSet};

use crate::{
    horizon_sessions, session_semesters, CourseCycle, Season, Semester,
};

// A relevé Capsule is read from three sections, each with its own semantics
// for the plan the student builds from it (ADR
// `2026-08-import-de-releve-capsule`): a course under `Laval` was actually
// taken at Université Laval and pins its real session; one under `Recognized`
// was credited from another institution and never pins a session; one under
// `InProgress` is a current or future registration, still pinned to its
// session.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptSection {
    Recognized,
    Laval,
    InProgress,
}

// `grade` is `None` for a « CRÉDITS EN COURS » row: that section has no note
// column, since the session is not over yet.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptCourse {
    pub code: String,
    pub cycle: CourseCycle,
    pub title: String,
    pub grade: Option<String>,
    pub credits: i64,
}

// `institution` is only ever `Some` under « RECONNAISSANCE DES ACQUIS »
// (« Université de Montréal »): the other two sections are Université Laval
// itself and never name it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TranscriptSession {
    pub section: TranscriptSection,
    pub semester: Semester,
    pub institution: Option<String>,
    pub courses: Vec<TranscriptCourse>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Transcript {
    // document order: the same order the student took the sessions in
    pub sessions: Vec<TranscriptSession>,
}

// D et mieux, plus P (« Passed » : a language requirement or a stage graded
// pass/fail) — anything below is a real échec, not a rounding call (ADR
// `2026-08-import-de-releve-capsule`).
pub const PASSING_GRADES: [&str; 12] = [
    "A+", "A", "A-", "B+", "B", "B-", "C+", "C", "C-", "D+", "D", "P",
];

// Everything a relevé demands of the plan it is applied to: where the
// horizon starts, how far it must reach, which sessions stay open to
// regular courses, and the fate of every course on the relevé — pinned to
// its real session, credited without one, or ignored with a named reason.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct TranscriptApplication {
    pub start: Semester,
    pub study_sessions: usize,
    // 1-based index of the latest *graded* Laval session within the
    // horizon — every session up to it is déjà complétée and closed to
    // the solver for unpinned courses; the « CRÉDITS EN COURS » sessions
    // stay open, the student can still adjust them (ADR
    // `2026-08-sessions-completees-fermees-au-solveur`). 0 = none.
    pub completed_sessions: usize,
    // the heaviest pinned load of any one relevé session, in credits: the
    // student demonstrably carried it, so the plan's credit cap must be at
    // least this — a lower cap would make his own past infeasible (ADR
    // `2026-08-plancher-du-plafond-de-credits-depuis-le-releve`)
    pub max_session_credits: u32,
    pub summers_open: bool,
    pub pinned: BTreeMap<String, usize>,
    pub credited: BTreeSet<String>,
    pub ignored: Vec<IgnoredCourse>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct IgnoredCourse {
    pub code: String,
    pub reason: IgnoredReason,
}

#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum IgnoredReason {
    // an échec at Laval — the note itself, kept for display (`E`, `F`, …)
    Failed(String),
    // its session falls outside the horizon: either an été strictly before
    // `start` (an été never opens the horizon) or a session the 32-session
    // clamp still could not reach
    OutsideHorizon,
    // a « RECONNAISSANCE DES ACQUIS » row whose note is not `V`
    UnexpectedGrade(String),
    // a sigle today's répertoire no longer carries (a retired course, an
    // older curriculum's code): the snapshot owns every `Course` the app
    // can reason about, so the row is surfaced here rather than poisoning
    // the plan with a code no solver request could ever resolve
    NotInCatalogue,
}

// the widest horizon the search below ever tries: a relevé spanning more
// than 8 years can legitimately make `apply_transcript` return a
// `study_sessions` this high, so a long transcript still gets every course
// pinned instead of silently losing the older ones to `OutsideHorizon`.
// `pub` so the UI's own « Sessions » number input (`components/panel.rs`)
// shares this exact ceiling instead of a separately-maintained magic
// number — the decision left open above, resolved by widening the input
// (`ui/src/capsule.rs::apply_to_plan` then needs no clamp of its own: every
// `study_sessions` this module can ever return already fits).
pub const MAX_STUDY_SESSIONS: usize = 32;

// Turns a parsed relevé into the plan changes it demands (ADR
// `2026-08-import-de-releve-capsule`): `None` when the relevé holds no
// ULaval session at all — nothing to anchor a plan to. Every course of the
// relevé lands in exactly one of `pinned`, `credited` or `ignored`; nothing
// is ever dropped silently. `known` is the catalogue's sigles: a code
// outside it is ignored (`NotInCatalogue`) instead of pinned or credited —
// the same gate `validate_new_code` holds at every other door (ADR
// `2026-08-sigles-inconnus-du-releve-ignores`). The sessions themselves
// still anchor `start` and grow the horizon: the student attended them,
// whatever today's répertoire still carries.
pub fn apply_transcript(
    transcript: &Transcript,
    minimum_study_sessions: usize,
    program_floor: Option<Semester>,
    known: &BTreeSet<String>,
) -> Option<TranscriptApplication> {
    let start = earliest_start(transcript, program_floor)?;
    let latest_key = latest_laval_key(transcript)?;

    let (study_sessions, seasons) =
        grow_horizon(start, minimum_study_sessions, latest_key);
    let semesters = session_semesters(start, &seasons);

    let summers_open = transcript.sessions.iter().any(|session| {
        is_laval_session(session.section)
            && session.semester.season == Season::Summer
            && semesters.contains(&session.semester)
    });

    let (pinned, credited, ignored) =
        resolve_courses(transcript, &semesters, known);

    // graded (`Laval`) sessions are over; the latest one closes every
    // session up to itself — `InProgress` sessions never do
    let completed_sessions = transcript
        .sessions
        .iter()
        .filter(|session| session.section == TranscriptSection::Laval)
        .filter_map(|session| {
            semesters
                .iter()
                .position(|semester| *semester == session.semester)
                .map(|position| position + 1)
        })
        .max()
        .unwrap_or(0);

    let max_session_credits =
        heaviest_session(transcript, &semesters, &pinned);

    Some(TranscriptApplication {
        start,
        study_sessions,
        completed_sessions,
        max_session_credits,
        summers_open,
        pinned,
        credited,
        ignored,
    })
}

// the heaviest per-session load actually pinned — only courses whose final
// fate is « pinned at this very session » count, so a failed row or a
// retake's earlier attempt never inflates the load
fn heaviest_session(
    transcript: &Transcript,
    semesters: &[Semester],
    pinned: &BTreeMap<String, usize>,
) -> u32 {
    let mut heaviest: i64 = 0;
    for session in &transcript.sessions {
        let Some(index) = semesters
            .iter()
            .position(|semester| *semester == session.semester)
            .map(|position| position + 1)
        else {
            continue;
        };
        let load: i64 = session
            .courses
            .iter()
            .filter(|course| pinned.get(&course.code) == Some(&index))
            .map(|course| course.credits.max(0))
            .sum();
        heaviest = heaviest.max(load);
    }
    u32::try_from(heaviest).unwrap_or(u32::MAX)
}

fn is_laval_session(section: TranscriptSection) -> bool {
    matches!(
        section,
        TranscriptSection::Laval | TranscriptSection::InProgress
    )
}

// (year, within-year rank): `Semester::year` already lands on the actual
// civil year of every season (the arithmetic `session_semesters` itself
// relies on), but the three seasons of one year still need ordering — hiver,
// puis été, puis automne — since `Season`'s derived `Ord` is declaration
// order, not calendar time.
fn semester_key(semester: Semester) -> (u16, u8) {
    let rank = match semester.season {
        Season::Winter => 0,
        Season::Summer => 1,
        Season::Fall => 2,
    };
    (semester.year, rank)
}

// the earliest Fall/Winter session actually attended (or in progress) at
// Laval — an été is filtered out here regardless of how early it sits, so it
// can never become `start` (`horizon_sessions` only ever opens on an A or an
// H). `program_floor` — the earliest « Fréquentation » start among the
// currently-pursued program(s) on « PROGRAMME(S) FRÉQUENTÉ(S) »
// (`parser::transcript::parse_program_floor`) — excludes any session from an
// older, unrelated program (a finished certificat or an earlier bac): every
// ULaval credit the student ever earned shares this one flat section, with
// no per-course program tag to sort by, so a date floor is the only signal
// available (ADR `2026-08-import-de-releve-capsule`).
fn earliest_start(
    transcript: &Transcript,
    program_floor: Option<Semester>,
) -> Option<Semester> {
    transcript
        .sessions
        .iter()
        .filter(|session| {
            is_laval_session(session.section)
                && session.semester.season != Season::Summer
                && program_floor.is_none_or(|floor| {
                    semester_key(session.semester) >= semester_key(floor)
                })
        })
        .map(|session| session.semester)
        .min_by_key(|semester| semester_key(*semester))
}

// the latest Laval/InProgress semester on the relevé — the target the
// horizon must be grown to reach. `None` only when there is no Laval/
// InProgress session at all, which the sole caller already rules out by
// running this after `earliest_start` returned `Some` (ADR
// `2026-07-expect-en-production`: an error channel exists at no cost here —
// `?` — so no `expect` is needed to reach the "impossible" case).
fn latest_laval_key(transcript: &Transcript) -> Option<(u16, u8)> {
    transcript
        .sessions
        .iter()
        .filter(|session| is_laval_session(session.section))
        .map(|session| session.semester)
        .max_by_key(|semester| semester_key(*semester))
        .map(semester_key)
}

// the smallest horizon, from `minimum_study_sessions` up to the 32-session
// clamp, whose semesters reach `latest`; `horizon_sessions` grows
// monotonically with `n`, so the first hit is the smallest — a bounded loop,
// never a `while`. When even 32 does not reach, the clamp is kept and the
// caller reports the still-uncovered courses as `OutsideHorizon`.
fn grow_horizon(
    start: Semester,
    minimum_study_sessions: usize,
    latest_key: (u16, u8),
) -> (usize, Vec<Season>) {
    let minimum = minimum_study_sessions.max(2);
    for n in minimum..=MAX_STUDY_SESSIONS {
        let seasons = horizon_sessions(start.season, n);
        let reaches = session_semesters(start, &seasons)
            .iter()
            .any(|semester| semester_key(*semester) >= latest_key);
        if reaches {
            return (n, seasons);
        }
    }
    (
        MAX_STUDY_SESSIONS,
        horizon_sessions(start.season, MAX_STUDY_SESSIONS),
    )
}

enum CourseOutcome {
    Pinned(usize),
    Credited,
    Ignored(IgnoredReason),
}

// the chronological walk that resolves every row of the relevé: sessions
// are read in `Transcript::sessions`'s own document order (the order the
// student took them), and a later row for the same code overwrites an
// earlier one — a retake settles wherever it finally landed, never counted
// twice
fn resolve_courses(
    transcript: &Transcript,
    semesters: &[Semester],
    known: &BTreeSet<String>,
) -> (
    BTreeMap<String, usize>,
    BTreeSet<String>,
    Vec<IgnoredCourse>,
) {
    let mut outcomes: BTreeMap<String, CourseOutcome> = BTreeMap::new();
    for session in &transcript.sessions {
        let index = semesters
            .iter()
            .position(|semester| *semester == session.semester)
            .map(|position| position + 1);
        for course in &session.courses {
            // the catalogue gate comes before any section semantics: a
            // retired sigle is unresolvable whatever its grade says
            let outcome = if known.contains(&course.code) {
                course_outcome(session.section, course, index)
            } else {
                CourseOutcome::Ignored(IgnoredReason::NotInCatalogue)
            };
            outcomes.insert(course.code.clone(), outcome);
        }
    }
    let mut pinned = BTreeMap::new();
    let mut credited = BTreeSet::new();
    let mut ignored = Vec::new();
    for (code, outcome) in outcomes {
        match outcome {
            CourseOutcome::Pinned(index) => {
                pinned.insert(code, index);
            }
            CourseOutcome::Credited => {
                credited.insert(code);
            }
            CourseOutcome::Ignored(reason) => {
                ignored.push(IgnoredCourse { code, reason });
            }
        }
    }
    (pinned, credited, ignored)
}

// one row's fate, from its section and its (possibly absent) grade — the
// three sections read completely differently
fn course_outcome(
    section: TranscriptSection,
    course: &TranscriptCourse,
    index: Option<usize>,
) -> CourseOutcome {
    let grade = course.grade.as_deref().unwrap_or("");
    match section {
        TranscriptSection::Recognized => {
            if grade == "V" {
                CourseOutcome::Credited
            } else {
                CourseOutcome::Ignored(IgnoredReason::UnexpectedGrade(
                    grade.to_string(),
                ))
            }
        }
        TranscriptSection::InProgress => pin_or_outside(index),
        TranscriptSection::Laval => {
            if PASSING_GRADES.contains(&grade) {
                pin_or_outside(index)
            } else {
                CourseOutcome::Ignored(IgnoredReason::Failed(
                    grade.to_string(),
                ))
            }
        }
    }
}

fn pin_or_outside(index: Option<usize>) -> CourseOutcome {
    match index {
        Some(index) => CourseOutcome::Pinned(index),
        None => CourseOutcome::Ignored(IgnoredReason::OutsideHorizon),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn semester(raw: &str) -> Semester {
        raw.parse().unwrap_or_else(|e| panic!("{raw}: {e}"))
    }

    fn course(code: &str, grade: Option<&str>) -> TranscriptCourse {
        TranscriptCourse {
            code: code.to_string(),
            cycle: CourseCycle::First,
            title: "Titre".to_string(),
            grade: grade.map(str::to_string),
            credits: 3,
        }
    }

    fn session(
        section: TranscriptSection,
        raw_semester: &str,
        courses: Vec<TranscriptCourse>,
    ) -> TranscriptSession {
        TranscriptSession {
            section,
            semester: semester(raw_semester),
            institution: None,
            courses,
        }
    }

    // the catalogue most tests want: every sigle of the transcript itself
    fn all_codes(transcript: &Transcript) -> BTreeSet<String> {
        transcript
            .sessions
            .iter()
            .flat_map(|session| session.courses.iter())
            .map(|course| course.code.clone())
            .collect()
    }

    #[test]
    fn a_transcript_of_recognized_credits_alone_anchors_nothing() {
        // reconnaissance only: no Laval session to hang a calendar on
        let transcript = Transcript {
            sessions: vec![session(
                TranscriptSection::Recognized,
                "H13",
                vec![course("MAT-1910", Some("V"))],
            )],
        };
        assert_eq!(
            apply_transcript(&transcript, 2, None, &all_codes(&transcript)),
            None
        );
    }

    #[test]
    fn completed_sessions_end_at_the_latest_graded_session_never_en_cours() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "H25",
                    vec![course("GEX-1001", Some("B"))],
                ),
                session(
                    TranscriptSection::InProgress,
                    "A25",
                    vec![course("GEX-1002", None)],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("a Laval session anchors the plan"));

        // horizon from A24 : 1=A24, 2=H25, 3=É25, 4=A25
        assert_eq!(application.pinned["GEX-1002"], 4);
        assert_eq!(
            application.completed_sessions, 2,
            "H25 est la dernière session notée; l'A25 en cours reste ouverte"
        );
    }

    #[test]
    fn the_heaviest_session_counts_only_finally_pinned_rows() {
        // A24 : one passed row (3 cr) plus a failed one that must not
        // count; H25 : two passed rows — the heaviest load is H25's 6
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![
                        course("GEX-1000", Some("A")),
                        course("GEX-1001", Some("E")),
                    ],
                ),
                session(
                    TranscriptSection::Laval,
                    "H25",
                    vec![
                        course("GEX-1002", Some("B")),
                        course("GEX-1003", Some("C+")),
                    ],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("a Laval session anchors the plan"));
        assert_eq!(
            application.max_session_credits, 6,
            "l'échec de l'A24 ne compte pas; l'H25 porte 6 crédits épinglés"
        );
    }

    #[test]
    fn a_code_outside_the_catalogue_is_ignored_whatever_its_section_says() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![
                        course("GEX-1000", Some("A")),
                        course("ECN-2901", Some("A+")),
                    ],
                ),
                session(
                    TranscriptSection::Recognized,
                    "H13",
                    vec![course("MAT-0000", Some("V"))],
                ),
            ],
        };
        let known: BTreeSet<String> = BTreeSet::from(["GEX-1000".to_string()]);
        let application = apply_transcript(&transcript, 2, None, &known)
            .unwrap_or_else(|| panic!("a Laval session anchors the plan"));

        assert_eq!(application.pinned["GEX-1000"], 1);
        assert!(
            !application.pinned.contains_key("ECN-2901"),
            "a passing grade does not resurrect a retired sigle"
        );
        assert!(
            !application.credited.contains("MAT-0000"),
            "the gate holds for RECONNAISSANCE rows too"
        );
        let reasons: Vec<&IgnoredCourse> =
            application.ignored.iter().collect();
        assert_eq!(reasons.len(), 2);
        assert!(reasons
            .iter()
            .all(|entry| entry.reason == IgnoredReason::NotInCatalogue));
    }

    #[test]
    fn start_is_the_earliest_fall_or_winter_laval_session() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "H25",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1001", Some("B"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .expect("a Laval session anchors a start");
        assert_eq!(application.start, semester("A24"), "the earlier session");
    }

    #[test]
    fn a_program_floor_excludes_an_earlier_unrelated_programs_sessions() {
        // a certificat finished years before the current bac: without
        // `program_floor` this A18 session would wrongly become `start`
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A18",
                    vec![course("CER-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
            ],
        };
        let floor = semester("A24");
        let application = apply_transcript(
            &transcript,
            2,
            Some(floor),
            &all_codes(&transcript),
        )
        .unwrap_or_else(|| panic!("A24 still anchors a start"));

        assert_eq!(
            application.start, floor,
            "the A18 certificat session never anchors start"
        );
        assert_eq!(
            application.ignored,
            [IgnoredCourse {
                code: "CER-1000".to_string(),
                reason: IgnoredReason::OutsideHorizon,
            }],
            "the older program's course is still reported, never dropped"
        );
    }

    #[test]
    fn a_transcript_with_only_summer_sessions_has_no_start() {
        let transcript = Transcript {
            sessions: vec![session(
                TranscriptSection::Laval,
                "E24",
                vec![course("GEX-1000", Some("A"))],
            )],
        };
        assert!(
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .is_none(),
            "no A/H session, nothing to anchor a plan to"
        );
    }

    #[test]
    fn an_ete_before_the_first_fall_or_winter_never_becomes_start() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "E24",
                    vec![course("GEX-0100", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(application.start, semester("A24"));
        assert_eq!(
            application.ignored,
            [IgnoredCourse {
                code: "GEX-0100".to_string(),
                reason: IgnoredReason::OutsideHorizon,
            }],
            "the leading été sits before start, out of the horizon"
        );
    }

    #[test]
    fn the_horizon_grows_past_the_requested_minimum_to_reach_the_latest() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "A25",
                    vec![course("GEX-2000", Some("B"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(
            application.study_sessions, 3,
            "2 study sessions (through É25) do not reach A25"
        );
        assert_eq!(application.pinned["GEX-2000"], 4, "A24 H25 É25 A25");
    }

    #[test]
    fn a_session_the_32_session_clamp_cannot_reach_is_outside_horizon() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::InProgress,
                    "A99",
                    vec![course("GEX-9999", None)],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(application.study_sessions, MAX_STUDY_SESSIONS);
        assert_eq!(
            application.ignored,
            [IgnoredCourse {
                code: "GEX-9999".to_string(),
                reason: IgnoredReason::OutsideHorizon,
            }],
            "A99 sits well past what 32 study sessions can reach"
        );
    }

    #[test]
    fn an_ete_on_the_releve_opens_the_summer_session() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "E25",
                    vec![course("GEX-2500", Some("B"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert!(application.summers_open, "É25 lands inside the horizon");
        assert_eq!(application.pinned["GEX-2500"], 3, "A24 H25 É25");
    }

    #[test]
    fn a_d_passes_and_an_e_fails() {
        let transcript = Transcript {
            sessions: vec![session(
                TranscriptSection::Laval,
                "A24",
                vec![
                    course("GEX-1000", Some("D")),
                    course("GEX-1001", Some("E")),
                ],
            )],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(application.pinned["GEX-1000"], 1, "D passes");
        assert_eq!(
            application.ignored,
            [IgnoredCourse {
                code: "GEX-1001".to_string(),
                reason: IgnoredReason::Failed("E".to_string()),
            }],
            "E fails, the note is kept"
        );
    }

    #[test]
    fn a_fail_then_pass_retake_is_pinned_once() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("E"))],
                ),
                session(
                    TranscriptSection::Laval,
                    "H25",
                    vec![course("GEX-1000", Some("B"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(application.pinned["GEX-1000"], 2, "pinned at the retake");
        assert!(
            application.ignored.is_empty(),
            "the earlier échec is superseded, not also reported"
        );
    }

    #[test]
    fn a_v_grade_is_credited_without_a_session() {
        let mut recognized = session(
            TranscriptSection::Recognized,
            "A24",
            vec![course("MAT-1910", Some("V"))],
        );
        recognized.institution = Some("Université de Montréal".to_string());
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                recognized,
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert!(application.credited.contains("MAT-1910"));
        assert!(
            !application.pinned.contains_key("MAT-1910"),
            "a credited course never pins a session"
        );
    }

    #[test]
    fn a_recognized_row_with_a_letter_grade_is_an_unexpected_grade() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::Recognized,
                    "A24",
                    vec![course("MAT-1910", Some("B"))],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(
            application.ignored,
            [IgnoredCourse {
                code: "MAT-1910".to_string(),
                reason: IgnoredReason::UnexpectedGrade("B".to_string()),
            }]
        );
    }

    #[test]
    fn an_empty_transcript_has_nothing_to_apply() {
        let transcript = Transcript { sessions: vec![] };
        assert!(apply_transcript(
            &transcript,
            2,
            None,
            &all_codes(&transcript)
        )
        .is_none());
    }

    #[test]
    fn an_in_progress_course_is_pinned_with_no_grade_at_all() {
        let transcript = Transcript {
            sessions: vec![
                session(
                    TranscriptSection::Laval,
                    "A24",
                    vec![course("GEX-1000", Some("A"))],
                ),
                session(
                    TranscriptSection::InProgress,
                    "H25",
                    vec![course("GEX-2000", None)],
                ),
            ],
        };
        let application =
            apply_transcript(&transcript, 2, None, &all_codes(&transcript))
                .unwrap_or_else(|| panic!("A24 anchors a start"));
        assert_eq!(application.pinned["GEX-2000"], 2, "pinned at H25");
    }
}
