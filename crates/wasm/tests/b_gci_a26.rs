use ulaval_scheduler_core::{
    apply_prereq_overrides, Course, CourseManual, Program, RuleCourses,
    PREPARATORY_RULE_TITLE,
};
use ulaval_scheduler_wasm::organigramme::{generate, OrganigrammeInput};

#[test]
fn b_gci_a26_places_the_33_base_courses_under_the_published_cap() {
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
    let input = OrganigrammeInput {
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
        completed_sessions: 0,
        seed: Default::default(),
        max_nodes: Some(2_000_000),
        max_solutions: Some(1),
    };
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
