use ulaval_scheduler_core::Course;

use crate::organigramme::{self, OrganigrammeInput, OrganigrammeReport};

// The Dioxus app's worker speaks JSON strings both ways: one `Request` in,
// always one `Response` out — an unreadable request answers under the
// reserved id 0 instead of vanishing. The snapshot never rides in a message:
// the worker holds it (see `boundary`), so a request carries only the
// student's state, and `query` is the very `OrganigrammeInput` the
// JavaScript interface sends, minus its `courses` (ADR
// `2026-08-fusion-des-crates-wasm-et-ui-calculations`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    // build the path: everything laid over the horizon, pins honoured
    Place {
        id: u64,
        query: OrganigrammeInput,
    },
    // prove the student's own organigramme — every course must be pinned
    // or passed
    Verify {
        id: u64,
        query: OrganigrammeInput,
    },
    // the « + H28 » chips: which sessions could host `code` if pinned there
    AdmissibleSessions {
        id: u64,
        query: OrganigrammeInput,
        code: String,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Response {
    Report {
        id: u64,
        report: OrganigrammeReport,
    },
    Admissible {
        id: u64,
        code: String,
        sessions: Vec<usize>,
    },
    Error {
        id: u64,
        message: String,
    },
}

pub fn handle(request: &str, courses: &[Course]) -> String {
    let response = match serde_json::from_str::<Request>(request) {
        Err(error) => return refusal(format!("unreadable request : {error}")),
        Ok(Request::Place { id, query }) => {
            reported(id, organigramme::generate(&query, courses))
        }
        Ok(Request::Verify { id, query }) => {
            reported(id, organigramme::verify(&query, courses))
        }
        Ok(Request::AdmissibleSessions { id, query, code }) => {
            match organigramme::admissible(&query, courses, &code) {
                Ok(sessions) => Response::Admissible { id, code, sessions },
                Err(message) => Response::Error { id, message },
            }
        }
    };
    serialized(response)
}

// A refusal that no request owns: an unreadable message, or a worker asked
// to compute before it received its snapshot. Answered under the reserved
// id 0 — never silence, and never a verdict computed on nothing.
pub fn refusal(message: String) -> String {
    serialized(Response::Error { id: 0, message })
}

fn reported(id: u64, result: Result<OrganigrammeReport, String>) -> Response {
    match result {
        Ok(report) => Response::Report { id, report },
        Err(message) => Response::Error { id, message },
    }
}

fn serialized(response: Response) -> String {
    // expect over `?`: serializing strings, maps and vecs provably
    // cannot fail
    serde_json::to_string(&response)
        .expect("Response serialization always succeeds")
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // The placement behaviours themselves are proven in `organigramme`;
    // what is left to prove here is the envelope — that every answer comes
    // back under the id that asked for it, and that nothing vanishes.

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

    // the request shape the Dioxus worker actually sends: no `courses`, and
    // explicit budgets — the UI always decides them
    fn request(kind: &str, id: u64, fields: &str) -> String {
        format!(
            r#"{{"kind":"{kind}","id":{id},
                 "query":{{"start":"fall","study_sessions":2,"credit_cap":6,
                           "max_nodes":100000,"max_solutions":10,{fields}}}}}"#
        )
    }

    fn parsed(response: &str) -> serde_json::Value {
        serde_json::from_str(response)
            .unwrap_or_else(|e| panic!("response literal: {e}"))
    }

    #[test]
    fn a_place_request_answers_with_the_report_under_its_id() {
        let response = parsed(&handle(
            &request("place", 7, &format!(r#""program":{PROGRAM}"#)),
            &courses(),
        ));
        assert_eq!(response["kind"], "report");
        assert_eq!(response["id"], 7);
        assert_eq!(response["report"]["placement"]["completion"], "complete");
        assert_eq!(
            response["report"]["sessions"],
            serde_json::json!(["fall", "winter", "summer"])
        );
    }

    #[test]
    fn a_verify_request_answers_with_the_report_under_its_id() {
        let response = parsed(&handle(
            &request(
                "verify",
                8,
                &format!(
                    r#""program":{PROGRAM},
                       "pinned":{{"GEX-1000":1,"GEX-1001":2}}"#
                ),
            ),
            &courses(),
        ));
        assert_eq!(response["kind"], "report");
        assert_eq!(response["id"], 8);
        assert_eq!(
            response["report"]["placement"]["solutions"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn an_admissible_request_answers_the_probe_under_its_id_and_sigle() {
        let response = parsed(&handle(
            &format!(
                r#"{{"kind":"admissible-sessions","id":10,"code":"gex-1001",
                     "query":{{"start":"fall","study_sessions":2,
                               "credit_cap":6,"max_nodes":100000,
                               "max_solutions":10,"program":{PROGRAM},
                               "pinned":{{"GEX-1000":1}}}}}}"#
            ),
            &courses(),
        ));
        assert_eq!(response["kind"], "admissible");
        assert_eq!(response["id"], 10);
        // the sigle comes back as it was asked, lowercase included
        assert_eq!(response["code"], "gex-1001");
        // session 1 is barred by the prerequisite, the été by the offer
        assert_eq!(response["sessions"], serde_json::json!([2]));
    }

    #[test]
    fn a_refused_question_becomes_an_error_under_the_id_that_asked_it() {
        // one per arm: what the solver refuses must not turn into a verdict
        let place = parsed(&handle(
            &request("place", 11, r#""electives":["ZZZ-9999"]"#),
            &courses(),
        ));
        assert_eq!(place["kind"], "error");
        assert_eq!(place["id"], 11);
        assert!(
            place["message"]
                .as_str()
                .unwrap_or_default()
                .contains("ZZZ-9999"),
            "{place}"
        );

        // verify with a course left unplaced: incomplete question, not false
        let verify = parsed(&handle(
            &request(
                "verify",
                12,
                &format!(r#""program":{PROGRAM},"pinned":{{"GEX-1000":1}}"#),
            ),
            &courses(),
        ));
        assert_eq!(verify["kind"], "error");
        assert_eq!(verify["id"], 12);
        assert!(
            verify["message"]
                .as_str()
                .unwrap_or_default()
                .contains("GEX-1001"),
            "{verify}"
        );

        let admissible = parsed(&handle(
            r#"{"kind":"admissible-sessions","id":13,"code":"Z",
                "query":{"start":"fall","study_sessions":2,
                         "credit_cap":6,"max_nodes":100000,
                         "max_solutions":10,
                         "electives":["ZZZ-9999"]}}"#,
            &courses(),
        ));
        assert_eq!(admissible["kind"], "error");
        assert_eq!(admissible["id"], 13);
    }

    #[test]
    fn an_unreadable_request_answers_under_the_reserved_id() {
        let response = parsed(&handle("not json at all", &courses()));
        assert_eq!(response["kind"], "error");
        assert_eq!(response["id"], 0);
        let message = response["message"].as_str().unwrap_or_default();
        assert!(message.contains("unreadable request"), "{message}");
    }

    // what the boundary answers when the worker has no snapshot yet
    #[test]
    fn a_refusal_that_no_request_owns_takes_the_reserved_id_too() {
        let response = parsed(&refusal("no catalogue".to_string()));
        assert_eq!(response["kind"], "error");
        assert_eq!(response["id"], 0);
        assert_eq!(response["message"], "no catalogue");
    }
}
