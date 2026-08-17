use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    admissible_sessions, horizon_sessions, place, placement_intake,
    summer_indices, Course, Placement, PlacementIntake, PlacementRequest,
    Program, Season,
};

// The worker speaks JSON strings both ways: one `Request` in, always one
// `Response` out — an unreadable request answers under the reserved id 0
// instead of vanishing. The snapshot never rides in a message: the worker
// holds it (see `boundary`), so a request carries only the student's state.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    // build the path: everything laid over the horizon, pins honoured
    Place {
        id: u64,
        query: PlaceQuery,
    },
    // prove the student's own organigramme — every course must be pinned
    // or passed; the rules are counted UI-side by `coverage_report` (ADR
    // `2026-08-verification-automatique-du-cheminement`)
    Verify {
        id: u64,
        query: PlaceQuery,
    },
    // the « + H28 » chips: which sessions could host `code` if pinned there
    AdmissibleSessions {
        id: u64,
        query: PlaceQuery,
        code: String,
    },
}

// `OrganigrammeInput` of the JS module, minus `courses` (held by the
// worker) and with explicit budgets — the UI always decides them.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceQuery {
    #[serde(default)]
    pub program: Option<Program>,
    #[serde(default)]
    pub electives: Vec<String>,
    #[serde(default)]
    pub passed: Vec<String>,
    // code → 1-based session number
    #[serde(default)]
    pub pinned: BTreeMap<String, usize>,
    pub start: Season,
    // the A/H alternation only — the étés come on top
    pub study_sessions: usize,
    pub credit_cap: u32,
    #[serde(default)]
    pub concomitant: bool,
    #[serde(default)]
    pub summers_open: bool,
    #[serde(default)]
    pub seed: BTreeMap<String, usize>,
    pub max_nodes: u64,
    pub max_solutions: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Response {
    Report {
        id: u64,
        report: Report,
    },
    Admissible {
        id: u64,
        code: String,
        sessions: BTreeSet<usize>,
    },
    Error {
        id: u64,
        message: String,
    },
}

// the `OrganigrammeReport` shape of the JS module: the horizon is returned
// with the placement, since the session numbers only mean something next
// to the seasons they index
#[derive(Debug, Clone, serde::Serialize)]
pub struct Report {
    pub sessions: Vec<Season>,
    pub placement: Placement,
    pub set_aside: Vec<String>,
    // electives the intake added because a candidate's prerequisites force
    // them — the UI adopts and announces them, never silently (ADR
    // `2026-08-injection-des-electifs-forces-par-les-prealables`)
    pub injected: Vec<String>,
}

pub fn handle(request: &str, courses: &[Course]) -> String {
    let response = match serde_json::from_str::<Request>(request) {
        Err(error) => Response::Error {
            id: 0,
            message: format!("unreadable request : {error}"),
        },
        Ok(Request::Place { id, query }) => match generate(&query, courses) {
            Ok(report) => Response::Report { id, report },
            Err(message) => Response::Error { id, message },
        },
        Ok(Request::Verify { id, query }) => match verify(&query, courses) {
            Ok(report) => Response::Report { id, report },
            Err(message) => Response::Error { id, message },
        },
        Ok(Request::AdmissibleSessions { id, query, code }) => {
            match admissible(&query, courses, &code) {
                Ok(sessions) => Response::Admissible { id, code, sessions },
                Err(message) => Response::Error { id, message },
            }
        }
    };
    // expect over `?`: serializing strings, maps and vecs provably
    // cannot fail
    serde_json::to_string(&response)
        .expect("Response serialization always succeeds")
}

fn generate(query: &PlaceQuery, courses: &[Course]) -> Result<Report, String> {
    let intake = intake(query, courses)?;
    let sessions = horizon_sessions(query.start, query.study_sessions);
    let placement = solve(query, &intake, &sessions)?;
    Ok(Report {
        sessions,
        placement,
        set_aside: intake.set_aside,
        injected: intake.injected,
    })
}

// `place` with everything pinned proves the placement; a course left
// without a session is an incomplete question, never a false verdict. The
// rules are counted UI-side (`coverage_report` over the same selection).
fn verify(query: &PlaceQuery, courses: &[Course]) -> Result<Report, String> {
    let intake = intake(query, courses)?;
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
    let sessions = horizon_sessions(query.start, query.study_sessions);
    let placement = solve(query, &intake, &sessions)?;
    Ok(Report {
        sessions,
        placement,
        set_aside: intake.set_aside,
        injected: intake.injected,
    })
}

fn admissible(
    query: &PlaceQuery,
    courses: &[Course],
    code: &str,
) -> Result<BTreeSet<usize>, String> {
    let intake = intake(query, courses)?;
    let sessions = horizon_sessions(query.start, query.study_sessions);
    let open_summers = open_summers(query, &sessions);
    admissible_sessions(
        &PlacementRequest {
            sessions: &sessions,
            credit_cap: query.credit_cap,
            concomitant: query.concomitant,
            courses: &intake.courses,
            passed: &intake.passed,
            pinned: &intake.pinned,
            stages: &intake.stages,
            open_summers: &open_summers,
            seed: &query.seed,
            max_nodes: query.max_nodes,
            max_solutions: query.max_solutions,
        },
        // intake uppercases every code; the probe must ask about the same
        &code.to_uppercase(),
    )
    .map_err(|e| e.to_string())
}

fn intake(
    query: &PlaceQuery,
    courses: &[Course],
) -> Result<PlacementIntake, String> {
    // ponytail: `placement_intake` speaks the `CODE=SESSION` shape, so one
    // format! reuses its whole validation instead of a second parser
    let pins: Vec<String> = query
        .pinned
        .iter()
        .map(|(code, session)| format!("{code}={session}"))
        .collect();
    placement_intake(
        query.program.as_ref(),
        &query.electives,
        &query.passed,
        &pins,
        courses,
    )
    .map_err(|e| e.to_string())
}

fn solve(
    query: &PlaceQuery,
    intake: &PlacementIntake,
    sessions: &[Season],
) -> Result<Placement, String> {
    let open_summers = open_summers(query, sessions);
    place(&PlacementRequest {
        sessions,
        credit_cap: query.credit_cap,
        concomitant: query.concomitant,
        courses: &intake.courses,
        passed: &intake.passed,
        pinned: &intake.pinned,
        stages: &intake.stages,
        open_summers: &open_summers,
        seed: &query.seed,
        max_nodes: query.max_nodes,
        max_solutions: query.max_solutions,
    })
    .map_err(|e| e.to_string())
}

// an été absent from the set still hosts stages and pinned courses (ADR
// `2026-08-stage-place-en-ete-sauf-epinglage`)
fn open_summers(query: &PlaceQuery, sessions: &[Season]) -> BTreeSet<usize> {
    if query.summers_open {
        summer_indices(sessions)
    } else {
        BTreeSet::new()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    use ulaval_scheduler_core::Completion;

    // GEX-1001 requires GEX-1000; both offered fall and winter
    const COURSES: &str = r#"[
        {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
         "prerequisites":null,"equivalents":[],
         "seasons":{"fall":{"last_offered":2026,"options":null},
                    "winter":{"last_offered":2026,"options":null}}},
        {"code":"GEX-1001","title":"T","credits":3,"cycle":1,
         "prerequisites":{"raw":"GEX-1000","tree":"GEX-1000"},
         "equivalents":[],
         "seasons":{"fall":{"last_offered":2026,"options":null},
                    "winter":{"last_offered":2026,"options":null}}}
    ]"#;

    const PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
        "title":"P","cycle":1,"credits_required":6,
        "mandatory":["GEX-1000","GEX-1001"],
        "rules":[],"concentrations":[],"profiles":[]}"#;

    fn courses() -> Vec<Course> {
        serde_json::from_str(COURSES)
            .unwrap_or_else(|e| panic!("courses literal: {e}"))
    }

    fn query(fields: &str) -> String {
        format!(
            r#"{{"start":"fall","study_sessions":2,"credit_cap":6,
                 "max_nodes":100000,"max_solutions":10,{fields}}}"#
        )
    }

    fn parsed(response: &str) -> serde_json::Value {
        serde_json::from_str(response)
            .unwrap_or_else(|e| panic!("response literal: {e}"))
    }

    #[test]
    fn a_place_request_answers_with_the_report_under_its_id() {
        let request = format!(
            r#"{{"kind":"place","id":7,
                 "query":{}}}"#,
            query(&format!(r#""program":{PROGRAM}"#))
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "report");
        assert_eq!(response["id"], 7);
        assert_eq!(response["report"]["placement"]["completion"], "complete");
        assert_eq!(
            response["report"]["sessions"],
            serde_json::json!(["fall", "winter", "summer"])
        );
        assert_eq!(
            response["report"]["placement"]["solutions"][0]["placement"]
                ["GEX-1001"],
            2
        );
        assert!(
            response["report"].get("coverage").is_none(),
            "generation counts no rule"
        );
    }

    #[test]
    fn a_verify_request_proves_the_fully_pinned_path() {
        let request = format!(
            r#"{{"kind":"verify","id":8,
                 "query":{}}}"#,
            query(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
            ))
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "report");
        assert_eq!(response["report"]["placement"]["completion"], "complete");
        assert_eq!(
            response["report"]["placement"]["solutions"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn a_verify_request_with_an_unplaced_course_is_the_incomplete_error() {
        let request = format!(
            r#"{{"kind":"verify","id":9,
                 "query":{}}}"#,
            query(&format!(
                r#""program":{PROGRAM},"pinned":{{"GEX-1000":1}}"#
            ))
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "error");
        assert_eq!(response["id"], 9);
        let message = response["message"].as_str().unwrap_or_default();
        assert!(message.contains("GEX-1001"), "{message}");
    }

    #[test]
    fn an_admissible_request_answers_the_chip_probe_lowercase_included() {
        let request = format!(
            r#"{{"kind":"admissible-sessions","id":10,"code":"gex-1001",
                 "query":{}}}"#,
            query(&format!(
                r#""program":{PROGRAM},"pinned":{{"GEX-1000":1}}"#
            ))
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "admissible");
        assert_eq!(response["code"], "gex-1001");
        // session 1 is barred by the prerequisite, the été by the offer
        assert_eq!(response["sessions"], serde_json::json!([2]));
    }

    #[test]
    fn an_admissible_request_surfaces_intake_errors() {
        let request = format!(
            r#"{{"kind":"admissible-sessions","id":11,"code":"GEX-1001",
                 "query":{}}}"#,
            query(r#""electives":["ZZZ-9999"]"#)
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "error");
        let message = response["message"].as_str().unwrap_or_default();
        assert!(message.contains("ZZZ-9999"), "{message}");
    }

    #[test]
    fn a_place_request_surfaces_intake_and_placement_errors() {
        let intake_error = parsed(&handle(
            &format!(
                r#"{{"kind":"place","id":1,"query":{}}}"#,
                query(r#""electives":["ZZZ-9999"]"#)
            ),
            &courses(),
        ));
        assert_eq!(intake_error["kind"], "error");

        let placement_error = parsed(&handle(
            &format!(
                r#"{{"kind":"place","id":2,"query":{}}}"#,
                query(r#""electives":[]"#)
            ),
            &courses(),
        ));
        assert_eq!(placement_error["kind"], "error");
        let message = placement_error["message"].as_str().unwrap_or_default();
        assert!(message.contains("at least one session"), "{message}");
    }

    #[test]
    fn a_verify_request_surfaces_intake_and_placement_errors() {
        let intake_error = parsed(&handle(
            &format!(
                r#"{{"kind":"verify","id":14,"query":{}}}"#,
                query(r#""electives":["ZZZ-9999"]"#)
            ),
            &courses(),
        ));
        assert_eq!(intake_error["kind"], "error");

        // a session the horizon does not have: malformed question, not a
        // « false »
        let placement_error = parsed(&handle(
            &format!(
                r#"{{"kind":"verify","id":15,"query":{}}}"#,
                query(&format!(
                    r#""program":{PROGRAM},
                       "pinned":{{"GEX-1000":1,"GEX-1001":9}}"#
                ))
            ),
            &courses(),
        ));
        assert_eq!(placement_error["kind"], "error");
        let message = placement_error["message"].as_str().unwrap_or_default();
        assert!(message.contains("outside 1..=3"), "{message}");
    }

    #[test]
    fn an_admissible_request_surfaces_placement_errors() {
        // probing a passed course is the typed error, carried through
        let request = format!(
            r#"{{"kind":"admissible-sessions","id":16,"code":"GEX-1000",
                 "query":{}}}"#,
            query(r#""electives":["gex-1000"],"passed":["GEX-1000"]"#)
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["kind"], "error");
        let message = response["message"].as_str().unwrap_or_default();
        assert!(message.contains("passed and pinned"), "{message}");
    }

    #[test]
    fn a_closed_ete_blocks_a_summer_only_course_an_open_one_hosts_it() {
        let summer_only: Vec<Course> = serde_json::from_str(
            r#"[{"code":"GEX-1002","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{"summer":{"last_offered":2026,
                                      "options":null}}}]"#,
        )
        .unwrap_or_else(|e| panic!("courses literal: {e}"));

        let closed = parsed(&handle(
            &format!(
                r#"{{"kind":"place","id":4,"query":{}}}"#,
                query(r#""electives":["gex-1002"]"#)
            ),
            &summer_only,
        ));
        assert_eq!(
            closed["report"]["placement"]["blocked"][0]["code"], "GEX-1002",
            "the été is closed to regular courses"
        );

        let open = parsed(&handle(
            &format!(
                r#"{{"kind":"place","id":5,"query":{}}}"#,
                query(r#""electives":["gex-1002"],"summers_open":true"#)
            ),
            &summer_only,
        ));
        assert_eq!(
            open["report"]["placement"]["solutions"][0]["placement"]
                ["GEX-1002"],
            3
        );
    }

    #[test]
    fn an_unreadable_request_answers_under_the_reserved_id() {
        let response = parsed(&handle("not json at all", &courses()));
        assert_eq!(response["kind"], "error");
        assert_eq!(response["id"], 0);
        let message = response["message"].as_str().unwrap_or_default();
        assert!(message.contains("unreadable request"), "{message}");
    }

    #[test]
    fn a_verified_placement_that_breaks_a_prerequisite_is_a_complete_proof() {
        let request = format!(
            r#"{{"kind":"verify","id":13,
                 "query":{}}}"#,
            query(&format!(
                r#""program":{PROGRAM},
                   "pinned":{{"GEX-1000":2,"GEX-1001":1}}"#
            ))
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(response["report"]["placement"]["completion"], "complete");
        assert_eq!(
            response["report"]["placement"]["solutions"],
            serde_json::json!([])
        );
    }

    #[test]
    fn the_report_reuses_completion_verbatim() {
        // a zero budget through the whole pipe: the truncation is named
        let request = format!(
            r#"{{"kind":"place","id":3,
                 "query":{{"start":"fall","study_sessions":2,
                           "credit_cap":6,"max_nodes":0,
                           "max_solutions":10,"program":{PROGRAM}}}}}"#
        );
        let response = parsed(&handle(&request, &courses()));
        assert_eq!(
            response["report"]["placement"]["completion"],
            serde_json::json!(Completion::NodeBudget)
        );
    }
}
