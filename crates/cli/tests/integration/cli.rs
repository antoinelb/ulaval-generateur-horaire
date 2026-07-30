use std::path::PathBuf;
use std::process::Command;

// the exact JSON shape the scraper writes to `data/cours/{session}.json`:
// two courses on the same monday slot, a winter-only course borrowing its
// equivalent's fall offering, a winter-only orphan, and a variable-credit
// stage — enough to drive every exit of the binary
const SNAPSHOT: &str = r#"{"courses":[
  {"code":"GEX-1000","title":"T","credits":3,"cycle":1,
   "prerequisites":null,"equivalents":[],
   "seasons":{"fall":{"options":[[{"nrc":"81001","section":"A",
     "mode":"in-person","slots":[{"day":"monday","start":"08:30",
     "end":"11:20"}]}]]}}},
  {"code":"GCI-1000","title":"T","credits":3,"cycle":1,
   "prerequisites":null,"equivalents":[],
   "seasons":{"fall":{"options":[[{"nrc":"82002","section":"A",
     "mode":"in-person","slots":[{"day":"monday","start":"08:30",
     "end":"11:20"}]}]]}}},
  {"code":"GEX-2000","title":"T","credits":3,"cycle":1,
   "prerequisites":null,"equivalents":["GCI-1000"],
   "seasons":{"winter":{"options":[[]]}}},
  {"code":"GEX-3000","title":"T","credits":3,"cycle":1,
   "prerequisites":null,"equivalents":[],
   "seasons":{"winter":{"options":[[]]}}},
  {"code":"GEX-2580","title":"Stage","credits":{"min":6,"max":12},
   "cycle":1,"prerequisites":null,"equivalents":[],
   "seasons":{"fall":{"options":[[{"nrc":"85800","section":null,
     "mode":"remote","slots":[]}]]}}}
]}"#;

#[test]
fn the_binary_prints_a_schedule_end_to_end() {
    let dir = test_dir("e2e-happy");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .args([
            "schedule",
            "a2026",
            "GEX-1000",
            "--data-dir",
            &dir.display().to_string(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("81001"), "the NRC is printed: {stdout}");
    assert!(stdout.contains("Total : 3 crédits"), "{stdout}");
    cleanup(&dir);
}

#[test]
fn the_binary_exits_2_on_a_conflict_after_printing_the_schedule() {
    let dir = test_dir("e2e-conflict");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .args([
            "schedule",
            "a2026",
            "GEX-1000",
            "GCI-1000",
            "--data-dir",
            &dir.display().to_string(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    // the schedule is still shown — nothing is lost to the failure
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠ conflit"), "{stdout}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GEX-1000"), "{stderr}");
    cleanup(&dir);
}

#[test]
fn the_binary_resolves_an_equivalent_offering_end_to_end() {
    let dir = test_dir("e2e-equivalent");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .args([
            "schedule",
            "a2026",
            "GEX-2000",
            "--data-dir",
            &dir.display().to_string(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("82002"),
        "the equivalent's offering is attended: {stdout}"
    );
    cleanup(&dir);
}

#[test]
fn the_binary_exits_2_when_a_course_is_not_offered() {
    let dir = test_dir("e2e-not-offered");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .args([
            "schedule",
            "a2026",
            "GEX-3000",
            "--data-dir",
            &dir.display().to_string(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GEX-3000"), "{stderr}");
    cleanup(&dir);
}

#[test]
fn the_binary_exits_2_on_a_variable_credit_stage() {
    // no weighting flag in v0: the missing choice is surfaced, not
    // defaulted to a bound
    let dir = test_dir("e2e-stage");

    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .args([
            "schedule",
            "a2026",
            "GEX-2580",
            "--data-dir",
            &dir.display().to_string(),
        ])
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GEX-2580"), "{stderr}");
    cleanup(&dir);
}

#[test]
fn the_binary_rejects_a_naked_invocation_with_exit_code_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_ulaval-scheduler"))
        .output()
        .unwrap_or_else(|e| panic!("run the scheduler binary: {e}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage"), "{stderr}");
}

fn test_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("ulaval-scheduler-e2e-{name}"));
    // leftovers from an earlier failed run
    let _ = std::fs::remove_dir_all(&dir);
    let cours = dir.join("cours");
    std::fs::create_dir_all(&cours)
        .unwrap_or_else(|e| panic!("create {}: {e}", cours.display()));
    std::fs::write(cours.join("a2026.json"), SNAPSHOT)
        .unwrap_or_else(|e| panic!("write the snapshot: {e}"));
    dir
}

fn cleanup(dir: &PathBuf) {
    std::fs::remove_dir_all(dir)
        .unwrap_or_else(|e| panic!("cleanup {}: {e}", dir.display()));
}
