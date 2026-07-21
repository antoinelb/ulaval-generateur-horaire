use std::collections::BTreeMap;
use std::fs;

use ulaval_scheduler_core::{Credits, Season};
use ulaval_scheduler_scraper::parser;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/courses",
);

// The last four pages each pin one family of the anomalies the first full
// catalogue run logged:
//   med-1911  « 6 à 12 » credits, a stage the student weights himself
//   ift-1004  two top-level sections, only one of them carrying labs
//   cso-6702  two top-level sections hanging off one common seminar NRC
//   drt-7104  a stray `<b>` on the page, which HTML5 turns into a re-parent
const FIXTURES: &[&str] = &[
    "act-4114", "chm-0150", "cso-6702", "drt-7104", "ecn-4901", "esp-1000",
    "frn-1112", "gae-3008", "gci-1007", "gci-2010", "gci-2510", "gex-3100",
    "gex-3333", "gex-4008", "gex-7002", "gmc-1590", "gmc-7000", "gml-1001",
    "ift-1004", "med-1911", "phi-7750",
];

#[test]
fn parses_every_course_fixture_without_anomalies() {
    for name in FIXTURES {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let page = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"))
            .unwrap_or_else(|| panic!("{name} is in scope"));

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

// GCI-2510 is a « Stage » seminar carrying no credits card at all: it is
// worth 0 credits rather than being dropped. Its préalable — an obligatory
// training to pass — reads as an examination, so the whole table above
// covers it like any other page.
#[test]
fn a_seminar_without_a_credits_card_is_worth_zero() {
    let html_path = format!("{FIXTURE_DIR}/gci-2510.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let page = parser::course::parse(&html)
        .unwrap_or_else(|e| panic!("parse: {e}"))
        .expect("GCI-2510 is in scope");

    assert_eq!(page.course.credits, Credits::Fixed(0));
}

// A page can be perfectly well-formed and still describe an activity the
// generator has no business scheduling. MDD-5101 is a post-doctoral dental
// residency (« Études post-MDD »), PSY-7851 a doctoral thesis milestone
// (third cycle only): both are recognized, then deliberately dropped —
// no course, and no anomaly either, since nothing was lost by accident.
//
// PSY-785x also falsifies the claim in ADR
// `2026-07-troisieme-cycle-hors-perimetre` that these activities are all
// numbered 8xxx: the catalogue's 8xxx filter is a shortcut before the HTTP
// request, not an exhaustive one.
#[test]
fn a_course_beyond_the_second_cycle_yields_no_course_and_no_anomaly() {
    for name in ["mdd-5101", "psy-7851"] {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let parsed = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        assert!(parsed.is_none(), "{name} must be out of scope");
    }
}

// MED-1911 lists no session at all, so the range is the only thing its
// fixture asserts — nothing else can mask a regression on it.
#[test]
fn a_stage_the_student_weights_himself_keeps_both_bounds() {
    let html_path = format!("{FIXTURE_DIR}/med-1911.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let page = parser::course::parse(&html)
        .unwrap_or_else(|e| panic!("parse: {e}"))
        .expect("MED-1911 is in scope");

    assert_eq!(page.course.credits, Credits::Range { min: 6, max: 12 });
}

// The falsifier of ADR `2026-07-sections-en-groupes-de-choix` §5. Automne
// 2026 offers NRC 85469 (in class) with labs 85470/85471, and NRC 85472
// (Z3, remote) with none. The old flat model read this as « one of
// {85469, 85472} and one of {85470, 85471} », whose product pairs the
// remote section with an in-class lab and cannot express 85472 on its own.
#[test]
fn labs_stay_attached_to_the_section_that_offers_them() {
    let html_path = format!("{FIXTURE_DIR}/ift-1004.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let page = parser::course::parse(&html)
        .unwrap_or_else(|e| panic!("parse: {e}"))
        .expect("IFT-1004 is in scope");

    let fall = page
        .course
        .seasons
        .get(&Season::Fall)
        .expect("fall offering");
    let nrcs: Vec<Vec<&str>> = fall
        .options
        .iter()
        .map(|option| option.iter().map(|s| s.nrc.as_str()).collect())
        .collect();

    assert_eq!(
        nrcs,
        vec![
            vec!["85469", "85470"],
            vec!["85469", "85471"],
            vec!["85472"],
        ]
    );
}

// DRT-7104's automne 2023 block contains `<b>Droit de la concurrence<b>` —
// the closing tag is a typo. HTML5's adoption agency algorithm reconstructs
// the unclosed `<b>`s, which re-parents section B two levels below the
// session, out of reach of a direct-children scan. The section vanished
// from `data/cours/a2023.json` and only the « N sections offertes »
// reconciliation noticed.
#[test]
fn a_section_re_parented_by_a_stray_tag_is_still_found() {
    let html_path = format!("{FIXTURE_DIR}/drt-7104.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let page = parser::course::parse(&html)
        .unwrap_or_else(|e| panic!("parse: {e}"))
        .expect("DRT-7104 is in scope");

    let fall = page
        .course
        .seasons
        .get(&Season::Fall)
        .expect("fall offering");
    let nrcs: Vec<&str> = fall
        .options
        .iter()
        .flatten()
        .map(|s| s.nrc.as_str())
        .collect();

    assert_eq!(nrcs, vec!["84328", "84329"]);
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
            .unwrap_or_else(|e| panic!("parse {name}: {e}"))
            .unwrap_or_else(|| panic!("{name} is in scope"));

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
