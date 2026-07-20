use ulaval_scheduler_core::Program;

// Same contract as `course`: round-trip every real program fixture to prove
// `Program` captures every field losslessly.
const FIXTURE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/test_cases/programs",
);

const FIXTURES: &[&str] = &[
    "baccalaureat-en-genie-civil",
    "baccalaureat-en-genie-des-eaux",
    "baccalaureat-en-genie-industriel",
    "baccalaureat-en-genie-mecanique",
    "baccalaureat-en-genie-physique",
    "maitrise-en-genie-des-eaux-avec-memoire",
];

#[test]
fn round_trips_every_fixture() {
    for name in FIXTURES {
        let path = format!("{FIXTURE_DIR}/{name}.json");
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {path}: {e}"));

        let program: Program = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("deserialize {name}: {e}"));
        let reserialized = serde_json::to_value(&program)
            .unwrap_or_else(|e| panic!("serialize {name}: {e}"));
        let original: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("parse {name} as value: {e}"));

        assert_eq!(reserialized, original, "lossy round-trip for {name}");
    }
}
