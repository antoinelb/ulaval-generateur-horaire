use std::fs;

use ulaval_scheduler_core::parser;

const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/transcripts",
);

const FIXTURES: &[&str] = &["exemple"];

// Regenerates every expected fixture from its frozen HTML (ADR
// `2026-07-fixture-attendue-derivee-avant-le-parseur`: the parser already
// reads these pages, so the expected output is derived by it, hand-reviewed
// against the page, then frozen — never written by hand). Run with
// `UPDATE_FIXTURES=1 cargo test -p ulaval-scheduler-core --test integration
// parser_transcript`; a plain run leaves the files untouched.
#[test]
fn update_fixtures() {
    if std::env::var_os("UPDATE_FIXTURES").is_none() {
        return;
    }
    for name in FIXTURES {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));
        let page = parser::transcript::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));
        let json = serde_json::to_string_pretty(&page.transcript)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        fs::write(format!("{FIXTURE_DIR}/{name}.json"), json + "\n")
            .unwrap_or_else(|e| panic!("write {name}.json: {e}"));
    }
}

#[test]
fn parses_every_transcript_fixture_without_anomalies() {
    for name in FIXTURES {
        let html_path = format!("{FIXTURE_DIR}/{name}.html");
        let html = fs::read_to_string(&html_path)
            .unwrap_or_else(|e| panic!("read {html_path}: {e}"));

        let page = parser::transcript::parse(&html)
            .unwrap_or_else(|e| panic!("parse {name}: {e}"));

        assert!(
            page.anomalies.is_empty(),
            "anomalies on {name}: {:?}",
            page.anomalies
        );

        let got = serde_json::to_value(&page.transcript)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));

        let json_path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("read {json_path}: {e}"));
        let expected: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {json_path}: {e}"));

        assert_eq!(
            got, expected,
            "parsed transcript differs from {name}.json"
        );
    }
}
