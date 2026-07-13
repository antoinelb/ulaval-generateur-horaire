use ulaval_scheduler_core::Catalogue;

// Same contract as `course` and `program`: round-trip the real listing
// fixture to prove `Catalogue` captures every field losslessly.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/listing",
);

const FIXTURES: &[&str] = &["gex"];

#[test]
fn round_trips_every_fixture() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));

        let catalogue: Catalogue = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("deserialize {name}: {e}"));
        let reserialized = serde_json::to_value(&catalogue)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        let original: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name} as value: {e}"));

        assert_eq!(reserialized, original, "lossy round-trip for {name}");
    }
}
