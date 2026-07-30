use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{schedule_report, Course, Season};

// The frozen weekly-schedule contract (ADR
// `2026-07-contrat-horaire-hebdomadaire-vers-ui`): each fixture pins one
// phenomenon, its expected output derived by a throwaway brute-force
// reference — the solver reproduces these verdicts, never the reverse.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/schedules",
);

const FIXTURES: &[&str] = &[
    "alternative-invalid",
    "back-to-back",
    "chosen-conflict",
    "chosen-non-first",
    "contained-slot",
    "different-days",
    "hybrid-with-conflict",
    "identical-slots",
    "lab-only-conflict",
    "pairwise-infeasible",
    "partial-overlap",
    "remote-never-conflicts",
    "same-day-disjoint",
    "second-slot-conflict",
    "shared-nrc",
    "single-course",
    "swap-requires-other-move",
    "triple-infeasible-pairwise-ok",
];

#[derive(serde::Deserialize)]
struct Fixture {
    season: Season,
    courses: Vec<Course>,
    #[serde(default)]
    chosen: BTreeMap<String, BTreeSet<String>>,
    expected: serde_json::Value,
}

#[test]
fn reproduces_every_frozen_verdict() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));
        let fixture: Fixture = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        let report =
            schedule_report(&fixture.courses, fixture.season, &fixture.chosen)
                .unwrap_or_else(|e| panic!("report {name}: {e}"));

        let got = serde_json::to_value(&report)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        assert_eq!(got, fixture.expected, "verdict differs on {name}");
    }
}
