use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    schedule_intake, schedule_report, Course, ScheduleReport,
};

// What JS hands the two weekly functions: the snapshot it loaded, the
// session, the codes the student typed, and the options he pinned — one
// sorted NRC set per course, an option having no identifier of its own (ADR
// `2026-07-contrat-horaire-hebdomadaire-vers-ui`). Unknown fields are
// refused rather than ignored: a typo in the JS object must not be read as
// a default (« never lose input silently »).
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(target_arch = "wasm32", derive(tsify::Tsify))]
#[serde(deny_unknown_fields)]
pub struct ScheduleInput {
    pub courses: Vec<Course>,
    // `a2026`, `h2027`, `e2026`
    pub session: String,
    pub codes: Vec<String>,
    #[serde(default)]
    #[cfg_attr(
        target_arch = "wasm32",
        tsify(type = "Record<string, string[]>")
    )]
    pub chosen: BTreeMap<String, BTreeSet<String>>,
}

// Build the week: `chosen` pins whatever the student has already settled —
// possibly nothing — and every other course takes the first option of a
// conflict-free combination.
pub fn generate(input: &ScheduleInput) -> Result<ScheduleReport, String> {
    let intake = schedule_intake(&input.courses, &input.session, &input.codes)
        .map_err(|e| e.to_string())?;
    schedule_report(&intake.courses, intake.season, &normalized(input))
        .map_err(|e| e.to_string())
}

// Verify the week the student assembled: every requested course must carry
// its chosen option, so the report's `valid` flags judge *his* combination
// instead of one the solver picked for him. A missing pin is an incomplete
// question, not a false verdict.
pub fn verify(input: &ScheduleInput) -> Result<ScheduleReport, String> {
    let chosen = normalized(input);
    let unpinned: Vec<String> = input
        .codes
        .iter()
        .map(|code| code.to_uppercase())
        .filter(|code| !chosen.contains_key(code))
        .collect();
    if !unpinned.is_empty() {
        return Err(format!(
            "verification needs a chosen option for every course : {}",
            unpinned.join(", ")
        ));
    }
    generate(input)
}

// codes are uppercased on the way in (`normalize_codes`), so the pins must
// be too — otherwise a lowercase key names no requested course
fn normalized(input: &ScheduleInput) -> BTreeMap<String, BTreeSet<String>> {
    input
        .chosen
        .iter()
        .map(|(code, nrcs)| (code.to_uppercase(), nrcs.clone()))
        .collect()
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // two courses, one in-person option each, offered in fall
    fn input(session: &str, codes: &[&str], chosen: &str) -> ScheduleInput {
        serde_json::from_str(&format!(
            r#"{{"session":"{session}",
                 "codes":{},
                 "chosen":{chosen},
                 "courses":[{},{}]}}"#,
            serde_json::to_string(codes)
                .unwrap_or_else(|e| panic!("codes: {e}")),
            course("GEX-1000", "monday", "90001"),
            course("GEX-1001", "tuesday", "90002"),
        ))
        .unwrap_or_else(|e| panic!("input literal: {e}"))
    }

    fn course(code: &str, day: &str, nrc: &str) -> String {
        format!(
            r#"{{"code":"{code}","title":"T","credits":3,"cycle":1,
                 "prerequisites":null,"equivalents":[],
                 "seasons":{{"fall":{{"last_offered":2026,
                   "options":[[{{"nrc":"{nrc}","section":"A",
                     "mode":"in-person","slots":[{{"day":"{day}",
                       "start":"08:30","end":"11:20"}}]}}]]}}}}}}"#
        )
    }

    #[test]
    fn generation_lowercases_nothing_and_reports_a_valid_week() {
        let report =
            generate(&input("a2026", &["gex-1000", "GEX-1001"], "{}"))
                .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.valid);
        let codes: Vec<&str> = report
            .courses
            .iter()
            .map(|course| course.code.as_str())
            .collect();
        assert_eq!(codes, ["GEX-1000", "GEX-1001"]);
    }

    #[test]
    fn generation_surfaces_every_intake_error() {
        let error = generate(&input("x2026", &["gex-1000"], "{}"))
            .expect_err("no such season letter");
        assert!(error.contains("a<year>"), "{error}");
    }

    #[test]
    fn generation_surfaces_every_report_error() {
        // a pin naming a course that was not requested
        let error = generate(&input(
            "a2026",
            &["gex-1000"],
            r#"{"gex-1001":["90002"]}"#,
        ))
        .expect_err("pinned but not requested");
        assert!(error.contains("GEX-1001"), "{error}");
    }

    #[test]
    fn verification_needs_a_chosen_option_for_every_course() {
        let error = verify(&input(
            "a2026",
            &["gex-1000", "gex-1001"],
            r#"{"gex-1000":["90001"]}"#,
        ))
        .expect_err("GEX-1001 has no pin");
        assert!(error.contains("GEX-1001"), "{error}");
    }

    #[test]
    fn verification_judges_the_students_own_combination() {
        // pins uppercase the way the codes do, so a lowercase key still
        // names its course
        let report = verify(&input(
            "a2026",
            &["gex-1000", "gex-1001"],
            r#"{"gex-1000":["90001"],"GEX-1001":["90002"]}"#,
        ))
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.valid);
    }
}
