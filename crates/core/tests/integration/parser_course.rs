use std::fs;

use ulaval_scheduler_core::parser;
use ulaval_scheduler_core::{Credits, PrereqTree, Prerequisites, Season};

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/courses",
);

// Some pages each pin one family of the anomalies full catalogue runs
// logged:
//   med-1911  « 6 à 12 » credits, a stage the student weights himself
//   ift-1004  two top-level sections, only one of them carrying labs
//   cso-6702  two top-level sections hanging off one common seminar NRC
//   drt-7104  a stray `<b>` on the page, which HTML5 turns into a re-parent
//   bio-1003  two distinct préuniversitaire messages, one behind
//             « REMARQUE : », a comma enumeration its « ou » governs, and
//             cégep sigles kept Raw
//   gex-3001  « GCI-2010* », the répertoire's mark for a préalable that
//             « peut être suivi simultanément »
const FIXTURES: &[&str] = &[
    "act-4114", "bio-1003", "chm-0150", "cso-6702", "drt-7104", "ecn-4901",
    "esp-1000", "frn-1112", "gae-3008", "gci-1007", "gci-1011", "gci-2010",
    "gci-2510", "gex-3001", "gex-3100", "gex-3333", "gex-4008", "gex-7002",
    "gmc-1590", "gmc-7000", "gml-1001", "ift-1004", "med-1911", "phi-7750",
];

// Regenerates every expected fixture from its frozen HTML (ADR
// `2026-07-fixture-attendue-derivee-avant-le-parseur`: the parser already
// reads these pages, so the expected output is derived by it, hand-reviewed
// against the page, then frozen — never written by hand). Run with
// `UPDATE_FIXTURES=1 cargo test -p ulaval-scheduler-core`; a plain run
// leaves the files untouched.
#[test]
fn update_fixtures() {
    if std::env::var_os("UPDATE_FIXTURES").is_none() {
        return;
    }
    for name in FIXTURES {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));
        let page = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"))
            .unwrap_or_else(|| panic!("{name} is in scope"));
        let json = serde_json::to_string_pretty(&page.course)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        fs::write(format!("{FIXTURE_DIR}/{name}.json"), json + "\n")
            .unwrap_or_else(|e| panic!("write {name}.json: {e}"));
    }
}

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

// The star the répertoire puts after a sigle — « GCI-2010* », glossed on
// the page as « un préalable qui peut être suivi simultanément » — is the
// only thing GEX-3001's fixture is here for: it must reach the tree as a
// concomitant leaf, not be swallowed the way the grammar used to
// (ADR `2026-08-etoile-de-concomitance-au-parsing`).
#[test]
fn a_starred_prerequisite_reaches_the_tree_as_a_concomitant_leaf() {
    let html_path = format!("{FIXTURE_DIR}/gex-3001.html");
    let html = fs::read_to_string(&html_path)
        .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

    let page = parser::course::parse(&html)
        .unwrap_or_else(|e| panic!("parse: {e}"))
        .expect("GEX-3001 is in scope");

    assert_eq!(
        page.course.prerequisites,
        Some(Prerequisites::Parsed {
            raw: "GCI-2010*".to_string(),
            tree: PrereqTree::Concomitant {
                concomitant: "GCI-2010".to_string()
            },
        })
    );
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
        .as_deref()
        .expect("published schedule")
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
        .as_deref()
        .expect("published schedule")
        .iter()
        .flatten()
        .map(|s| s.nrc.as_str())
        .collect();

    assert_eq!(nrcs, vec!["84328", "84329"]);
}

// The vintage of each retained block lives inside the offering itself
// (`last_offered`). GCI-1007 lists Automne 2024, 2025 and 2026; only 2026
// survives. GCI-1011 has no session section at all: the new-course rule
// marks it fall and winter with no vintage and no schedule (ADR
// `2026-07-cours-sans-section-de-session-offert-automne-hiver`).
#[test]
fn each_retained_season_carries_the_year_it_was_read_from() {
    for (name, expected) in [
        ("gci-1007", vec![(Season::Fall, Some(2026))]),
        (
            "ecn-4901",
            vec![(Season::Winter, Some(2026)), (Season::Summer, Some(2026))],
        ),
        (
            "gci-1011",
            vec![(Season::Fall, None), (Season::Winter, None)],
        ),
    ] {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let page = parser::course::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"))
            .unwrap_or_else(|| panic!("{name} is in scope"));

        let vintages: Vec<(Season, Option<u16>)> = page
            .course
            .seasons
            .iter()
            .map(|(&season, offering)| (season, offering.last_offered))
            .collect();
        assert_eq!(vintages, expected, "vintages parsed from {name}");
    }
}
