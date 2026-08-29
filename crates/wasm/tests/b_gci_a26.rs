use std::collections::BTreeMap;

use ulaval_scheduler_core::{
    apply_prereq_overrides, Completion, Course, CourseManual, Program,
    RuleCourses, PREPARATORY_RULE_TITLE,
};
use ulaval_scheduler_wasm::organigramme::{generate, OrganigrammeInput};

#[test]
fn b_gci_a26_places_the_33_base_courses_under_the_published_cap() {
    let courses = catalogue();
    let input = base_input();
    let report = generate(&input, &courses).expect("generation succeeds");
    let solution = report
        .placement
        .solutions
        .first()
        .expect("the base pathway has a placement");
    assert_eq!(solution.placement.len(), 33);
    assert!(solution.left_out.is_empty());
    assert!(solution.credit_shortfalls.is_empty());
    assert!(report.placement.blocked.is_empty());
}

// The 2026-08-27 UX report's complaint, on the real program: delaying one
// course used to rebuild half the path, while the banner promised the
// proposal followed the current one « du plus près ». MAT-1910 pushed from
// A2 to A3 must carry GCI-2002 — which lists it as a prerequisite — and
// leave the other thirty courses exactly where the student saw them (ADR
// `2026-08-b-minimise-la-distance-au-seed`).
#[test]
fn a_delayed_course_moves_only_its_true_dependents() {
    let courses = catalogue();
    let reference = generate(&base_input(), &courses)
        .expect("generation succeeds")
        .placement
        .solutions
        .swap_remove(0)
        .placement;
    assert_eq!(reference["MAT-1910"], 4, "A2 in the reference grid");

    let delayed = OrganigrammeInput {
        seed: reference.clone(),
        // A3, the next automne offering it
        pinned: BTreeMap::from([("MAT-1910".to_string(), 7)]),
        ..base_input()
    };
    let report = generate(&delayed, &courses).expect("generation succeeds");
    assert_eq!(
        report.placement.completion,
        Completion::Complete,
        "the optimum is proven, not merely reached"
    );
    let solution = &report.placement.solutions[0];
    let moved: Vec<&String> = solution
        .placement
        .iter()
        .filter(|(code, session)| reference.get(*code) != Some(session))
        .map(|(code, _)| code)
        .collect();
    assert_eq!(
        moved,
        ["GCI-2002", "MAT-1910"],
        "the delayed course and its one dependent, nothing else"
    );
    assert_eq!(solution.placement["MAT-1910"], 7);
    assert_eq!(reference["GCI-2001"], solution.placement["GCI-2001"]);
    assert_eq!(reference["GCI-1007"], solution.placement["GCI-1007"]);
}

fn catalogue() -> Vec<Course> {
    let catalogue: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/cours.json"))
            .expect("the committed catalogue parses");
    let mut courses: Vec<Course> =
        serde_json::from_value(catalogue["courses"].clone())
            .expect("the catalogue carries courses");
    let manual: CourseManual =
        serde_json::from_str(include_str!("../../../data/cours.manuel.json"))
            .expect("the manual catalogue parses");
    courses.extend(manual.courses.clone());
    courses.sort_by(|left, right| left.code.cmp(&right.code));
    let notes =
        apply_prereq_overrides(&mut courses, &manual.overrides_for("A26"));
    assert!(
        notes.is_empty(),
        "all committed corrections apply: {notes:?}"
    );
    courses
}

fn base_input() -> OrganigrammeInput {
    let program: Program = serde_json::from_str(include_str!(
        "../../../data/programmes/B-GCI-A26.json"
    ))
    .expect("the B-GCI A26 snapshot parses");
    let passed: Vec<String> = program
        .rules
        .iter()
        .find(|rule| rule.title == PREPARATORY_RULE_TITLE)
        .and_then(|rule| match &rule.courses {
            RuleCourses::List { courses } => Some(courses.clone()),
            _ => None,
        })
        .unwrap_or_default();
    OrganigrammeInput {
        courses: None,
        program: Some(program),
        concentration: Some("Eau et environnement".to_string()),
        profile: None,
        electives: Vec::new(),
        passed,
        pinned: Default::default(),
        start: ulaval_scheduler_core::Season::Fall,
        study_sessions: 8,
        credit_cap: 17,
        concomitant: false,
        summers_open: false,
        frozen: Default::default(),
        seed: Default::default(),
        max_nodes: Some(2_000_000),
        max_solutions: Some(1),
    }
}
