use std::collections::{BTreeMap, BTreeSet};

use crate::course::{
    is_preuniversity, Course, PrereqTree, Prerequisites, Season,
};
use crate::preparatory::PREPARATORY_RULE_TITLE;
use crate::program::{
    Concentration, Profile, Program, Rule, RuleCourses, Semester,
    STAGES_RULE_TITLE,
};
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
    // the same typo rule as the coverage report: a chosen title the program
    // does not carry is surfaced, never guessed at (décision 2026-08-19 :
    // le solveur place aussi les obligatoires du cheminement choisi)
    #[error("no concentration titled « {title} » in the program")]
    UnknownConcentration { title: String },
    #[error("no profile titled « {title} » in the program")]
    UnknownProfile { title: String },
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
    // electives the intake added itself because a candidate's prerequisites
    // force them — surfaced so the caller can adopt and announce them,
    // never silent (ADR
    // `2026-08-injection-des-electifs-forces-par-les-prealables`)
    pub injected: Vec<String>,
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
// `set_aside`, never silently dropped. The chosen concentration and
// profile join the program scope whole: their mandatory courses enter the
// list and their rules feed the injection pool (décision 2026-08-19).
pub fn placement_intake(
    program: Option<&Program>,
    concentration: Option<&str>,
    profile: Option<&str>,
    electives: &[String],
    passed: &[String],
    pins: &[String],
    all: &[Course],
) -> Result<PlacementIntake, IntakeError> {
    let concentration = chosen_concentration(program, concentration)?;
    let profile = chosen_profile(program, profile)?;
    let electives = normalize_codes(electives)?;
    let passed_codes = normalize_codes(passed)?;
    let mut list = course_list(
        program,
        concentration,
        profile,
        &electives,
        &passed_codes,
    );
    let rules = scoped_rules(program, concentration, profile);
    let injected =
        inject_forced_electives(&mut list, &rules, &passed_codes, all);
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
        injected,
    })
}

// The chosen blocks, found by title. Choosing one the program does not
// carry — or choosing one with no program at all — is the student's typo,
// surfaced with the same words as the coverage report.
fn chosen_concentration<'a>(
    program: Option<&'a Program>,
    title: Option<&str>,
) -> Result<Option<&'a Concentration>, IntakeError> {
    let Some(title) = title else {
        return Ok(None);
    };
    program
        .and_then(|program| program.concentration(title))
        .map(Some)
        .ok_or_else(|| IntakeError::UnknownConcentration {
            title: title.to_string(),
        })
}

fn chosen_profile<'a>(
    program: Option<&'a Program>,
    title: Option<&str>,
) -> Result<Option<&'a Profile>, IntakeError> {
    let Some(title) = title else {
        return Ok(None);
    };
    program
        .and_then(|program| program.profile(title))
        .map(Some)
        .ok_or_else(|| IntakeError::UnknownProfile {
            title: title.to_string(),
        })
}

// every rule of the chosen scopes — the injection pool below, so a
// concentration's own electives are injectable exactly like the program's
fn scoped_rules<'a>(
    program: Option<&'a Program>,
    concentration: Option<&'a Concentration>,
    profile: Option<&'a Profile>,
) -> Vec<&'a Rule> {
    program
        .map(|program| program.rules.as_slice())
        .unwrap_or_default()
        .iter()
        .chain(
            concentration
                .map(|block| block.rules.as_slice())
                .unwrap_or_default(),
        )
        .chain(
            profile
                .map(|block| block.rules.as_slice())
                .unwrap_or_default(),
        )
        .collect()
}

// GMC-3002, mandatory at the B-GMC, requires GLO-1901 — an elective of a
// choice rule: no student finishes without it, so the intake takes it
// itself rather than letting the screen declare the whole program
// unplaceable. A code is injected when it is *forced* — some candidate's
// tree is unsatisfiable without it even granting every operand that could
// ever hold — and a rule of the chosen scopes lists it (a true elective).
// A choice between two electives forces neither and stays blocked, as does
// a forced code from outside the program: the injection never chooses for
// the student (ADR `2026-08-injection-des-electifs-forces-par-les-prealables`).
fn inject_forced_electives(
    list: &mut Vec<String>,
    rules: &[&Rule],
    passed: &[String],
    all: &[Course],
) -> Vec<String> {
    let pool: BTreeSet<&str> = rules
        .iter()
        .filter_map(|rule| match &rule.courses {
            RuleCourses::List { courses } => Some(courses.iter()),
            _ => None,
        })
        .flatten()
        .map(String::as_str)
        .collect();
    let by_code: BTreeMap<&str, &Course> = all
        .iter()
        .map(|course| (course.code.as_str(), course))
        .collect();
    let passed: BTreeSet<&str> = passed.iter().map(String::as_str).collect();
    let mut injected: Vec<String> = Vec::new();
    // an injected course can force further electives through its own tree:
    // iterate to the fixpoint — each productive round draws at least one
    // code from the pool, so the bound is the pool itself
    for _ in 0..=pool.len() {
        let round = forced_round(list, &pool, &passed, &by_code);
        if round.is_empty() {
            break;
        }
        injected.extend(round.iter().cloned());
        list.extend(round);
    }
    injected
}

// one scan of the current list: every leaf a candidate's tree forces, that
// a program rule lists and that the student neither holds nor passed
fn forced_round(
    list: &[String],
    pool: &BTreeSet<&str>,
    passed: &BTreeSet<&str>,
    by_code: &BTreeMap<&str, &Course>,
) -> Vec<String> {
    let held: BTreeSet<&str> = list.iter().map(String::as_str).collect();
    // « could this operand ever hold »: the same optimism the solver's
    // pre-search screen applies, plus the pool — an injectable elective
    // counts as holdable since this very pass can take it
    let possible = |code: &str| {
        passed.contains(code)
            || held.contains(code)
            || is_preuniversity(code)
            || pool.contains(code)
    };
    let mut round: Vec<String> = Vec::new();
    for code in list {
        if passed.contains(code.as_str()) {
            // history: its prerequisites are done with
            continue;
        }
        let Some(nodes) = by_code
            .get(code.as_str())
            .and_then(|course| parsed_tree(course))
            .and_then(flatten_lite)
        else {
            continue;
        };
        if !eval_lite(&nodes, &possible) {
            // hopeless even with every possible operand granted — the
            // screen's to name, not the injection's to guess about
            continue;
        }
        for leaf in nodes.iter().filter_map(|node| match node {
            LiteNode::Course(leaf) => Some(*leaf),
            _ => None,
        }) {
            if held.contains(leaf)
                || passed.contains(leaf)
                || !pool.contains(leaf)
                || round.iter().any(|kept| kept == leaf)
            {
                continue;
            }
            if !eval_lite(&nodes, &|code: &str| code != leaf && possible(code))
            {
                round.push(leaf.to_string());
            }
        }
    }
    round
}

fn parsed_tree(course: &Course) -> Option<&PrereqTree> {
    match &course.prerequisites {
        Some(Prerequisites::Parsed { tree, .. }) => Some(tree),
        _ => None,
    }
}

// the injection's view of a tree: course leaves and connectors — raw text
// and credit thresholds are optimistically satisfiable, so they collapse
// to `Free`
enum LiteNode<'a> {
    Course(&'a str),
    Free,
    All(Vec<usize>),
    Any(Vec<usize>),
}

// same budget as the solver's `MAX_TREE_NODES` — a tree past it injects
// nothing and is left to the solver, which refuses it loudly
const MAX_INJECTION_NODES: usize = 10_000;

// breadth-first, children after their parent, so one reverse scan
// evaluates children before parents — no recursion, no unbounded loop
// (the shape the solver's own `flatten` uses)
fn flatten_lite(tree: &PrereqTree) -> Option<Vec<LiteNode<'_>>> {
    let mut pending: Vec<&PrereqTree> = vec![tree];
    let mut nodes: Vec<LiteNode> = Vec::new();
    for cursor in 0..MAX_INJECTION_NODES {
        if cursor >= pending.len() {
            return Some(nodes);
        }
        nodes.push(match pending[cursor] {
            PrereqTree::Course(code) => LiteNode::Course(code),
            PrereqTree::Raw { .. } | PrereqTree::ProgramCredits { .. } => {
                LiteNode::Free
            }
            PrereqTree::All { all } => {
                let children =
                    (pending.len()..pending.len() + all.len()).collect();
                pending.extend(all.iter());
                LiteNode::All(children)
            }
            PrereqTree::Any { any } => {
                let children =
                    (pending.len()..pending.len() + any.len()).collect();
                pending.extend(any.iter());
                LiteNode::Any(children)
            }
        });
    }
    None
}

fn eval_lite(nodes: &[LiteNode], leaf: &impl Fn(&str) -> bool) -> bool {
    let mut verdicts = vec![false; nodes.len()];
    for i in (0..nodes.len()).rev() {
        verdicts[i] = match &nodes[i] {
            LiteNode::Course(code) => leaf(code),
            LiteNode::Free => true,
            LiteNode::All(children) => {
                children.iter().all(|&child| verdicts[child])
            }
            LiteNode::Any(children) => {
                children.iter().any(|&child| verdicts[child])
            }
        };
    }
    verdicts.first().copied().unwrap_or(true)
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

// The horizon's seasons turned into semesters — the only place the app
// does calendar arithmetic: a hiver belongs to the civil year after its
// automne, an été and the next automne keep the hiver's year
// (« A2026 → H2027 → É2027 → A2027 → … »).
pub fn session_semesters(
    start: Semester,
    seasons: &[Season],
) -> Vec<Semester> {
    let mut year = start.year;
    let mut previous: Option<Season> = None;
    seasons
        .iter()
        .map(|&season| {
            if previous == Some(Season::Fall) {
                year += 1;
            }
            previous = Some(season);
            Semester { season, year }
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
// courses (reference order), the chosen concentration's and profile's
// mandatory courses (décision 2026-08-19), the mandatory stage (the
// « Stages » rule lists it first, ADR
// `2026-08-stage-obligatoire-en-prose-promu-en-regle`), the chosen
// electives and the passed courses — deduplicated, so a passed mandatory
// course appears once and carries its Course object (ADR
// `2026-08-stage-obligatoire-et-scolarite-preparatoire-dans-lintake`).
pub fn course_list(
    program: Option<&Program>,
    concentration: Option<&Concentration>,
    profile: Option<&Profile>,
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
            concentration
                .map(|block| block.mandatory.clone())
                .unwrap_or_default(),
        )
        .chain(
            profile
                .map(|block| block.mandatory.clone())
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

    // --- session_semesters ---

    #[test]
    fn the_semester_walk_crosses_civil_years_at_each_automne() {
        let seasons = horizon_sessions(Season::Fall, 4);
        let start = "A26".parse().unwrap_or_else(|e| panic!("{e}"));
        let labels: Vec<String> = session_semesters(start, &seasons)
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(labels, ["A26", "H27", "E27", "A27", "H28", "E28"]);
    }

    #[test]
    fn a_winter_start_keeps_its_own_year() {
        let seasons = [Season::Winter, Season::Summer, Season::Fall];
        let start = "H27".parse().unwrap_or_else(|e| panic!("{e}"));
        let labels: Vec<String> = session_semesters(start, &seasons)
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(labels, ["H27", "E27", "A27"]);
    }

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
            None,
            None,
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
            None,
            None,
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
        assert_eq!(course_list(Some(&program), None, None, &[], &[]), ["M-1"]);
    }

    fn program_with_scopes() -> Program {
        serde_json::from_str(
            r#"{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":["M-1"],"rules":[],
                "concentrations":[
                  {"title":"Robotique","mandatory":["C-1"],
                   "rules":[{"title":"Règle 1",
                             "constraint":{"type":"credits","min":3,"max":3},
                             "courses":["C-2"]}]}],
                "profiles":[
                  {"title":"Profil international","mandatory":["F-1"],
                   "rules":[]}]}"#,
        )
        .unwrap_or_else(|e| panic!("program literal: {e}"))
    }

    #[test]
    fn the_course_list_adds_the_chosen_scopes_mandatory_courses() {
        // program mandatory first, then the chosen concentration's, then
        // the chosen profile's — reference order, before the electives
        let program = program_with_scopes();
        let list = course_list(
            Some(&program),
            program.concentration("Robotique"),
            program.profile("Profil international"),
            &["E-1".to_string()],
            &[],
        );
        assert_eq!(list, ["M-1", "C-1", "F-1", "E-1"]);
        let unscoped =
            course_list(Some(&program), None, None, &["E-1".to_string()], &[]);
        assert_eq!(unscoped, ["M-1", "E-1"], "no choice, no scoped course");
    }

    #[test]
    fn an_unknown_concentration_or_profile_is_an_error_naming_it() {
        let program = program_with_scopes();
        let all: [Course; 0] = [];
        let concentration = placement_intake(
            Some(&program),
            Some("Zzz"),
            None,
            &[],
            &[],
            &[],
            &all,
        )
        .expect_err("no concentration titled Zzz");
        assert!(concentration.to_string().contains("Zzz"), "{concentration}");
        let profile = placement_intake(
            Some(&program),
            None,
            Some("Yyy"),
            &[],
            &[],
            &[],
            &all,
        )
        .expect_err("no profile titled Yyy");
        assert!(profile.to_string().contains("Yyy"), "{profile}");
        // a choice with no program at all is the same typo, not a pass
        let orphan =
            placement_intake(None, Some("Zzz"), None, &[], &[], &[], &all)
                .expect_err("a concentration needs a program");
        assert!(orphan.to_string().contains("Zzz"), "{orphan}");
        let orphan_profile =
            placement_intake(None, None, Some("Yyy"), &[], &[], &[], &all)
                .expect_err("a profile needs a program");
        assert!(
            orphan_profile.to_string().contains("Yyy"),
            "{orphan_profile}"
        );
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
            None,
            None,
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
            None,
            None,
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

    // --- forced-elective injection ---

    fn program_with_rule(mandatory: &str, listed: &str) -> Program {
        // the negotiated rule proves a non-list rule feeds the pool nothing
        serde_json::from_str(&format!(
            r#"{{"code":"p","slug":"p","semester":"A26","title":"P","cycle":1,
                "credits_required":120,"mandatory":[{mandatory}],
                "rules":[{{"title":"Règle 1",
                           "constraint":{{"type":"course","min":1,"max":1}},
                           "courses":[{listed}]}},
                         {{"title":"Règle 2","courses":"negotiated",
                           "raw":"convenus avec la direction"}}],
                "concentrations":[],"profiles":[]}}"#
        ))
        .unwrap_or_else(|e| panic!("program literal: {e}"))
    }

    fn with_prereqs(code: &str, nrc: &str, tree: &str) -> Course {
        let mut course = monday(code, nrc);
        course.prerequisites = Some(
            serde_json::from_str(&format!(r#"{{"raw":"r","tree":{tree}}}"#))
                .unwrap_or_else(|e| panic!("prerequisites literal: {e}")),
        );
        course
    }

    #[test]
    fn a_forced_elective_is_injected_once_and_surfaced() {
        let program = program_with_rule(
            r#""GMC-3002","GMC-3003""#,
            r#""GLO-1901","IFT-1903""#,
        );
        // both mandatory courses force the same elective: injected once —
        // the raw operand is optimistically satisfiable, never blocking
        let all = [
            with_prereqs("GMC-3002", "1", r#""GLO-1901""#),
            with_prereqs(
                "GMC-3003",
                "2",
                r#"{"all":["GLO-1901",{"raw":"un examen"}]}"#,
            ),
            monday("GLO-1901", "3"),
            monday("IFT-1903", "4"),
        ];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(intake.injected, ["GLO-1901"]);
        assert!(intake.selection.contains("GLO-1901"), "counts in coverage");
        assert!(
            intake
                .courses
                .iter()
                .any(|course| course.code == "GLO-1901"),
            "the injected course reaches the solver"
        );
        assert!(
            !intake.selection.contains("IFT-1903"),
            "the untouched alternative stays unchosen"
        );
    }

    #[test]
    fn a_choice_between_two_listed_electives_forces_neither() {
        let program =
            program_with_rule(r#""GMC-3002""#, r#""GLO-1901","IFT-1903""#);
        let all = [
            with_prereqs(
                "GMC-3002",
                "1",
                r#"{"any":["GLO-1901","IFT-1903"]}"#,
            ),
            monday("GLO-1901", "3"),
            monday("IFT-1903", "4"),
        ];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(intake.injected.is_empty(), "the injection never chooses");
    }

    #[test]
    fn a_forced_code_from_outside_the_program_is_never_injected() {
        // XYZ-1000 could never hold: the tree is hopeless, and naming it is
        // the pre-search screen's job, not the injection's
        let program = program_with_rule(r#""GMC-3002""#, r#""GLO-1901""#);
        let all = [
            with_prereqs("GMC-3002", "1", r#""XYZ-1000""#),
            monday("GLO-1901", "3"),
        ];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(intake.injected.is_empty());
    }

    #[test]
    fn an_unavailable_alternative_leaves_the_listed_elective_forced() {
        // any(GLO-1901, XYZ-1000): the outside code could never hold, so
        // the listed elective is the only satisfying branch — forced
        let program = program_with_rule(r#""GMC-3002""#, r#""GLO-1901""#);
        let all = [
            with_prereqs(
                "GMC-3002",
                "1",
                r#"{"any":["GLO-1901","XYZ-1000"]}"#,
            ),
            monday("GLO-1901", "3"),
        ];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(intake.injected, ["GLO-1901"]);
    }

    #[test]
    fn injection_chains_through_the_injected_courses_own_tree() {
        let program =
            program_with_rule(r#""GMC-3002""#, r#""GLO-1901","GLO-1902""#);
        let all = [
            with_prereqs("GMC-3002", "1", r#""GLO-1901""#),
            with_prereqs("GLO-1901", "2", r#""GLO-1902""#),
            monday("GLO-1902", "3"),
        ];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(intake.injected, ["GLO-1901", "GLO-1902"]);
    }

    #[test]
    fn held_or_passed_codes_are_never_reinjected() {
        let program = program_with_rule(r#""GMC-3002""#, r#""GLO-1901""#);
        let all = [
            with_prereqs("GMC-3002", "1", r#""GLO-1901""#),
            monday("GLO-1901", "3"),
        ];
        let chosen = placement_intake(
            Some(&program),
            None,
            None,
            &["glo-1901".to_string()],
            &[],
            &[],
            &all,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(chosen.injected.is_empty(), "already an elective");

        let done = placement_intake(
            Some(&program),
            None,
            None,
            &[],
            &["gmc-3002".to_string()],
            &[],
            &all,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(done.injected.is_empty(), "a passed course is history");
    }

    #[test]
    fn a_concentration_elective_forced_by_its_mandatory_is_injected() {
        // C-1, mandatory of the chosen concentration, requires C-2 — listed
        // only by the concentration's own rule: the injection pool covers
        // the chosen scopes, so the elective is taken and surfaced
        let program = program_with_scopes();
        let all = [
            monday("M-1", "1"),
            with_prereqs("C-1", "2", r#""C-2""#),
            monday("C-2", "3"),
            monday("F-1", "4"),
        ];
        let intake = placement_intake(
            Some(&program),
            Some("Robotique"),
            Some("Profil international"),
            &[],
            &[],
            &[],
            &all,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(intake.injected, ["C-2"]);
        let codes: Vec<&str> = intake
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["M-1", "C-1", "F-1", "C-2"]);

        let unscoped =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            unscoped.injected.is_empty(),
            "an unchosen concentration feeds the solver nothing"
        );
        assert!(!unscoped.selection.contains("C-1"));
    }

    #[test]
    fn a_tree_over_budget_injects_nothing_and_is_left_to_the_solver() {
        let program = program_with_rule(r#""GMC-3002""#, r#""GLO-1901""#);
        let mut wide = monday("GMC-3002", "1");
        wide.prerequisites = Some(Prerequisites::Parsed {
            raw: "r".to_string(),
            tree: PrereqTree::All {
                all: (0..MAX_INJECTION_NODES)
                    .map(|_| PrereqTree::Course("GLO-1901".to_string()))
                    .collect(),
            },
        });
        let all = [wide, monday("GLO-1901", "3")];
        let intake =
            placement_intake(Some(&program), None, None, &[], &[], &[], &all)
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(intake.injected.is_empty());
    }

    #[test]
    fn the_placement_pipeline_propagates_every_intake_error() {
        let all = [monday("GEX-1000", "1")];
        let none: &[String] = &[];
        let duplicated_electives = placement_intake(
            None,
            None,
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
            None,
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
            None,
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
            None,
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
