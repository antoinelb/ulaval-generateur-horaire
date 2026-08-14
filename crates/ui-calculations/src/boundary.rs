// The worker's wasm boundary — browser-only glue, no logic (the same
// pattern as `wasm/src/boundary.rs`). The shim `worker.js` fetches the
// snapshot once, hands it to `init_snapshot` with the student's manual
// courses, then funnels every message through `handle_message`.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use ulaval_scheduler_core::Course;

use crate::merge::merge_manual;
use crate::protocol::handle;

thread_local! {
    static SNAPSHOT: RefCell<Vec<Course>> = const { RefCell::new(Vec::new()) };
}

#[derive(serde::Deserialize)]
struct Snapshot {
    courses: Vec<Course>,
}

// Parse once, keep for every later request. Returns the merged catalogue
// size; collisions are the caller's to display, so they come back too.
#[wasm_bindgen]
pub fn init_snapshot(
    snapshot_json: &str,
    manual_json: &str,
) -> Result<String, JsValue> {
    let snapshot: Snapshot = serde_json::from_str(snapshot_json)
        .map_err(|e| JsValue::from_str(&format!("snapshot : {e}")))?;
    let manual: Vec<Course> = serde_json::from_str(manual_json)
        .map_err(|e| JsValue::from_str(&format!("manual courses : {e}")))?;
    let merged = merge_manual(snapshot.courses, manual);
    let summary = serde_json::json!({
        "course_count": merged.courses.len(),
        "collisions": merged.collisions,
    });
    SNAPSHOT.with(|cell| *cell.borrow_mut() = merged.courses);
    // expect over `?`: serializing a number and strings provably cannot fail
    Ok(serde_json::to_string(&summary)
        .expect("Summary serialization always succeeds"))
}

#[wasm_bindgen]
pub fn handle_message(request: &str) -> String {
    SNAPSHOT.with(|cell| handle(request, &cell.borrow()))
}
