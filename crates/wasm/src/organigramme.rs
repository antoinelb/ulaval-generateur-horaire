use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    coverage_report, horizon_sessions, place, placement_intake,
    summer_indices, Course, CoverageReport, Placement, PlacementError,
    PlacementIntake, PlacementRequest, Program, Season,
};

// Browser-sized budgets: the search runs on the JS thread, so the defaults
// stop long before a tab freezes. Truncation is never silent — the
// report's `completion` says which bound was hit — and a caller who knows
// better raises them (ADR `2026-07-budget-de-b-en-double-borne`).
const DEFAULT_MAX_NODES: u64 = 1_000_000;
const DEFAULT_MAX_SOLUTIONS: usize = 100;

// What JS hands the two organigramme functions. The horizon is described,
// never listed: `start` and `study_sessions` go through
// `horizon_sessions`, so the été-after-each-hiver rule stays in core and
// out of the view. Unknown fields are refused rather than ignored.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
#[serde(deny_unknown_fields)]
pub struct OrganigrammeInput {
    // Wire format only: the functions below never read it — the boundary
    // resolves it against the loaded snapshot (`catalogue::resolve`) and
    // hands the result as their `courses` argument. Optional since a worker
    // that called `init_snapshot` stops sending the catalogue per call.
    #[serde(default)]
    pub courses: Option<Vec<Course>>,
    #[serde(default)]
    pub program: Option<Program>,
    #[serde(default)]
    pub concentration: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub electives: Vec<String>,
    #[serde(default)]
    pub passed: Vec<String>,
    // code → 1-based session number; for `verify` it is the whole
    // organigramme the student assembled
    #[serde(default)]
    #[cfg_attr(
        all(target_arch = "wasm32", feature = "boundary"),
        tsify(type = "Record<string, number>")
    )]
    pub pinned: BTreeMap<String, usize>,
    pub start: Season,
    // the A/H alternation only — the étés come on top
    pub study_sessions: usize,
    pub credit_cap: u32,
    #[serde(default)]
    pub concomitant: bool,
    // ponytail: all the étés or none — per-été opening when the UI asks
    #[serde(default)]
    pub summers_open: bool,
    #[serde(default)]
    #[cfg_attr(
        all(target_arch = "wasm32", feature = "boundary"),
        tsify(type = "Record<string, number>")
    )]
    pub seed: BTreeMap<String, usize>,
    #[serde(default)]
    pub max_nodes: Option<u64>,
    #[serde(default)]
    pub max_solutions: Option<usize>,
}

// The horizon is returned with the placement: the session numbers only mean
// something next to the seasons they index, and JS never computed them.
#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
pub struct OrganigrammeReport {
    pub sessions: Vec<Season>,
    pub placement: Placement,
    // program-derived codes the snapshot does not carry — surfaced, never
    // dropped (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`)
    pub set_aside: Vec<String>,
    // electives the intake added because a candidate's prerequisites force
    // them — the caller adopts and announces them, never silently (ADR
    // `2026-08-injection-des-electifs-forces-par-les-prealables`)
    pub injected: Vec<String>,
    // regular courses the escalation seated in an été the caller had
    // closed — named so the UI explains instead of silently overriding the
    // setting (ADR `2026-08-escalade-etes-ouverts-dans-le-repli`)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub summers_forced: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageReport>,
}

// Build the path: every course of the program (plus the chosen electives)
// laid over the horizon, `pinned` fixing whatever the student already
// settled.
pub fn generate(
    input: &OrganigrammeInput,
    courses: &[Course],
) -> Result<OrganigrammeReport, String> {
    let intake = intake(input, courses)?;
    let sessions = horizon_sessions(input.start, input.study_sessions);
    let placement = with_request(input, &intake, &sessions, place_escalating)?;
    let summers_forced =
        forced_summers(&placement, &intake, &sessions, input.summers_open);
    Ok(OrganigrammeReport {
        sessions,
        placement,
        set_aside: intake.set_aside,
        injected: intake.injected,
        summers_forced,
        coverage: None,
    })
}

// « Proposer » wants a grid, not a verdict. The exact arrangement first —
// unchanged, and it answers whenever it can. When it yields nothing the
// escalation runs: the same exact question with every été open (the
// demotion in core keeps them a last resort), then only the best-effort
// pass — every course it does place still honours every constraint, and
// what does not fit is left out and named rather than seated in violation
// (ADRs `2026-08-placement-au-mieux-en-repli`,
// `2026-08-escalade-etes-ouverts-dans-le-repli`). The « Ouvrir les étés »
// setting itself is never touched: the report names the codes forced into
// an été and the caller explains.
//
// The relaxed pass is cheap whatever the others cost: the sentinel is
// available at every depth, so the first leaf is reached in about one
// expansion per course. It keeps its own `completion` — inheriting the
// exact pass's would answer a question nobody asked of it.
fn place_escalating(
    request: &PlacementRequest,
) -> Result<Placement, PlacementError> {
    let exact = place(request)?;
    if !exact.solutions.is_empty() {
        return Ok(exact);
    }
    let all_summers = summer_indices(request.sessions);
    let escalated = if all_summers != *request.open_summers {
        place(&PlacementRequest {
            open_summers: &all_summers,
            ..*request
        })
    } else {
        Ok(exact)
    };
    escalated.and_then(|opened| {
        if !opened.solutions.is_empty() {
            return Ok(opened);
        }
        place(&PlacementRequest {
            allow_unplaced: true,
            // the sentinel is tried last, so the first leaf is the greedy
            // filling and every later one is strictly worse
            max_solutions: 1,
            open_summers: &all_summers,
            ..*request
        })
    })
}

// The codes the escalation seated in an été the plan keeps closed — read
// off the winning solution, so a pass that opened the étés without using
// them declares nothing. A pin or a stage sits there by its own right,
// never « forced ».
fn forced_summers(
    placement: &Placement,
    intake: &PlacementIntake,
    sessions: &[Season],
    summers_open: bool,
) -> Vec<String> {
    if summers_open {
        return Vec::new();
    }
    let summers = summer_indices(sessions);
    placement
        .solutions
        .first()
        .map(|solution| {
            solution
                .placement
                .iter()
                .filter(|(code, session)| {
                    summers.contains(session)
                        && !intake.stages.contains(*code)
                        && !intake.pinned.contains_key(*code)
                })
                .map(|(code, _)| code.clone())
                .collect()
        })
        .unwrap_or_default()
}

// Verify the path the student assembled — the two halves of « is this bac
// whole », per his decision of 2026-08-02 (ADR
// `2026-08-module-wasm-quatre-fonctions-js`): `place` proves the placement
// itself (prerequisites, credit cap, closed étés, weekly feasibility) with
// every remaining course pinned, so it builds nothing; `coverage_report`
// answers whether the selection satisfies the program's rules. A course
// left without a session is an incomplete question, not a false verdict.
pub fn verify(
    input: &OrganigrammeInput,
    courses: &[Course],
) -> Result<OrganigrammeReport, String> {
    let intake = intake(input, courses)?;
    let unplaced: Vec<String> = intake
        .courses
        .iter()
        .map(|course| &course.code)
        .filter(|code| {
            !intake.passed.contains(*code)
                && !intake.pinned.contains_key(*code)
        })
        .cloned()
        .collect();
    if !unplaced.is_empty() {
        return Err(format!(
            "verification needs a session for every course left to place : {}",
            unplaced.join(", ")
        ));
    }
    let sessions = horizon_sessions(input.start, input.study_sessions);
    let placement = solve(input, &intake, &sessions)?;
    let coverage = input
        .program
        .as_ref()
        .map(|program| {
            coverage_report(
                program,
                input.concentration.as_deref(),
                input.profile.as_deref(),
                &intake.selection,
                &intake.courses,
            )
        })
        .transpose()
        .map_err(|e| e.to_string())?;
    Ok(OrganigrammeReport {
        sessions,
        placement,
        set_aside: intake.set_aside,
        injected: intake.injected,
        summers_forced: Vec::new(),
        coverage,
    })
}

// The sessions that could host `code` — what the JS interface needs for its
// « + H28 » chips (CORRECTIFS-AMONT item 12): one `place` probe per session,
// pin semantics answering the very question the click asks. 1-based numbers,
// the shape `pinned` speaks.
pub fn admissible(
    input: &OrganigrammeInput,
    courses: &[Course],
    code: &str,
) -> Result<Vec<usize>, String> {
    let intake = intake(input, courses)?;
    let sessions = horizon_sessions(input.start, input.study_sessions);
    let code = code.to_uppercase();
    with_request(input, &intake, &sessions, |request| {
        ulaval_scheduler_core::admissible_sessions(request, &code)
    })
    .map(|admissible| admissible.into_iter().collect())
}

fn intake(
    input: &OrganigrammeInput,
    courses: &[Course],
) -> Result<PlacementIntake, String> {
    // ponytail: `placement_intake` speaks the `CODE=SESSION` shape, so one
    // format! reuses its whole validation instead of a second parser
    let pins: Vec<String> = input
        .pinned
        .iter()
        .map(|(code, session)| format!("{code}={session}"))
        .collect();
    placement_intake(
        input.program.as_ref(),
        input.concentration.as_deref(),
        input.profile.as_deref(),
        &input.electives,
        &input.passed,
        &pins,
        courses,
    )
    .map_err(|e| e.to_string())
}

fn solve(
    input: &OrganigrammeInput,
    intake: &PlacementIntake,
    sessions: &[Season],
) -> Result<Placement, String> {
    with_request(input, intake, sessions, place)
}

fn with_request<T>(
    input: &OrganigrammeInput,
    intake: &PlacementIntake,
    sessions: &[Season],
    ask: impl FnOnce(&PlacementRequest) -> Result<T, PlacementError>,
) -> Result<T, String> {
    // an été absent from the set still hosts stages and pinned courses (ADR
    // `2026-08-stage-place-en-ete-sauf-epinglage`)
    let open_summers = if input.summers_open {
        summer_indices(sessions)
    } else {
        BTreeSet::new()
    };
    ask(&PlacementRequest {
        // proving stays proving: only `generate` escalates, and it says so
        // by passing `place_escalating` as its `ask`
        allow_unplaced: false,
        sessions,
        credit_cap: input.credit_cap,
        concomitant: input.concomitant,
        courses: &intake.courses,
        passed: &intake.passed,
        pinned: &intake.pinned,
        stages: &intake.stages,
        open_summers: &open_summers,
        seed: &input.seed,
        max_nodes: input.max_nodes.unwrap_or(DEFAULT_MAX_NODES),
        max_solutions: input.max_solutions.unwrap_or(DEFAULT_MAX_SOLUTIONS),
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use ulaval_scheduler_core::Completion;

    // GEX-1001 requires GEX-1000; both offered fall and winter, GEX-1002
    // only in summer
    const COURSES: &str = r#"[
        {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
         "prerequisites":null,"equivalents":[],
         "seasons":{"fall":{"last_offered":2026,"options":null},
                    "winter":{"last_offered":2026,"options":null}}},
        {"code":"GEX-1001","title":"T","credits":3,"cycle":1,
         "prerequisites":{"raw":"GEX-1000","tree":"GEX-1000"},
         "equivalents":[],
         "seasons":{"fall":{"last_offered":2026,"options":null},
                    "winter":{"last_offered":2026,"options":null}}},
        {"code":"GEX-1002","title":"T","credits":3,"cycle":1,
         "prerequisites":null,"equivalents":[],
         "seasons":{"summer":{"last_offered":2026,"options":null}}}
    ]"#;

    const PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
        "title":"P","cycle":1,"credits_required":6,
        "mandatory":["GEX-1000","GEX-1001"],
        "rules":[],"concentrations":[],"profiles":[]}"#;

    fn courses() -> Vec<Course> {
        serde_json::from_str(COURSES)
            .unwrap_or_else(|e| panic!("courses literal: {e}"))
    }

    fn input(fields: &str) -> OrganigrammeInput {
        serde_json::from_str(&format!(
            r#"{{"start":"fall","study_sessions":2,
                 "credit_cap":6,{fields}}}"#
        ))
        .unwrap_or_else(|e| panic!("input literal: {e}"))
    }

    // The field the functions never read: its only reader is the boundary,
    // which no native test compiles — so the wire contract is proven here.
    #[test]
    fn the_catalogue_rides_in_the_input_or_stays_out_of_it() {
        assert!(input(r#""electives":[]"#).courses.is_none());
        let carried: OrganigrammeInput = serde_json::from_str(&format!(
            r#"{{"courses":{COURSES},"start":"fall","study_sessions":2,
                 "credit_cap":6}}"#
        ))
        .unwrap_or_else(|e| panic!("input literal: {e}"));
        assert_eq!(carried.courses.map(|courses| courses.len()), Some(3));
    }

    // The 2026-08-14 user report: a course nothing can seat used to freeze
    // the whole proposal, grid untouched and no message. Now the exact and
    // escalated passes still answer nothing and the best-effort pass fills
    // what it can.
    #[test]
    fn generation_falls_back_to_a_best_effort_filling() {
        // a one-session horizon (fall only, so no été to open): GEX-1001
        // needs its prerequisite strictly earlier and GEX-1002 is
        // summer-only — nothing exact exists at any escalation
        let query: OrganigrammeInput = serde_json::from_str(&format!(
            r#"{{"start":"fall","study_sessions":1,"credit_cap":6,
                 "program":{PROGRAM},"electives":["GEX-1002"]}}"#
        ))
        .unwrap_or_else(|e| panic!("input literal: {e}"));
        let report =
            generate(&query, &courses()).unwrap_or_else(|e| panic!("{e}"));
        let solution = report
            .placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the fallback answers"));
        // the placeable course lands, the others are left out and named,
        // never seated in violation
        assert_eq!(solution.placement["GEX-1000"], 1);
        assert_eq!(
            solution.left_out,
            BTreeSet::from(["GEX-1001".to_string(), "GEX-1002".to_string()])
        );
        // no été was opened for anything: none exists on this horizon
        assert!(report.summers_forced.is_empty());
        // the culprit keeps its reason for the UI to word
        assert_eq!(report.placement.blocked[0].code, "GEX-1002");
    }

    #[test]
    fn a_closed_ete_is_opened_as_a_last_resort_and_named() {
        // summer-only GEX-1002 with the étés closed: the exact pass proves
        // nothing, the escalation opens the étés and names the forced code
        // — the « Ouvrir les étés » setting itself stays the student's
        let report =
            generate(&input(r#""electives":["gex-1002"]"#), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            report.placement.blocked.is_empty(),
            "the winning pass sees an open été"
        );
        assert_eq!(report.placement.solutions[0].placement["GEX-1002"], 3);
        assert_eq!(report.summers_forced, ["GEX-1002"]);
    }

    #[test]
    fn an_ete_the_student_opened_is_never_declared_forced() {
        // GEX-1001 misses its prerequisite, so even the open-été exact
        // pass fails and the best-effort one answers: GEX-1002 sits in an
        // été the student opened — nothing was forced
        let report = generate(
            &input(
                r#""electives":["gex-1001","gex-1002"],"summers_open":true"#,
            ),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let solution = report
            .placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the fallback answers"));
        assert_eq!(solution.placement["GEX-1002"], 3);
        assert!(solution.left_out.contains("GEX-1001"));
        assert!(report.summers_forced.is_empty());
    }

    #[test]
    fn a_stage_in_its_ete_is_never_declared_forced() {
        // a closed été already hosts stages by right (ADR
        // `2026-08-stage-place-en-ete-sauf-epinglage`): the exact pass
        // wins and nothing is « forced »
        let program = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
            "title":"P","cycle":1,"credits_required":6,
            "mandatory":["GEX-1000"],
            "rules":[{"title":"Stages",
                      "constraint":{"type":"course","min":1,"max":8},
                      "courses":["GEX-1002"],"credits_in_addition":true}],
            "concentrations":[],"profiles":[]}"#;
        let report =
            generate(&input(&format!(r#""program":{program}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.placement.solutions[0].placement["GEX-1002"], 3);
        assert!(
            report.summers_forced.is_empty(),
            "the été is the stage's home"
        );
    }

    #[test]
    fn a_pinned_summer_course_is_never_declared_forced() {
        // the pin already grants the été (ADR
        // `2026-08-stage-place-en-ete-sauf-epinglage`): the escalation
        // must not claim it forced anything
        let report = generate(
            &input(
                r#""electives":["gex-1001","gex-1002"],
                   "pinned":{"GEX-1002":3}"#,
            ),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let solution = report
            .placement
            .solutions
            .first()
            .unwrap_or_else(|| panic!("the fallback answers"));
        assert_eq!(solution.placement["GEX-1002"], 3);
        assert!(report.summers_forced.is_empty());
    }

    #[test]
    fn an_exhausted_budget_still_answers_with_an_empty_report() {
        let report = generate(
            &input(r#""electives":["gex-1002"],"max_nodes":0"#),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.placement.solutions.is_empty());
        assert!(report.summers_forced.is_empty());
    }

    // The nominal case must not move an inch: the escalation only ever
    // runs after an exact pass that answered nothing.
    #[test]
    fn a_solvable_program_never_reaches_the_fallback() {
        let report =
            generate(&input(&format!(r#""program":{PROGRAM}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.summers_forced.is_empty());
        for solution in &report.placement.solutions {
            assert!(solution.left_out.is_empty(), "{:?}", solution.left_out);
        }
    }

    #[test]
    fn generation_lays_the_program_over_the_horizon() {
        let report =
            generate(&input(&format!(r#""program":{PROGRAM}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        // the horizon inserts an été after the hiver
        assert_eq!(
            report.sessions,
            [Season::Fall, Season::Winter, Season::Summer]
        );
        assert_eq!(report.placement.completion, Completion::Complete);
        assert_eq!(
            report.placement.solutions[0].placement,
            BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GEX-1001".to_string(), 2),
            ])
        );
        assert!(report.coverage.is_none(), "generation does not count rules");
    }

    #[test]
    fn generation_places_the_chosen_concentrations_mandatory_courses() {
        // GEX-1001 is mandatory only inside the concentration: unchosen it
        // never reaches the solver, chosen it is placed like the program's
        // own (décision 2026-08-19)
        let program = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
            "title":"P","cycle":1,"credits_required":6,
            "mandatory":["GEX-1000"],"rules":[],
            "concentrations":[{"title":"Robotique",
                               "mandatory":["GEX-1001"],"rules":[]}],
            "profiles":[]}"#;
        let unchosen =
            generate(&input(&format!(r#""program":{program}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            !unchosen.placement.solutions[0]
                .placement
                .contains_key("GEX-1001"),
            "an unchosen concentration feeds the solver nothing"
        );

        let chosen = generate(
            &input(&format!(
                r#""program":{program},"concentration":"Robotique""#
            )),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(chosen.placement.solutions[0].placement["GEX-1001"], 2);
    }

    #[test]
    fn generation_surfaces_an_unknown_concentration() {
        let error = generate(
            &input(&format!(r#""program":{PROGRAM},"concentration":"Zzz""#)),
            &courses(),
        )
        .expect_err("no concentration titled Zzz");
        assert!(error.contains("Zzz"), "{error}");
    }

    #[test]
    fn generation_sets_aside_the_codes_the_snapshot_does_not_carry() {
        let program = PROGRAM
            .replace(r#""GEX-1000","GEX-1001""#, r#""GEX-1000","GHOST-999""#);
        let report =
            generate(&input(&format!(r#""program":{program}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.set_aside, ["GHOST-999"]);
    }

    #[test]
    fn an_open_ete_hosts_a_summer_course_without_forcing_anything() {
        let open = generate(
            &input(r#""electives":["gex-1002"],"summers_open":true"#),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(open.placement.blocked.is_empty());
        assert_eq!(open.placement.solutions[0].placement["GEX-1002"], 3);
        assert!(
            open.summers_forced.is_empty(),
            "an été the student opened forces nothing"
        );
    }

    // the JSON names are what JS reads off the returned object
    #[test]
    fn the_report_serializes_under_its_published_names() {
        let generated =
            generate(&input(&format!(r#""program":{PROGRAM}"#)), &courses())
                .unwrap_or_else(|e| panic!("{e}"));
        let json = serde_json::to_value(&generated)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        assert_eq!(
            json["sessions"],
            serde_json::json!(["fall", "winter", "summer"])
        );
        assert_eq!(json["placement"]["completion"], "complete");
        assert_eq!(
            json["placement"]["solutions"][0]["placement"]["GEX-1001"],
            2
        );
        assert_eq!(json["set_aside"], serde_json::json!([]));
        assert!(json.get("coverage").is_none(), "generation counts no rule");
        assert!(
            json.get("summers_forced").is_none(),
            "the field only rides when the escalation used it"
        );

        let verified = verify(
            &input(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
            )),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let json = serde_json::to_value(&verified)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        assert_eq!(json["coverage"]["mandatory"][0]["scope"], "program");
    }

    #[test]
    fn admissible_sessions_probe_offer_and_precedence() {
        let courses = courses();
        let both = input(&format!(r#""program":{PROGRAM}"#));
        // GEX-1000 must precede GEX-1001, and the movable GEX-1001 has no
        // seat left past the hiver — so each course has exactly one session
        assert_eq!(
            admissible(&both, &courses, "gex-1000")
                .unwrap_or_else(|e| panic!("{e}")),
            [1]
        );
        assert_eq!(
            admissible(&both, &courses, "GEX-1001")
                .unwrap_or_else(|e| panic!("{e}")),
            [2]
        );
        // summer-only GEX-1002: a pin is deliberate, so it lands in the été
        // even closed (ADR `2026-08-stage-place-en-ete-sauf-epinglage`)
        let summer = input(r#""electives":["gex-1002"]"#);
        assert_eq!(
            admissible(&summer, &courses, "GEX-1002")
                .unwrap_or_else(|e| panic!("{e}")),
            [3]
        );
    }

    #[test]
    fn admissible_sessions_surface_every_intake_error() {
        let error =
            admissible(&input(r#""electives":["ZZZ-9999"]"#), &courses(), "Z")
                .expect_err("a typed typo must not survive");
        assert!(error.contains("ZZZ-9999"), "{error}");
    }

    #[test]
    fn generation_surfaces_every_intake_error() {
        let error =
            generate(&input(r#""electives":["ZZZ-9999"]"#), &courses())
                .expect_err("a typed typo must not survive");
        assert!(error.contains("ZZZ-9999"), "{error}");
    }

    #[test]
    fn generation_surfaces_every_placement_error() {
        let error = generate(&input(r#""electives":[]"#), &courses())
            .expect_err("no course to place");
        assert!(error.contains("at least one session"), "{error}");
    }

    #[test]
    fn verification_needs_a_session_for_every_course_left_to_place() {
        let error = verify(
            &input(&format!(
                r#""program":{PROGRAM},"pinned":{{"GEX-1000":1}}"#
            )),
            &courses(),
        )
        .expect_err("GEX-1001 has no session");
        assert!(error.contains("GEX-1001"), "{error}");
    }

    #[test]
    fn verification_proves_the_students_own_path_and_counts_the_rules() {
        let report = verify(
            &input(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"gex-1000":1,"gex-1001":2}}"#
            )),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.placement.completion, Completion::Complete);
        assert_eq!(report.placement.solutions.len(), 1, "one pinned path");
        let coverage = report.coverage.expect("a program was given");
        assert!(coverage.mandatory[0].missing.is_empty());
    }

    #[test]
    fn verification_refuses_a_path_that_breaks_a_prerequisite() {
        // GEX-1001 before GEX-1000: the pins are honoured, no solution
        // survives, and the search is complete — a proof, not a budget cut
        let report = verify(
            &input(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"GEX-1000":2,"GEX-1001":1}}"#
            )),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.placement.completion, Completion::Complete);
        assert!(report.placement.solutions.is_empty());
    }

    #[test]
    fn verification_without_a_program_proves_the_placement_alone() {
        let report = verify(
            &input(r#""electives":["gex-1000"],"pinned":{"GEX-1000":1}"#),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.coverage.is_none(), "no program, no rules to count");
    }

    #[test]
    fn verification_surfaces_every_placement_error() {
        // a session number the horizon does not have — an error, not a
        // « false », since the question itself is malformed
        let error = verify(
            &input(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"GEX-1000":1,"GEX-1001":9}}"#
            )),
            &courses(),
        )
        .expect_err("session 9 of a 3-session horizon");
        assert!(error.contains("outside 1..=3"), "{error}");
    }

    #[test]
    fn verification_surfaces_every_coverage_error() {
        // an unknown title now dies at the intake (the solver reads the
        // scope too), so the coverage half is proven by a counting error:
        // the two pinned courses sum 6 credits over the rule's max of 3
        let program = PROGRAM.replace(
            r#""rules":[]"#,
            r#""rules":[{"title":"Règle 1",
                         "constraint":{"type":"credits","min":3,"max":3},
                         "courses":["GEX-1000","GEX-1001"]}]"#,
        );
        let error = verify(
            &input(&format!(
                r#""program":{program},
                   "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
            )),
            &courses(),
        )
        .expect_err("6 credits over the max of 3");
        assert!(error.contains("above the max"), "{error}");
    }

    #[test]
    fn verification_surfaces_an_unknown_concentration() {
        let error = verify(
            &input(&format!(
                r#""program":{PROGRAM},"concentration":"Aucune",
                   "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
            )),
            &courses(),
        )
        .expect_err("no such concentration");
        assert!(error.contains("Aucune"), "{error}");
    }

    #[test]
    fn verification_surfaces_every_intake_error() {
        let error = verify(&input(r#""electives":["ZZZ-9999"]"#), &courses())
            .expect_err("a typed typo must not survive");
        assert!(error.contains("ZZZ-9999"), "{error}");
    }
}
