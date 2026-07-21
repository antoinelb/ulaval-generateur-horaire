use std::fs;

use ulaval_scheduler_scraper::parser;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/programs",
);

// Each page comes with the anomalies it is *expected* to raise, in order:
// an anomaly the table does not list fails the test, so a parser that starts
// giving up on a page it used to read says so.
const FIXTURES: &[(&str, &[&str])] = &[
    ("baccalaureat-en-genie-civil", &[]),
    ("baccalaureat-en-genie-des-eaux", &[]),
    (
        "baccalaureat-en-genie-industriel",
        &[
            "Réussir le cours ANL-2020",
            "Réussir les cours requis par sa concentration",
        ],
    ),
    (
        "baccalaureat-en-genie-mecanique",
        &[
            // its concentrations sit under no <h3> at all
            "3 blocks under no heading",
            "Réussir le cours ANL-2020",
            "Réussir le cours ANL-2020",
            "Réussir le cours ANL-2020",
            // a heading cut off mid-sentence, naming no number
            "Règle 1 – Réussir la scolarité de",
            "deuxième cycle suivante :",
        ],
    ),
    (
        "baccalaureat-en-genie-physique",
        &[
            "Réussir le cours ANL-2020",
            "Le profil est satisfait par la réussite",
        ],
    ),
    ("maitrise-en-genie-des-eaux-avec-memoire", &[]),
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
    let page = parse_fixture("maitrise-en-genie-des-eaux-avec-memoire");

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
// prose of its block. It has no rule to live in, so dropping the prose drops
// a graduation requirement (ADR `2026-07-notes-en-prose-conservees`).
#[test]
fn a_graduation_requirement_stated_in_prose_survives_as_a_note() {
    for (name, course) in [
        ("baccalaureat-en-genie-civil", "GCI-2580"),
        ("baccalaureat-en-genie-des-eaux", "GEX-1580"),
        ("baccalaureat-en-genie-mecanique", "GMC-2580"),
    ] {
        let page = parse_fixture(name);

        assert!(
            page.program.notes.iter().any(|note| note.contains(course)),
            "{name} lost the {course} requirement: {:?}",
            page.program.notes
        );
    }
}

// Génie des eaux Règle 4 opens with an unlabelled list, then six thematic
// subgroups. The model has no subgroup, so every group has to be flattened
// into one list — dropping the leading one loses ENT-4020 and GEX-3501,
// which no other group carries.
#[test]
fn every_course_group_of_a_rule_reaches_the_course_list() {
    let page = parse_fixture("baccalaureat-en-genie-des-eaux");

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
    assert_eq!(rule.notes.len(), 8, "six subgroup labels and two notes");
}

fn parse_fixture(name: &str) -> parser::program::ProgramPage {
    let html_path = format!("{FIXTURE_DIR}/{name}.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    parser::program::parse(&html)
        .unwrap_or_else(|e| panic!("parse {name}: {e}"))
}
