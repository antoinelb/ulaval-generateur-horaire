use std::collections::BTreeSet;

use ulaval_scheduler_core::{
    horizon_sessions, normalize_codes, session_semesters, Course,
    CoverageReport, PrereqStatus, Program, Semester,
};

// The request-free half of the frozen JavaScript surface (ADRs
// `2026-08-surface-wasm-etendue-a-huit-fonctions`,
// `2026-08-surface-javascript-plus-une-contrainte`): static questions asked
// per row or per grid, with no placement search behind them. The app itself
// asks core directly.

// What `prerequisites_met` takes. Unknown fields are refused rather than
// ignored, like every input of the crate.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
#[serde(deny_unknown_fields)]
pub struct PrerequisitesInput {
    pub course: Course,
    // passed and placed-before codes — never a hypothetical future placement
    pub satisfied: Vec<String>,
    // what sits in the very session being judged: it satisfies a leaf the
    // répertoire starred (« GCI-2010* », concomitance permise) and nothing
    // else. Absent = strict precedence, the reading before the star existed.
    #[serde(default)]
    pub same_session: Vec<String>,
    // the credits accumulated before the course's session
    pub credits: u32,
}

// `PrereqStatus` flattened for the wire: `met` plus the operands the
// verdict had to presume (raw text, préuniversitaire codes) — surfaced,
// never imposed.
#[derive(Debug, PartialEq, serde::Serialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
pub struct PrerequisitesReport {
    pub met: bool,
    pub assumed: BTreeSet<String>,
}

pub fn prerequisites(
    input: &PrerequisitesInput,
) -> Result<PrerequisitesReport, String> {
    let satisfied: BTreeSet<String> = normalize_codes(&input.satisfied)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let same_session: BTreeSet<String> = normalize_codes(&input.same_session)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    let status = ulaval_scheduler_core::prerequisites_met(
        &input.course,
        &satisfied,
        &same_session,
        input.credits,
    )
    .map_err(|e| e.to_string())?;
    Ok(match status {
        PrereqStatus::Met { assumed } => {
            PrerequisitesReport { met: true, assumed }
        }
        PrereqStatus::Unmet => PrerequisitesReport {
            met: false,
            assumed: BTreeSet::new(),
        },
    })
}

// What `coverage_report` takes: the rules question alone, on whatever
// partial grid the student has — `verify_organigramme` keeps demanding a
// complete placement, this does not.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
#[serde(deny_unknown_fields)]
pub struct CoverageInput {
    pub program: Program,
    #[serde(default)]
    pub concentration: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    // every code the student counts on: passed, placed, granted
    pub selection: Vec<String>,
    // Wire format only, resolved by the boundary — see `OrganigrammeInput`.
    #[serde(default)]
    pub courses: Option<Vec<Course>>,
}

pub fn coverage(
    input: &CoverageInput,
    courses: &[Course],
) -> Result<CoverageReport, String> {
    let selection: BTreeSet<String> = normalize_codes(&input.selection)
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    ulaval_scheduler_core::coverage_report(
        &input.program,
        input.concentration.as_deref(),
        input.profile.as_deref(),
        &selection,
        courses,
    )
    .map_err(|e| e.to_string())
}

// What `horizon_sessions` takes: the described horizon, answered as the
// semester codes (« A26 », « H27 », « E27 », …) so the été-after-each-hiver
// rule and the calendar arithmetic both stay out of the view.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg_attr(
    all(target_arch = "wasm32", feature = "boundary"),
    derive(tsify::Tsify)
)]
#[serde(deny_unknown_fields)]
pub struct HorizonInput {
    pub start: Semester,
    // the A/H alternation only — the étés come on top
    pub study_sessions: usize,
}

pub fn horizon(input: &HorizonInput) -> Result<Vec<Semester>, String> {
    let seasons = horizon_sessions(input.start.season, input.study_sessions);
    Ok(session_semesters(input.start, &seasons))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    const COURSE: &str = r#"{
        "code":"GEX-1001","title":"T","credits":3,"cycle":1,
        "prerequisites":{"raw":"GEX-1000 ET MAT-0130",
                         "tree":{"all":["GEX-1000","MAT-0130"]}},
        "equivalents":[],
        "seasons":{"fall":{"last_offered":2026,"options":null}}
    }"#;

    fn prerequisites_input(fields: &str) -> PrerequisitesInput {
        serde_json::from_str(&format!(r#"{{"course":{COURSE},{fields}}}"#))
            .unwrap_or_else(|e| panic!("input literal: {e}"))
    }

    #[test]
    fn prerequisites_answer_met_with_the_presumed_operands() {
        // GEX-1000 is held; préuniversitaire MAT-0130 is presumed, surfaced
        let report = prerequisites(&prerequisites_input(
            r#""satisfied":["gex-1000"],"credits":0"#,
        ))
        .unwrap_or_else(|e| panic!("{e}"));
        assert!(report.met);
        assert_eq!(report.assumed, BTreeSet::from(["MAT-0130".to_string()]));
    }

    #[test]
    fn prerequisites_answer_unmet_with_nothing_presumed() {
        let report = prerequisites(&prerequisites_input(
            r#""satisfied":[],"credits":0"#,
        ))
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            report,
            PrerequisitesReport {
                met: false,
                assumed: BTreeSet::new()
            }
        );
    }

    #[test]
    fn prerequisites_surface_a_tree_the_solver_refuses() {
        use ulaval_scheduler_core::{PrereqTree, Prerequisites};
        // one node over the solver's tree budget: the refusal rides up
        let mut input = prerequisites_input(r#""satisfied":[],"credits":0"#);
        input.course.prerequisites = Some(Prerequisites::Parsed {
            raw: "r".to_string(),
            tree: PrereqTree::All {
                all: (0..10_000)
                    .map(|_| PrereqTree::Course("GEX-1000".to_string()))
                    .collect(),
            },
        });
        let error = prerequisites(&input)
            .expect_err("an over-budget tree is refused, never guessed");
        assert!(error.contains("GEX-1001"), "{error}");
    }

    #[test]
    fn prerequisites_read_the_same_session_only_for_a_starred_leaf() {
        use ulaval_scheduler_core::{PrereqTree, Prerequisites};
        let mut input = prerequisites_input(
            r#""satisfied":[],"same_session":["gex-1000"],"credits":0"#,
        );
        input.course.prerequisites = Some(Prerequisites::Parsed {
            raw: "GEX-1000*".to_string(),
            tree: PrereqTree::Concomitant {
                concomitant: "GEX-1000".to_string(),
            },
        });
        assert!(prerequisites(&input).unwrap_or_else(|e| panic!("{e}")).met);
        // the same code, unstarred, still demands a session before
        input.course.prerequisites = Some(Prerequisites::Parsed {
            raw: "GEX-1000".to_string(),
            tree: PrereqTree::Course("GEX-1000".to_string()),
        });
        assert!(!prerequisites(&input).unwrap_or_else(|e| panic!("{e}")).met);
    }

    #[test]
    fn prerequisites_surface_a_duplicated_same_session_code() {
        let error = prerequisites(&prerequisites_input(
            r#""satisfied":[],"same_session":["GEX-1000","gex-1000"],
               "credits":0"#,
        ))
        .expect_err("a duplicated code is a typo to surface");
        assert!(error.contains("GEX-1000"), "{error}");
    }

    #[test]
    fn prerequisites_surface_a_duplicated_satisfied_code() {
        let error = prerequisites(&prerequisites_input(
            r#""satisfied":["GEX-1000","gex-1000"],"credits":0"#,
        ))
        .expect_err("a duplicated code is a typo to surface");
        assert!(error.contains("GEX-1000"), "{error}");
    }

    const PROGRAM: &str = r#"{"code":"B-GEX","slug":"gex","semester":"A26",
        "title":"P","cycle":1,"credits_required":6,
        "mandatory":["GEX-1000","GEX-1001"],
        "rules":[],"concentrations":[],"profiles":[]}"#;

    fn coverage_input(fields: &str) -> CoverageInput {
        serde_json::from_str(&format!(r#"{{"program":{PROGRAM},{fields}}}"#))
            .unwrap_or_else(|e| panic!("input literal: {e}"))
    }

    fn courses() -> Vec<Course> {
        serde_json::from_str(&format!("[{COURSE}]"))
            .unwrap_or_else(|e| panic!("courses literal: {e}"))
    }

    #[test]
    fn coverage_counts_a_partial_grid_without_demanding_a_placement() {
        let report = coverage(
            &coverage_input(r#""selection":["gex-1001"]"#),
            &courses(),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(report.mandatory[0].missing, ["GEX-1000"]);
    }

    #[test]
    fn coverage_surfaces_every_selection_and_report_error() {
        let duplicated = coverage(
            &coverage_input(r#""selection":["GEX-1001","gex-1001"]"#),
            &courses(),
        )
        .expect_err("a duplicated code is a typo to surface");
        assert!(duplicated.contains("GEX-1001"), "{duplicated}");

        let unknown = coverage(
            &coverage_input(r#""selection":[],"concentration":"Aucune""#),
            &courses(),
        )
        .expect_err("no such concentration");
        assert!(unknown.contains("Aucune"), "{unknown}");
    }

    #[test]
    fn the_horizon_answers_semester_codes_etes_included() {
        let input: HorizonInput =
            serde_json::from_str(r#"{"start":"A26","study_sessions":4}"#)
                .unwrap_or_else(|e| panic!("input literal: {e}"));
        let labels: Vec<String> = horizon(&input)
            .unwrap_or_else(|e| panic!("{e}"))
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(labels, ["A26", "H27", "E27", "A27", "H28", "E28"]);
    }
}
