use std::collections::{BTreeMap, BTreeSet};

use crate::course::{Course, Season};
use crate::preparatory::PREPARATORY_RULE_TITLE;
use crate::program::{Program, RuleCourses, STAGES_RULE_TITLE};
use crate::weekly::resolve_offering;

// The intake seam of every consumer of the solvers (the UI, any future
// harness): turn the student's typed input and the snapshot into
// solver-ready values. Pure — no IO — so it lives in core (invariant:
// business logic never in the view); the equivalence resolution in
// particular is domain logic, not glue (ADRs
// `2026-07-aides-dintake-extraites-dans-core`,
// `2026-07-retrait-des-harnais-cli-et-ui-debug`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntakeError {
    #[error(
        "unknown session {session:?}: expected a<year>, h<year> or e<year> \
         (e.g. a2026)"
    )]
    UnknownSession { session: String },
    #[error("duplicated course codes : {}", .codes.join(", "))]
    DuplicatedCodes { codes: Vec<String> },
    #[error("unknown course codes : {}", .codes.join(", "))]
    UnknownCodes { codes: Vec<String> },
    #[error("{code} is not offered in the requested season")]
    NotOffered { code: String },
    #[error("pinned expects CODE=SESSION : {spec}")]
    MalformedPin { spec: String },
    #[error("{code} : {reason}")]
    UnresolvedCredits { code: String, reason: String },
}

// solver-A-ready input: the season and the requested courses, offerings
// already resolved against their equivalents
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleIntake {
    pub season: Season,
    pub courses: Vec<Course>,
}

// everything `place` and `coverage_report` need from the student's typed
// input; the caller adds the sessions, the budgets and the concomitance
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementIntake {
    pub courses: Vec<Course>,
    pub passed: BTreeSet<String>,
    pub pinned: BTreeMap<String, usize>,
    // the coverage selection is the whole list, passed courses included —
    // a passed mandatory course still counts toward the rules
    pub selection: BTreeSet<String>,
    // program-derived codes with no snapshot data, to surface loudly (ADR
    // `2026-07-cours-sans-offre-ecarte-par-le-harnais`)
    pub set_aside: Vec<String>,
    // the selected courses that belong to the program's « Stages » rule —
    // the auto-included mandatory stage plus any optional stage typed as
    // an elective; the caller hands them to `PlacementRequest.stages` so
    // they land in an été unless pinned (ADR
    // `2026-08-stage-place-en-ete-sauf-epinglage`)
    pub stages: BTreeSet<String>,
}

// The weekly pipeline shared by every harness: session parsed, codes
// normalized, courses selected with their equivalents resolved. The UI
// then calls `schedule_report` with its pin map.
pub fn schedule_intake(
    all: &[Course],
    session: &str,
    codes: &[String],
) -> Result<ScheduleIntake, IntakeError> {
    let (season, _) = parse_session(session)?;
    let codes = normalize_codes(codes)?;
    let courses = select_courses(all, &codes, season)?;
    Ok(ScheduleIntake { season, courses })
}

// The placement pipeline shared by every harness: typed input (electives,
// passed, pins) strictly validated — a typo must not survive — while
// program-derived courses without snapshot data degrade loudly into
// `set_aside`, never silently dropped.
pub fn placement_intake(
    program: Option<&Program>,
    electives: &[String],
    passed: &[String],
    pins: &[String],
    all: &[Course],
) -> Result<PlacementIntake, IntakeError> {
    let electives = normalize_codes(electives)?;
    let passed_codes = normalize_codes(passed)?;
    let list = course_list(program, &electives, &passed_codes);
    let explicit: BTreeSet<&str> = electives
        .iter()
        .chain(&passed_codes)
        .map(String::as_str)
        .collect();
    let (courses, set_aside) = select_known(&list, all, &explicit)?;
    let pinned = parse_pins(pins)?;
    // intersected with the *selected* courses, not the whole list: a stage
    // set aside for lack of snapshot data must not reach the solver, which
    // requires a Course for every declared stage
    let stage_codes: BTreeSet<&str> =
        listed_rule_courses(program, STAGES_RULE_TITLE)
            .iter()
            .map(String::as_str)
            .collect();
    let stages = courses
        .iter()
        .map(|course| course.code.clone())
        .filter(|code| stage_codes.contains(code.as_str()))
        .collect();
    Ok(PlacementIntake {
        courses,
        passed: passed_codes.into_iter().collect(),
        pinned,
        selection: list.into_iter().collect(),
        set_aside,
        stages,
    })
}

// `a2026` → (Fall, 2026); a = automne, h = hiver, e = été. Only the season
// selects data — the snapshot keeps one offering per season — but the year
// is still validated: a malformed session is a typo to surface.
pub fn parse_session(session: &str) -> Result<(Season, u16), IntakeError> {
    let unknown = || IntakeError::UnknownSession {
        session: session.to_string(),
    };
    let mut letters = session.chars();
    let season = match letters.next() {
        Some('a') => Season::Fall,
        Some('h') => Season::Winter,
        Some('e') => Season::Summer,
        _ => return Err(unknown()),
    };
    let year = letters.as_str().parse().map_err(|_| unknown())?;
    Ok((season, year))
}

// A/H study sessions alternating from `start` — a summer start counts as a
// study session and flows into fall — with an été inserted after each
// hiver, the last included, so a stage always finds an été to land in.
// `study_sessions` counts only the alternation; the inserted étés come on
// top and stay closed to regular courses unless the caller opens them (ADR
// `2026-08-horizon-avec-ete-apres-chaque-hiver`)
pub fn horizon_sessions(start: Season, study_sessions: usize) -> Vec<Season> {
    (0..study_sessions)
        .scan(start, |season, _| {
            let current = *season;
            *season = match current {
                Season::Fall => Season::Winter,
                Season::Winter | Season::Summer => Season::Fall,
            };
            Some(current)
        })
        .flat_map(|season| {
            if season == Season::Winter {
                vec![Season::Winter, Season::Summer]
            } else {
                vec![season]
            }
        })
        .collect()
}

// 1-based indices of the Summer sessions — the shape `open_summers` and
// `pinned` speak
pub fn summer_indices(sessions: &[Season]) -> BTreeSet<usize> {
    sessions
        .iter()
        .enumerate()
        .filter(|&(_, &season)| season == Season::Summer)
        .map(|(index, _)| index + 1)
        .collect()
}

// codes are uppercased for the student's comfort; a duplicated code is a
// typo to surface, not a course to schedule twice
pub fn normalize_codes(codes: &[String]) -> Result<Vec<String>, IntakeError> {
    let codes: Vec<String> =
        codes.iter().map(|code| code.to_uppercase()).collect();
    let mut seen = BTreeSet::new();
    let duplicated: Vec<String> = codes
        .iter()
        .filter(|code| !seen.insert(code.as_str()))
        .cloned()
        .collect();
    if duplicated.is_empty() {
        Ok(codes)
    } else {
        Err(IntakeError::DuplicatedCodes { codes: duplicated })
    }
}

pub fn parse_pins(
    specs: &[String],
) -> Result<BTreeMap<String, usize>, IntakeError> {
    specs
        .iter()
        .map(|spec| {
            let malformed = || IntakeError::MalformedPin {
                spec: spec.to_string(),
            };
            let (code, session) =
                spec.split_once('=').ok_or_else(malformed)?;
            let session = session.parse().map_err(|_| malformed())?;
            Ok((code.to_uppercase(), session))
        })
        .collect()
}

// The préparatoire courses first — they gate the rest, and the passed set
// naturally excludes the ones already done — then the program's mandatory
// courses (reference order), the mandatory stage (the « Stages » rule
// lists it first, ADR `2026-08-stage-obligatoire-en-prose-promu-en-regle`),
// the chosen electives and the passed courses — deduplicated, so a passed
// mandatory course appears once and carries its Course object (ADR
// `2026-08-stage-obligatoire-et-scolarite-preparatoire-dans-lintake`).
pub fn course_list(
    program: Option<&Program>,
    electives: &[String],
    passed: &[String],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    listed_rule_courses(program, PREPARATORY_RULE_TITLE)
        .iter()
        .cloned()
        .chain(
            program
                .map(|program| program.mandatory.clone())
                .unwrap_or_default(),
        )
        .chain(
            listed_rule_courses(program, STAGES_RULE_TITLE)
                .first()
                .cloned(),
        )
        .chain(electives.iter().cloned())
        .chain(passed.iter().cloned())
        .filter(|code| seen.insert(code.clone()))
        .collect()
}

// the courses of the program rule bearing `title`, when it is a plain list
fn listed_rule_courses<'a>(
    program: Option<&'a Program>,
    title: &str,
) -> &'a [String] {
    program
        .map(|program| program.rules.as_slice())
        .unwrap_or_default()
        .iter()
        .find(|rule| rule.title == title)
        .and_then(|rule| match &rule.courses {
            RuleCourses::List { courses } => Some(courses.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

// One Course per requested code, cloned whole — the snapshot already
// carries every season an offering exists for, each dated by its
// `last_offered`. A code the snapshot does not carry is an error when
// explicitly typed, and otherwise (program-derived) set aside and returned
// for the caller to surface — never silently dropped either way.
pub fn select_known(
    codes: &[String],
    all: &[Course],
    explicit: &BTreeSet<&str>,
) -> Result<(Vec<Course>, Vec<String>), IntakeError> {
    let by_code: BTreeMap<&str, &Course> = all
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let unknown: Vec<&str> = codes
        .iter()
        .filter(|code| !by_code.contains_key(code.as_str()))
        .map(String::as_str)
        .collect();
    let typos: Vec<String> = unknown
        .iter()
        .filter(|code| explicit.contains(**code))
        .map(|code| code.to_string())
        .collect();
    if !typos.is_empty() {
        return Err(IntakeError::UnknownCodes { codes: typos });
    }
    let set_aside: Vec<String> =
        unknown.iter().map(|code| code.to_string()).collect();
    let courses = codes
        .iter()
        .filter_map(|code| by_code.get(code.as_str()))
        .map(|&course| course.clone())
        .collect();
    Ok((courses, set_aside))
}

// every requested course, its offering already resolved against its
// equivalents — all unknown codes are named in one error, never silently
// dropped
pub fn select_courses(
    all: &[Course],
    codes: &[String],
    season: Season,
) -> Result<Vec<Course>, IntakeError> {
    let by_code: BTreeMap<&str, &Course> = all
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let unknown: Vec<String> = codes
        .iter()
        .filter(|code| !by_code.contains_key(code.as_str()))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(IntakeError::UnknownCodes { codes: unknown });
    }
    codes
        .iter()
        .map(|code| {
            effective_course(by_code[code.as_str()], &by_code, season)
                .ok_or_else(|| IntakeError::NotOffered { code: code.clone() })
        })
        .collect()
}

// The offering actually attended may come from an equivalent — the most
// recent `last_offered` vintage wins, ties to the course (ADR
// `2026-07-equivalences-par-millesime-de-session`, vintage in-data since
// ADR `2026-07-snapshot-unique-des-cours-millesime-par-saison`). The
// requested course keeps its identity: only the offering is borrowed.
fn effective_course(
    course: &Course,
    by_code: &BTreeMap<&str, &Course>,
    season: Season,
) -> Option<Course> {
    let seed = course.seasons.get(&season);
    let offering = course
        .equivalents
        .iter()
        .filter_map(|code| by_code.get(code.as_str()))
        .filter_map(|equivalent| equivalent.seasons.get(&season))
        .fold(seed, |acc, offering| resolve_offering(acc, Some(offering)))?;
    let mut effective = course.clone();
    effective.seasons = std::iter::once((season, offering.clone())).collect();
    Some(effective)
}

// a stage's `Credits::Range` needs the student's chosen weighting, which
// no harness has an input for yet (open question of the plan) — the error
// is surfaced, never defaulted
pub fn credit_total(courses: &[Course]) -> Result<u32, IntakeError> {
    courses.iter().try_fold(0u32, |total, course| {
        course
            .credits
            .resolve(None)
            .map(|credits| total + credits)
            .map_err(|reason| IntakeError::UnresolvedCredits {
                code: course.code.clone(),
                reason,
            })
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // --- parse_session ---

    #[test]
    fn every_season_letter_parses_to_its_season_and_year() {
        for (session, expected) in [
            ("a2026", (Season::Fall, 2026)),
            ("h2027", (Season::Winter, 2027)),
            ("e2026", (Season::Summer, 2026)),
        ] {
            let parsed = parse_session(session)
                .unwrap_or_else(|e| panic!("{session}: {e}"));
            assert_eq!(parsed, expected, "for {session}");
        }
    }

    #[test]
    fn a_session_outside_the_naming_scheme_is_an_error() {
        for session in ["x2026", "2026", "", "a", "a20x6"] {
            let error =
                parse_session(session).expect_err("outside the scheme");
            assert!(
                error.to_string().contains("a<year>"),
                "for {session:?}: {error}"
            );
        }
    }

    // --- horizon_sessions ---

    #[test]
    fn the_horizon_inserts_an_ete_after_each_hiver_the_last_included() {
        assert_eq!(
            horizon_sessions(Season::Fall, 4),
            [
                Season::Fall,
                Season::Winter,
                Season::Summer,
                Season::Fall,
                Season::Winter,
                Season::Summer
            ]
        );
        assert_eq!(
            horizon_sessions(Season::Winter, 3),
            [
                Season::Winter,
                Season::Summer,
                Season::Fall,
                Season::Winter,
                Season::Summer
            ]
        );
    }

    #[test]
    fn a_summer_start_counts_as_a_study_session_and_flows_into_fall() {
        assert_eq!(
            horizon_sessions(Season::Summer, 3),
            [Season::Summer, Season::Fall, Season::Winter, Season::Summer]
        );
    }

    #[test]
    fn summer_indices_name_the_etes_one_based() {
        assert_eq!(
            summer_indices(&horizon_sessions(Season::Fall, 4)),
            BTreeSet::from([3, 6])
        );
        assert_eq!(summer_indices(&[Season::Fall]), BTreeSet::new());
    }

    // --- normalize_codes ---

    #[test]
    fn codes_are_uppercased_for_the_student() {
        let codes = normalize_codes(&["gex-1000".to_string()])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(codes, ["GEX-1000"]);
    }

    #[test]
    fn a_duplicated_code_is_an_error_naming_it() {
        // duplicated only once uppercased — the check runs on what will
        // actually be scheduled
        let error =
            normalize_codes(&["gex-1000".to_string(), "GEX-1000".to_string()])
                .expect_err("a duplicate is a typo");
        assert!(error.to_string().contains("GEX-1000"), "{error}");
    }

    // --- parse_pins ---

    #[test]
    fn pins_parse_and_uppercase_their_codes() {
        let pins =
            parse_pins(&["gci-1007=2".to_string(), "GEX-1002=1".to_string()])
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(pins["GCI-1007"], 2);
        assert_eq!(pins["GEX-1002"], 1);
    }

    #[test]
    fn a_malformed_pin_is_an_error_showing_the_expected_shape() {
        for spec in ["GCI-1007", "GCI-1007=two"] {
            let error =
                parse_pins(&[spec.to_string()]).expect_err("not CODE=SESSION");
            assert!(error.to_string().contains("CODE=SESSION"), "{error}");
        }
    }

    // --- course_list ---

    #[test]
    fn the_course_list_orders_mandatory_electives_then_passed_deduped() {
        let program: Program = serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":["M-1","M-2"],
                "rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let list = course_list(
            Some(&program),
            &["E-1".to_string(), "M-2".to_string()],
            &["P-1".to_string(), "E-1".to_string()],
        );
        assert_eq!(list, ["M-1", "M-2", "E-1", "P-1"]);
    }

    #[test]
    fn the_course_list_puts_preparatory_first_and_adds_the_mandatory_stage() {
        // the préparatoire rule enters whole; only the first Stages sigle
        // (the mandatory stage) does — optional stages stay elective
        let program: Program = serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":["M-1"],
                "rules":[
                  {"title":"Stages",
                   "constraint":{"type":"course","min":1,"max":8},
                   "courses":["S-1","S-2","S-3"],
                   "credits_in_addition":true},
                  {"title":"Scolarité préparatoire",
                   "courses":["Z-0130","Z-0150"]}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let list = course_list(
            Some(&program),
            &["E-1".to_string()],
            &["Z-0130".to_string()],
        );
        assert_eq!(list, ["Z-0130", "Z-0150", "M-1", "S-1", "E-1"]);
    }

    #[test]
    fn a_titled_rule_that_is_not_a_list_contributes_no_courses() {
        // a « Stages » rule whose courses drifted to a keyword shape must
        // not feed the list — non-list shapes are surfaced by the coverage
        // report, never guessed at here
        let program: Program = serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":["M-1"],
                "rules":[
                  {"title":"Stages",
                   "constraint":{"type":"course","min":1,"max":8},
                   "courses":"negotiated",
                   "raw":"convenus avec la direction"}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        assert_eq!(course_list(Some(&program), &[], &[]), ["M-1"]);
    }

    // --- selection and equivalents ---

    fn course(code: &str, season: &str, options: &str) -> Course {
        vintage_course(code, season, "2026", options)
    }

    fn vintage_course(
        code: &str,
        season: &str,
        last_offered: &str,
        options: &str,
    ) -> Course {
        serde_json::from_str(&format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"{season}":{{"last_offered":{last_offered},
                                          "options":{options}}}}}}}"#
        ))
        .unwrap_or_else(|e| panic!("course literal: {e}"))
    }

    fn option_json(nrc: &str, day: &str, start: &str, end: &str) -> String {
        format!(
            r#"[{{"nrc":"{nrc}","section":"A","mode":"in-person",
                  "slots":[{{"day":"{day}","start":"{start}",
                  "end":"{end}"}}]}}]"#
        )
    }

    fn monday(code: &str, nrc: &str) -> Course {
        course(
            code,
            "fall",
            &format!("[{}]", option_json(nrc, "monday", "08:30", "11:20")),
        )
    }

    #[test]
    fn every_unknown_code_is_named_in_one_error() {
        let all = [monday("GEX-1000", "1")];
        let error = select_courses(
            &all,
            &["GEX-1000".to_string(), "A-1".to_string(), "B-2".to_string()],
            Season::Fall,
        )
        .expect_err("two unknown codes");
        let message = error.to_string();
        assert!(message.contains("A-1"), "{message}");
        assert!(message.contains("B-2"), "{message}");
    }

    #[test]
    fn a_course_not_offered_in_the_season_is_an_error() {
        let all = [course(
            "GEX-1000",
            "winter",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        )];
        let error =
            select_courses(&all, &["GEX-1000".to_string()], Season::Fall)
                .expect_err("offered in winter only");
        let message = error.to_string();
        assert!(message.contains("GEX-1000"), "{message}");
        assert!(message.contains("not offered"), "{message}");
    }

    #[test]
    fn a_missing_offering_borrows_the_equivalents() {
        // the requested course keeps its identity, only the offering is
        // borrowed from the equivalent
        let mut wanted = course("GEX-1000", "winter", "[[]]");
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses =
            select_courses(&all, &["GEX-1000".to_string()], Season::Fall)
                .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(courses[0].code, "GEX-1000");
        assert_eq!(selected_nrc(&courses[0]), "7");
    }

    #[test]
    fn a_courses_own_offering_wins_over_its_equivalents() {
        // equal vintages: ties go to the course itself
        let mut wanted = monday("GEX-1000", "1");
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses =
            select_courses(&all, &["GEX-1000".to_string()], Season::Fall)
                .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(selected_nrc(&courses[0]), "1");
    }

    #[test]
    fn an_equivalent_with_a_newer_vintage_wins_the_offering() {
        // the vintage lives in the data now: an equivalent whose season was
        // read from a fresher session shadows the course's own offering
        // (ADR `2026-07-equivalences-par-millesime-de-session`)
        let mut wanted = vintage_course(
            "GEX-1000",
            "fall",
            "2024",
            &format!("[{}]", option_json("1", "monday", "08:30", "11:20")),
        );
        wanted.equivalents = vec!["GCI-1000".to_string()];
        let all = [wanted, monday("GCI-1000", "7")];

        let courses =
            select_courses(&all, &["GEX-1000".to_string()], Season::Fall)
                .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(courses[0].code, "GEX-1000", "identity is kept");
        assert_eq!(selected_nrc(&courses[0]), "7", "the offering is borrowed");
    }

    fn selected_nrc(course: &Course) -> &str {
        course.seasons[&Season::Fall]
            .options
            .as_deref()
            .expect("known schedule")[0][0]
            .nrc
            .as_str()
    }

    // --- select_known ---

    #[test]
    fn a_typed_unknown_code_is_an_error_naming_it() {
        let all = [monday("GEX-1000", "1")];
        let explicit: BTreeSet<&str> = ["ZZZ-9999"].into_iter().collect();
        let error = select_known(
            &["GEX-1000".to_string(), "ZZZ-9999".to_string()],
            &all,
            &explicit,
        )
        .expect_err("a typo must not survive");
        assert!(error.to_string().contains("ZZZ-9999"), "{error}");
    }

    #[test]
    fn a_program_derived_unknown_code_is_set_aside_not_fatal() {
        let all = [monday("GEX-1000", "1")];
        let (courses, set_aside) = select_known(
            &["GEX-1000".to_string(), "GHOST-999".to_string()],
            &all,
            &BTreeSet::new(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].code, "GEX-1000");
        assert_eq!(set_aside, ["GHOST-999"]);
    }

    // --- schedule_intake ---

    #[test]
    fn the_schedule_pipeline_parses_normalizes_and_selects() {
        let all = [monday("GEX-1000", "1")];
        let intake = schedule_intake(&all, "a2026", &["gex-1000".to_string()])
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(intake.season, Season::Fall);
        assert_eq!(intake.courses[0].code, "GEX-1000");
    }

    #[test]
    fn the_schedule_pipeline_propagates_every_intake_error() {
        let all = [monday("GEX-1000", "1")];
        let bad_session =
            schedule_intake(&all, "x2026", &["GEX-1000".to_string()])
                .expect_err("no such season letter");
        assert!(bad_session.to_string().contains("a<year>"), "{bad_session}");
        let duplicated = schedule_intake(
            &all,
            "a2026",
            &["gex-1000".to_string(), "GEX-1000".to_string()],
        )
        .expect_err("a duplicate is a typo");
        assert!(duplicated.to_string().contains("GEX-1000"), "{duplicated}");
        let unknown =
            schedule_intake(&all, "a2026", &["ZZZ-9999".to_string()])
                .expect_err("no such course");
        assert!(unknown.to_string().contains("ZZZ-9999"), "{unknown}");
    }

    // --- placement_intake ---

    #[test]
    fn the_placement_pipeline_orders_validates_and_sets_aside() {
        // GHOST-999 is mandatory but has no snapshot data: set aside
        // loudly; the passed course still lands in the coverage selection
        let program: Program = serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,
                "mandatory":["GEX-1000","GHOST-999"],
                "rules":[],"concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let all = [monday("GEX-1000", "1"), monday("GCI-1000", "2")];

        let intake = placement_intake(
            Some(&program),
            &["gci-1000".to_string()],
            &["gex-1000".to_string()],
            &["gci-1000=1".to_string()],
            &all,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        let codes: Vec<&str> = intake
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["GEX-1000", "GCI-1000"]);
        assert_eq!(intake.set_aside, ["GHOST-999"]);
        assert!(intake.passed.contains("GEX-1000"));
        assert_eq!(intake.pinned["GCI-1000"], 1);
        assert!(intake.selection.contains("GHOST-999"), "whole list");
        assert!(intake.selection.contains("GEX-1000"), "passed included");
        assert!(intake.stages.is_empty(), "no Stages rule, no stages");
    }

    #[test]
    fn the_placement_pipeline_flags_selected_stages_only() {
        // GEX-1580 (mandatory stage) has no snapshot data: set aside, so it
        // must not reach the solver's stages either; the optional stage
        // typed as an elective is selected and flagged
        let program: Program = serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":[],
                "rules":[
                  {"title":"Stages",
                   "constraint":{"type":"course","min":1,"max":8},
                   "courses":["GEX-1580","GEX-2590"],
                   "credits_in_addition":true}],
                "concentrations":[],"profiles":[]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"));
        let all = [monday("GEX-2590", "1")];

        let intake = placement_intake(
            Some(&program),
            &["gex-2590".to_string()],
            &[],
            &[],
            &all,
        )
        .unwrap_or_else(|e| panic!("{e}"));

        assert_eq!(intake.set_aside, ["GEX-1580"]);
        assert_eq!(
            intake.stages,
            BTreeSet::from(["GEX-2590".to_string()]),
            "only stages with a Course reach the solver"
        );
    }

    #[test]
    fn the_placement_pipeline_propagates_every_intake_error() {
        let all = [monday("GEX-1000", "1")];
        let none: &[String] = &[];
        let duplicated_electives = placement_intake(
            None,
            &["gex-1000".to_string(), "GEX-1000".to_string()],
            none,
            none,
            &all,
        )
        .expect_err("the same elective twice");
        assert!(
            duplicated_electives.to_string().contains("duplicated"),
            "{duplicated_electives}"
        );
        let duplicated_passed = placement_intake(
            None,
            none,
            &["gex-1000".to_string(), "GEX-1000".to_string()],
            none,
            &all,
        )
        .expect_err("the same passed course twice");
        assert!(
            duplicated_passed.to_string().contains("duplicated"),
            "{duplicated_passed}"
        );
        let typo = placement_intake(
            None,
            &["ZZZ-9999".to_string()],
            none,
            none,
            &all,
        )
        .expect_err("a typed typo must not survive");
        assert!(typo.to_string().contains("ZZZ-9999"), "{typo}");
        let bad_pin = placement_intake(
            None,
            &["GEX-1000".to_string()],
            none,
            &["GEX-1000".to_string()],
            &all,
        )
        .expect_err("no session number");
        assert!(bad_pin.to_string().contains("CODE=SESSION"), "{bad_pin}");
    }

    // --- credit_total ---

    #[test]
    fn fixed_credits_sum_over_the_courses() {
        let courses = [monday("A-1", "1"), monday("B-2", "2")];
        let total = credit_total(&courses).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(total, 6);
    }

    #[test]
    fn a_variable_credit_stage_surfaces_its_missing_weighting() {
        // no weighting input exists yet (open question of the plan): the
        // error is surfaced, never defaulted to a bound
        let stage: Course = serde_json::from_str(
            r#"{"code":"GEX-2580","title":"Stage",
                "credits":{"min":6,"max":12},"cycle":1,
                "prerequisites":null,"equivalents":[],"seasons":{}}"#,
        )
        .unwrap_or_else(|e| panic!("stage literal: {e}"));
        let error = credit_total(&[stage]).expect_err("no chosen weighting");
        assert!(error.to_string().contains("GEX-2580"), "{error}");
    }
}
