use std::collections::BTreeSet;

use ulaval_scheduler_core::{coverage_report, Course, Program};

// The frozen rules-coverage contract (ADR
// `2026-07-schema-du-rapport-de-couverture-en-fixtures`): fixtures written
// on the real pages of five programs, expected derived by the committed
// reference `tests/reference/solveur_b/verify_rules.py` — the verifier
// reproduces these verdicts, never the reverse.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/rules",
);

const FIXTURES: &[&str] = &[
    "concentration-mandatory-missing",
    "constraint-none-reported",
    "gex-rule1-count-missing",
    "gex-rule1-count-satisfied",
    "gex-rule2-credits-missing",
    "gex-rule3-credits-satisfied",
    "gex-rule4-min-equals-max",
    "gex-rule5-any-reported",
    "language-requirement-reported",
    "language-requirement-satisfied",
    "negotiated-rule-reported",
    "profile-mandatory-and-rules",
    "raw-only-rule-reported",
    "reference-rule-resolved",
];

#[derive(serde::Deserialize)]
struct Fixture {
    program: Program,
    #[serde(default)]
    concentration: Option<String>,
    #[serde(default)]
    profile: Option<String>,
    selection: Vec<String>,
    #[serde(default)]
    courses: Vec<Course>,
    expected: serde_json::Value,
}

#[test]
fn reproduces_every_frozen_coverage_report() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));
        let fixture: Fixture = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));
        let selection: BTreeSet<String> =
            fixture.selection.into_iter().collect();

        let report = coverage_report(
            &fixture.program,
            fixture.concentration.as_deref(),
            fixture.profile.as_deref(),
            &selection,
            &fixture.courses,
        )
        .unwrap_or_else(|e| panic!("report {name}: {e}"));

        let got = serde_json::to_value(&report)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        assert_eq!(got, fixture.expected, "coverage differs on {name}");
    }
}
