use std::fs;

use ulaval_scheduler_core::{parse_prereq_tree, CourseManual};

// The hand-maintained files the scraper never writes still have to parse
// with the core types every consumer uses — a format drift must fail here,
// not in a browser.

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");

#[test]
fn the_manual_courses_file_parses_with_the_snapshot_shape() {
    let raw = fs::read_to_string(format!("{DATA_DIR}/cours.manuel.json"))
        .expect("data/cours.manuel.json is readable");
    let file: CourseManual = serde_json::from_str(&raw)
        .expect("data/cours.manuel.json parses as a CourseManual");
    assert!(
        !file.courses.is_empty(),
        "the manual catalogue lists courses"
    );
    for course in &file.courses {
        // every manual course is offered in every season with no published
        // schedule — the convention that keeps a placeholder placeable
        // (ADR `2026-08-cours-manuels-offerts-en-toute-saison`)
        assert_eq!(
            course.seasons.len(),
            3,
            "{} must list the three seasons",
            course.code
        );
        assert!(
            course
                .seasons
                .values()
                .all(|s| s.last_offered.is_none() && s.options.is_none()),
            "{} must leave last_offered and options null",
            course.code
        );
    }
}

// The vintage overlay is what a student under an older program version
// actually gets, so a key nobody can match or an expression nobody can read
// must fail here rather than quietly correcting nothing in a browser (ADR
// `2026-08-correction-des-prealables-par-millesime`).
#[test]
fn every_vintage_correction_names_a_semester_and_parses() {
    let raw = fs::read_to_string(format!("{DATA_DIR}/cours.manuel.json"))
        .expect("data/cours.manuel.json is readable");
    let file: CourseManual = serde_json::from_str(&raw)
        .expect("data/cours.manuel.json parses as a CourseManual");
    assert_eq!(
        file.malformed_vintages(),
        Vec::<String>::new(),
        "every vintage key must name a semester (« A24 »)"
    );
    for (vintage, overlay) in &file.vintages {
        for (code, text) in &overlay.prerequisites {
            // empty is an answer — « this course had no prerequisites »
            if text.trim().is_empty() {
                continue;
            }
            parse_prereq_tree(text)
                .unwrap_or_else(|error| panic!("{vintage}/{code}: {error}"));
        }
    }
    // the vintage-less layer (the répertoire being plainly wrong, ADR
    // `2026-08-prealables-manuels-sans-millesime`) walks the same gate: a
    // correction nobody can parse would quietly correct nothing
    for (code, text) in &file.prerequisites {
        if text.trim().is_empty() {
            continue;
        }
        parse_prereq_tree(text)
            .unwrap_or_else(|error| panic!("{code}: {error}"));
    }
}
