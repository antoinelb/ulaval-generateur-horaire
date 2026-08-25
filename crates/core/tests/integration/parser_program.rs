use std::fs;

use ulaval_scheduler_core::parser;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/programs",
);

// Each page comes with the anomalies it is *expected* to raise, in order:
// an anomaly the table does not list fails the test, so a parser that starts
// giving up on a page it used to read says so.
const FIXTURES: &[(&str, &[&str])] = &[
    ("B-GCI", &[]),
    ("B-GEX", &[]),
    // the ANL-2020 requirement now becomes `language_requirement` and « requis
    // par sa concentration » a recognized `negotiated` rule: neither is an
    // anomaly anymore (ADR `2026-07-exigence-linguistique-champ-dedie`,
    // `2026-07-regles-negociees-reconnues`)
    ("B-GIN", &[]),
    (
        "B-GMC",
        // the three ANL-2020 rules and the passage intégré are handled now;
        // only the concentrations sitting under no <h3> remain an anomaly
        &["3 blocks under no heading"],
    ),
    ("B-GPH", &[]),
    ("M-GEX", &[]),
];

#[test]
fn parses_every_program_fixture() {
    for (name, expected) in FIXTURES {
        let page = parse_fixture(name);

        let raised: Vec<String> = page
            .anomalies
            .iter()
            .map(|anomaly| anomaly.to_string())
            .collect();
        assert_eq!(
            raised.len(),
            expected.len(),
            "unexpected anomalies on {name}: {raised:#?}"
        );
        for (anomaly, expected) in raised.iter().zip(expected.iter()) {
            assert!(
                anomaly.contains(expected),
                "on {name}, expected an anomaly about {expected:?}, got {anomaly:?}"
            );
        }

        let got = serde_json::to_value(&page.program)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));

        let json_path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("read {json_path}: {e}"));
        let expected: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {json_path}: {e}"));

        assert_eq!(got, expected, "parsed program differs from {name}.json");
    }
}

// The « Recherche » block of the maîtrise states no « N crédits exigés », so
// a rule built from it could only carry an invented constraint. Its courses
// are obligatory — GEX-6811 to 6814 are worth 7+7+7+9 = 30 credits, the
// mémoire the page describes — and 15 + 30 = 45, the credits required.
#[test]
fn a_second_program_block_contributes_mandatory_courses_not_a_rule() {
    let page = parse_fixture("M-GEX");

    assert_eq!(
        page.program.mandatory,
        [
            "GCI-7077", "GEX-6001", "GEX-6811", "GEX-6812", "GEX-6813",
            "GEX-6814"
        ]
    );
    assert_eq!(
        page.program.rules.len(),
        1,
        "« Recherche » names no rule of its own"
    );
}

// A stage that has to be passed to graduate is named nowhere but in the
// prose of its block. It is promoted into a « Stages » rule appended after
// the scraped ones — its credits « en sus » of the program total — and the
// paragraph survives as the rule's note (ADR
// `2026-08-stage-obligatoire-en-prose-promu-en-regle`).
#[test]
fn a_graduation_stage_stated_in_prose_becomes_a_rule() {
    use ulaval_scheduler_core::{Constraint, RuleCourses};

    for (name, course) in [
        ("B-GCI", "GCI-2580"),
        ("B-GEX", "GEX-1580"),
        ("B-GMC", "GMC-2580"),
    ] {
        let page = parse_fixture(name);

        let stage = page
            .program
            .rules
            .iter()
            .find(|rule| rule.title == "Stages")
            .unwrap_or_else(|| panic!("{name} lost the {course} requirement"));
        assert_eq!(
            stage.constraint,
            Some(Constraint::Course { min: 1, max: 8 }),
            "{name}: at least the mandatory stage, up to the eight allowed"
        );
        let courses = match &stage.courses {
            RuleCourses::List { courses } => courses,
            other => panic!("{name}: expected a course list, got {other:?}"),
        };
        assert_eq!(
            courses.first().map(String::as_str),
            Some(course),
            "{name}: the mandatory stage comes first"
        );
        assert!(stage.credits_in_addition, "{name}: « en sus » des crédits");
        assert!(
            stage.notes.iter().any(|note| note.contains(course)),
            "{name}: the paragraph survives as the rule's note"
        );
        assert!(
            !page.program.notes.iter().any(|note| note.contains(course)),
            "{name}: the prose no longer rides in program.notes"
        );
    }
}

// Génie des eaux Règle 4 opens with an unlabelled list, then six thematic
// subgroups. The model has no subgroup, so every group has to be flattened
// into one list — dropping the leading one loses ENT-4020 and GEX-3501,
// which no other group carries.
#[test]
fn every_course_group_of_a_rule_reaches_the_course_list() {
    let page = parse_fixture("B-GEX");

    let rule = page
        .program
        .rules
        .iter()
        .find(|rule| rule.title == "Règle 4")
        .expect("Règle 4");

    let courses = match &rule.courses {
        ulaval_scheduler_core::RuleCourses::List { courses } => courses,
        other => panic!("expected a course list, got {other:?}"),
    };
    assert_eq!(courses.len(), 19);
    assert!(courses.contains(&"ENT-4020".to_string()));
    assert!(courses.contains(&"GEX-3501".to_string()));
    assert_eq!(
        rule.notes.len(),
        6,
        "six subgroup labels; the two language notes moved to \
         program.language_requirement"
    );
}

// The language requirement is a course-or-test graduation gate lifted out of
// the rules/notes it hid in into its own field (ADR
// `2026-07-exigence-linguistique-champ-dedie`).
#[test]
fn the_language_requirement_becomes_its_own_field() {
    use ulaval_scheduler_core::PlacementTest;

    // two-box layout: both audiences, FLS-2093 carrying two ANDed thresholds
    let eaux = parse_fixture("B-GEX");
    let requirement = eaux
        .program
        .language_requirement
        .expect("génie des eaux states a language requirement");
    assert_eq!(requirement.francophone.course, "ANL-2020");
    assert_eq!(
        requirement.francophone.tests,
        vec![PlacementTest {
            name: "VEPT".to_string(),
            score: 53
        }]
    );
    let french = requirement
        .non_francophone
        .expect("the two-box layout spells out the non-francophone branch");
    assert_eq!(french.course, "FLS-2093");
    assert_eq!(
        french.tests,
        vec![
            PlacementTest {
                name: "TCF-TP".to_string(),
                score: 400
            },
            PlacementTest {
                name: "TCF-TP/ÉÉ".to_string(),
                score: 14
            },
        ]
    );

    // prose layout: francophone only, and only the first threshold — the later
    // « (VEPT : 63) » upgrade tier stays in raw
    let physique = parse_fixture("B-GPH");
    let requirement = physique
        .program
        .language_requirement
        .expect("génie physique states a language requirement");
    assert_eq!(
        requirement.francophone.tests,
        vec![PlacementTest {
            name: "VEPT".to_string(),
            score: 53
        }]
    );
    assert!(requirement.non_francophone.is_none());
    assert!(requirement.francophone.raw.contains("VEPT : 63"));
}

fn parse_fixture(name: &str) -> parser::program::ProgramPage {
    let html_path = format!("{FIXTURE_DIR}/{name}.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    // A26: the vintage the fixture pages were frozen under
    let semester = "A26".parse().unwrap_or_else(|e| panic!("{e}"));
    parser::program::parse(&html, semester)
        .unwrap_or_else(|e| panic!("parse {name}: {e}"))
}
