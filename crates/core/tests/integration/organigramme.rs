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
    "closed-summer-refuses-regular-course",
    "concomitant-relaxes-precedence",
    "credit-cap-forces-split",
    "multiple-solutions-all-enumerated",
    "open-summer-accepts-regular-course",
    "over-constrained-proves-infeasible",
    "passed-credits-count-toward-threshold",
    "passed-satisfies-prerequisite",
    "pinned-conflict-infeasible",
    "pinned-session-respected",
    "preparatory-all-passed-none-placed",
    "preparatory-none-passed-all-placed",
    "preparatory-some-passed-rest-precede-dependents",
    "prereq-chain-forces-order",
    "program-credits-threshold-gates",
    "season-restricts-placement",
    "stage-pinned-in-fall-lifts-summer-restriction",
    "stage-unpinned-lands-in-summer",
    // une épingle n'est pas un fait exempté : posée dans la session de son
    // propre préalable strict, concomitance décochée, elle est réfutée —
    // et rien d'autre ne peut refuser ici (les deux cours sont offerts aux
    // deux saisons, tiennent ensemble sous le plafond et ne se chevauchent
    // pas dans la semaine). ADR
    // `2026-08-une-epingle-est-verifiee-comme-le-reste`
    "strict-prereq-same-session",
    "unsatisfiable-prerequisite-proves-infeasible",
    "weekly-veto-splits-conflicting-courses",
    "winter-start-inverts-projects",
    // the relaxed family (ADR `2026-08-placement-au-mieux-en-repli`) —
    // hand-written: the Python reference enumerates the whole frontier and
    // a sentinel per course would multiply its space by (n+1)
    "relaxed-empty-domain-left-out-rest-placed",
    "relaxed-unsatisfiable-prerequisite-cascades",
    "relaxed-credit-excess-left-out",
    "relaxed-nothing-placeable-leaves-everything-out",
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
    #[serde(default)]
    stages: BTreeSet<String>,
    #[serde(default)]
    open_summers: BTreeSet<usize>,
    #[serde(default)]
    allow_unplaced: bool,
    courses: Vec<Course>,
    expected: Expected,
}

#[derive(serde::Deserialize)]
struct Expected {
    complete: bool,
    solutions: Vec<BTreeMap<String, usize>>,
    // one entry per solution, same order — absent means « every solution
    // places everything », which is the whole exact family
    #[serde(default)]
    left_out: Vec<BTreeSet<String>>,
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
            stages: &fixture.stages,
            open_summers: &fixture.open_summers,
            frozen: &BTreeSet::new(),
            seed: &BTreeMap::new(),
            max_nodes: 10_000_000,
            // relaxed, only the first leaf is interesting: the sentinel is
            // tried last at every depth, so the first one found is the
            // greedy filling and every later one is strictly worse. That
            // is also exactly how `wasm::organigramme` calls it.
            max_solutions: if fixture.allow_unplaced { 1 } else { 100_000 },
            allow_unplaced: fixture.allow_unplaced,
            allow_credit_shortfall: false,
            // the frozen sets are the *complete* enumeration: minimizing
            // would return one solution, balancing would move courses out
            // of the arrangement the reference derived
            minimize_seed_distance: false,
            balance: false,
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
        let mut got: Vec<(BTreeMap<String, usize>, BTreeSet<String>)> =
            placement
                .solutions
                .into_iter()
                .map(|solution| (solution.placement, solution.left_out))
                .collect();
        got.sort();
        let expected_left_out = if fixture.expected.left_out.is_empty() {
            vec![BTreeSet::new(); fixture.expected.solutions.len()]
        } else {
            fixture.expected.left_out.clone()
        };
        let mut want: Vec<(BTreeMap<String, usize>, BTreeSet<String>)> =
            fixture
                .expected
                .solutions
                .into_iter()
                .zip(expected_left_out)
                .collect();
        want.sort();
        assert_eq!(got, want, "solution set differs on {name}");
    }
}
