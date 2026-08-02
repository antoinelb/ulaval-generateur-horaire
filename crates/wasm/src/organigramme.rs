use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    coverage_report, horizon_sessions, place, placement_intake,
    summer_indices, Course, CoverageReport, Placement, PlacementIntake,
    PlacementRequest, Program, Season,
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
#[serde(deny_unknown_fields)]
pub struct OrganigrammeInput {
    pub courses: Vec<Course>,
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
    pub seed: BTreeMap<String, usize>,
    #[serde(default)]
    pub max_nodes: Option<u64>,
    #[serde(default)]
    pub max_solutions: Option<usize>,
}

// The horizon is returned with the placement: the session numbers only mean
// something next to the seasons they index, and JS never computed them.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrganigrammeReport {
    pub sessions: Vec<Season>,
    pub placement: Placement,
    // program-derived codes the snapshot does not carry — surfaced, never
    // dropped (ADR `2026-07-cours-sans-offre-ecarte-par-le-harnais`)
    pub set_aside: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage: Option<CoverageReport>,
}

// Build the path: every course of the program (plus the chosen electives)
// laid over the horizon, `pinned` fixing whatever the student already
// settled.
pub fn generate(
    input: &OrganigrammeInput,
) -> Result<OrganigrammeReport, String> {
    let intake = intake(input)?;
    let sessions = horizon_sessions(input.start, input.study_sessions);
    let placement = solve(input, &intake, &sessions)?;
    Ok(OrganigrammeReport {
        sessions,
        placement,
        set_aside: intake.set_aside,
        coverage: None,
    })
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
) -> Result<OrganigrammeReport, String> {
    let intake = intake(input)?;
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
        coverage,
    })
}

fn intake(input: &OrganigrammeInput) -> Result<PlacementIntake, String> {
    // ponytail: `placement_intake` speaks the `CODE=SESSION` shape, so one
    // format! reuses its whole validation instead of a second parser
    let pins: Vec<String> = input
        .pinned
        .iter()
        .map(|(code, session)| format!("{code}={session}"))
        .collect();
    placement_intake(
        input.program.as_ref(),
        &input.electives,
        &input.passed,
        &pins,
        &input.courses,
    )
    .map_err(|e| e.to_string())
}

fn solve(
    input: &OrganigrammeInput,
    intake: &PlacementIntake,
    sessions: &[Season],
) -> Result<Placement, String> {
    // an été absent from the set still hosts stages and pinned courses (ADR
    // `2026-08-stage-place-en-ete-sauf-epinglage`)
    let open_summers = if input.summers_open {
        summer_indices(sessions)
    } else {
        BTreeSet::new()
    };
    place(&PlacementRequest {
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

    fn input(fields: &str) -> OrganigrammeInput {
        serde_json::from_str(&format!(
            r#"{{"courses":{COURSES},"start":"fall","study_sessions":2,
                 "credit_cap":6,{fields}}}"#
        ))
        .unwrap_or_else(|e| panic!("input literal: {e}"))
    }

    #[test]
    fn generation_lays_the_program_over_the_horizon() {
        let report = generate(&input(&format!(r#""program":{PROGRAM}"#)))
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
    fn generation_sets_aside_the_codes_the_snapshot_does_not_carry() {
        let program = PROGRAM
            .replace(r#""GEX-1000","GEX-1001""#, r#""GEX-1000","GHOST-999""#);
        let report = generate(&input(&format!(r#""program":{program}"#)))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.set_aside, ["GHOST-999"]);
    }

    #[test]
    fn a_closed_ete_blocks_a_summer_only_course_an_open_one_hosts_it() {
        let closed = generate(&input(r#""electives":["gex-1002"]"#))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            closed.placement.blocked[0].code, "GEX-1002",
            "the été is closed to regular courses"
        );

        let open = generate(&input(
            r#""electives":["gex-1002"],"summers_open":true"#,
        ))
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(open.placement.blocked.is_empty());
        assert_eq!(open.placement.solutions[0].placement["GEX-1002"], 3);
    }

    // the JSON names are what JS reads off the returned object
    #[test]
    fn the_report_serializes_under_its_published_names() {
        let generated = generate(&input(&format!(r#""program":{PROGRAM}"#)))
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

        let verified = verify(&input(&format!(
            r#""program":{PROGRAM},
               "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
        )))
        .unwrap_or_else(|e| panic!("{e}"));
        let json = serde_json::to_value(&verified)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        assert_eq!(json["coverage"]["mandatory"][0]["scope"], "program");
    }

    #[test]
    fn generation_surfaces_every_intake_error() {
        let error = generate(&input(r#""electives":["ZZZ-9999"]"#))
            .expect_err("a typed typo must not survive");
        assert!(error.contains("ZZZ-9999"), "{error}");
    }

    #[test]
    fn generation_surfaces_every_placement_error() {
        let error = generate(&input(r#""electives":[]"#))
            .expect_err("no course to place");
        assert!(error.contains("at least one session"), "{error}");
    }

    #[test]
    fn verification_needs_a_session_for_every_course_left_to_place() {
        let error = verify(&input(&format!(
            r#""program":{PROGRAM},"pinned":{{"GEX-1000":1}}"#
        )))
        .expect_err("GEX-1001 has no session");
        assert!(error.contains("GEX-1001"), "{error}");
    }

    #[test]
    fn verification_proves_the_students_own_path_and_counts_the_rules() {
        let report = verify(&input(&format!(
            r#""program":{PROGRAM},
               "pinned":{{"gex-1000":1,"gex-1001":2}}"#
        )))
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
        let report = verify(&input(&format!(
            r#""program":{PROGRAM},
               "pinned":{{"GEX-1000":2,"GEX-1001":1}}"#
        )))
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.placement.completion, Completion::Complete);
        assert!(report.placement.solutions.is_empty());
    }

    #[test]
    fn verification_without_a_program_proves_the_placement_alone() {
        let report = verify(&input(
            r#""electives":["gex-1000"],"pinned":{"GEX-1000":1}"#,
        ))
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.coverage.is_none(), "no program, no rules to count");
    }

    #[test]
    fn verification_surfaces_every_placement_error() {
        // a session number the horizon does not have — an error, not a
        // « false », since the question itself is malformed
        let error = verify(&input(&format!(
            r#""program":{PROGRAM},
               "pinned":{{"GEX-1000":1,"GEX-1001":9}}"#
        )))
        .expect_err("session 9 of a 3-session horizon");
        assert!(error.contains("outside 1..=3"), "{error}");
    }

    #[test]
    fn verification_surfaces_every_coverage_error() {
        let error = verify(&input(&format!(
            r#""program":{PROGRAM},"concentration":"Aucune",
               "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
        )))
        .expect_err("no such concentration");
        assert!(error.contains("Aucune"), "{error}");
    }

    #[test]
    fn verification_surfaces_every_intake_error() {
        let error = verify(&input(r#""electives":["ZZZ-9999"]"#))
            .expect_err("a typed typo must not survive");
        assert!(error.contains("ZZZ-9999"), "{error}");
    }
}
