use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    place, Completion, Course, PlacementRequest, Season,
};

// The frozen placement contract (ADR
// `2026-07-schema-des-fixtures-de-placement`): every fixture pins one
// phenomenon, its complete solution set derived by the committed reference
// `tests/reference/solveur_b/place.py` — the solver reproduces these sets,
// never the reverse. The solver's search order differs from the frozen
// canonical order, so the output is canonicalized before comparison, as
// the ADR prescribes.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/organigrammes",
);

const FIXTURES: &[&str] = &[
    "concomitant-relaxes-precedence",
    "credit-cap-forces-split",
    "multiple-solutions-all-enumerated",
    "over-constrained-proves-infeasible",
    "passed-credits-count-toward-threshold",
    "passed-satisfies-prerequisite",
    "pinned-conflict-infeasible",
    "pinned-session-respected",
    "prereq-chain-forces-order",
    "program-credits-threshold-gates",
    "season-restricts-placement",
    "unsatisfiable-prerequisite-proves-infeasible",
    "weekly-veto-splits-conflicting-courses",
    "winter-start-inverts-projects",
];

#[derive(serde::Deserialize)]
struct Fixture {
    sessions: Vec<Season>,
    credit_cap: u32,
    #[serde(default)]
    concomitant: bool,
    #[serde(default)]
    passed: BTreeSet<String>,
    #[serde(default)]
    pinned: BTreeMap<String, usize>,
    courses: Vec<Course>,
    expected: Expected,
}

#[derive(serde::Deserialize)]
struct Expected {
    complete: bool,
    solutions: Vec<BTreeMap<String, usize>>,
}

#[test]
fn reproduces_every_frozen_solution_set() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));
        let fixture: Fixture = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        let placement = place(&PlacementRequest {
            sessions: &fixture.sessions,
            credit_cap: fixture.credit_cap,
            concomitant: fixture.concomitant,
            courses: &fixture.courses,
            passed: &fixture.passed,
            pinned: &fixture.pinned,
            seed: &BTreeMap::new(),
            max_nodes: 10_000_000,
            max_solutions: 100_000,
        })
        .unwrap_or_else(|e| panic!("place {name}: {e}"));

        assert_eq!(
            placement.completion == Completion::Complete,
            fixture.expected.complete,
            "completion differs on {name}"
        );
        // the fixtures avoid every unverifiable-operand case: nothing may
        // have been presumed to reach these sets
        for solution in &placement.solutions {
            assert!(
                solution.assumed.is_empty(),
                "unexpected assumption on {name}: {:?}",
                solution.assumed
            );
        }
        let mut got: Vec<BTreeMap<String, usize>> = placement
            .solutions
            .into_iter()
            .map(|solution| solution.placement)
            .collect();
        got.sort();
        assert_eq!(
            got, fixture.expected.solutions,
            "solution set differs on {name}"
        );
    }
}
