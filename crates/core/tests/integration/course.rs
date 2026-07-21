use ulaval_scheduler_core::Course;

// Round-trip every real test-case fixture: deserialize into `Course`,
// serialize back, and compare as JSON values (order-insensitive). Equality
// proves the type captures every field losslessly against the data the
// scraper will actually produce.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/courses",
);

// `med-1911` carries the range form of `credits`, `cso-6702` an NRC shared
// by two options: both are shapes no other fixture exercises.
const FIXTURES: &[&str] = &[
    "gci-1007", "gci-2010", "gex-7002", "gex-4008", "ecn-4901", "gae-3008",
    "med-1911", "cso-6702",
];

#[test]
fn round_trips_every_fixture() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));

        let course: Course = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("deserialize {name}: {e}"));
        let reserialized = serde_json::to_value(&course)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        let original: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name} as value: {e}"));

        assert_eq!(reserialized, original, "lossy round-trip for {name}");
    }
}
