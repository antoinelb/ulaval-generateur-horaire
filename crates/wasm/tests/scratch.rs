// scratch diagnostic — deleted before commit
use ulaval_scheduler_wasm::organigramme::{generate, OrganigrammeInput};

#[test]
fn bgmc_ui_shaped_request() {
    let courses_raw =
        std::fs::read_to_string("../../data/cours.json").expect("cours");
    let courses_value: serde_json::Value =
        serde_json::from_str(&courses_raw).expect("json");
    let courses: Vec<ulaval_scheduler_core::Course> =
        serde_json::from_value(courses_value["courses"].clone())
            .expect("courses");
    let program_raw =
        std::fs::read_to_string("../../data/programmes/B-GMC-H27.json")
            .expect("program");
    for extra in [
        r#""#,
        r#""concentration":"Cheminement sans concentration","#,
    ] {
        let input: OrganigrammeInput = serde_json::from_str(&format!(
            r#"{{"program":{program_raw},{extra}"start":"fall",
                 "study_sessions":8,"credit_cap":17,
                 "max_nodes":200000}}"#
        ))
        .expect("input");
        let t = std::time::Instant::now();
        let report = generate(&input, &courses).expect("generate");
        let solution = report.placement.solutions.first();
        println!(
            "extra={extra:?} -> completion={:?} solutions={} placed={} left_out={} blocked={} in {:?}",
            report.placement.completion,
            report.placement.solutions.len(),
            solution.map(|s| s.placement.len()).unwrap_or(0),
            solution.map(|s| s.left_out.len()).unwrap_or(0),
            report.placement.blocked.len(),
            t.elapsed(),
        );
    }
}
