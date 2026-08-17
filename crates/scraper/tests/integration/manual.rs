use std::fs;

use ulaval_scheduler_core::{Course, ProgramManual};

// The hand-maintained files the scraper never writes still have to parse
// with the core types every consumer uses — a format drift must fail here,
// not in a browser.

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");

#[derive(serde::Deserialize)]
struct CoursesFile {
    courses: Vec<Course>,
}

#[test]
fn the_manual_courses_file_parses_with_the_snapshot_shape() {
    let raw = fs::read_to_string(format!("{DATA_DIR}/cours.manuel.json"))
        .expect("data/cours.manuel.json is readable");
    let file: CoursesFile = serde_json::from_str(&raw)
        .expect("data/cours.manuel.json parses as {courses: [Course]}");
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

#[test]
fn every_manual_program_file_parses_with_the_core_type() {
    let dir = format!("{DATA_DIR}/programmes");
    let mut seen = 0;
    let entries = fs::read_dir(&dir)
        .expect("data/programmes is readable")
        .flatten();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".manuel.json") {
            continue;
        }
        let raw = fs::read_to_string(entry.path())
            .unwrap_or_else(|error| panic!("{name} is readable: {error}"));
        let manual: ProgramManual = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("{name} parses: {error}"));
        for cheminement in &manual.cheminements_types {
            assert!(
                cheminement
                    .sessions
                    .iter()
                    .any(|s| s.semester == cheminement.admission),
                "{name}: « {} » must place its admission {} in the timeline",
                cheminement.label,
                cheminement.admission
            );
        }
        seen += 1;
    }
    assert!(
        seen >= 3,
        "expected the converted programs, saw {seen} files"
    );
}
