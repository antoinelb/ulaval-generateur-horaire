use std::collections::BTreeMap;
use std::fs;

use ulaval_scheduler_core::Season;
use ulaval_scheduler_scraper::parser;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/courses",
);

const FIXTURES: &[&str] = &[
    "ecn-4901", "gae-3008", "gci-1007", "gci-2010", "gex-3100", "gex-3333",
    "gex-4008", "gex-7002",
];

#[test]
fn parses_every_course_fixture_without_anomalies() {
    for name in FIXTURES {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let page = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        assert!(
            page.anomalies.is_empty(),
            "anomalies on {name}: {:?}",
            page.anomalies
        );

        let got = serde_json::to_value(&page.course)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));

        let json_path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("read {json_path}: {e}"));
        let expected: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {json_path}: {e}"));

        assert_eq!(got, expected, "parsed course differs from {name}.json");
    }
}

// `Course` is keyed by season alone, but the snapshots are named per
// session (`a2026`), so the year of each retained block has to reach the
// scraper. GCI-1007 lists Automne 2024, 2025 and 2026; only 2026 survives.
#[test]
fn each_retained_season_carries_the_year_it_was_read_from() {
    for (name, expected) in [
        ("gci-1007", vec![(Season::Fall, 2026)]),
        (
            "ecn-4901",
            vec![(Season::Winter, 2026), (Season::Summer, 2026)],
        ),
    ] {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let page = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        assert_eq!(
            page.years,
            BTreeMap::from_iter(expected),
            "years parsed from {name}"
        );
        assert_eq!(
            page.years.keys().collect::<Vec<_>>(),
            page.course.seasons.keys().collect::<Vec<_>>(),
            "every retained season must have a year ({name})"
        );
    }
}
