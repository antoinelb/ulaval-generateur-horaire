// Browser-only IO glue — no logic, the `wasm/src/boundary.rs` pattern.
// Everything here is wasm32-only: `make static` lints it on that target.

use dioxus::prelude::*;

use crate::data::{DataError, RawData};

const COURSES: Asset = asset!("/assets/data/cours.json");
const META: Asset = asset!("/assets/data/meta.json");
const MANUAL: Asset = asset!("/assets/data/cours.manuel.json");
// `asset!()` is compile-time, so the manifest cannot read the directory at
// runtime: `build.rs` generates it from whatever `make ui-data` copied —
// adding a snapshot needs no code change (ADR
// `2026-08-manifeste-de-programmes-genere`)
include!(concat!(env!("OUT_DIR"), "/programmes.rs"));

pub async fn fetch_raw_data() -> Result<RawData, DataError> {
    let courses = fetch_text(&COURSES.to_string(), "cours.json").await?;
    // the meta is auxiliary: a failed fetch degrades to « date inconnue »
    // in `parse_data` instead of blocking the app (ERR-5)
    let meta = fetch_text(&META.to_string(), "meta.json").await.ok();
    // same rule for the hand-maintained catalogue: without it the app runs
    // on the scraped snapshot alone, which is a smaller answer, not a
    // broken one
    let manual = fetch_text(&MANUAL.to_string(), "cours.manuel.json")
        .await
        .ok();
    let mut programs = Vec::new();
    for (name, asset) in PROGRAMS {
        programs.push((
            name.to_string(),
            fetch_text(&asset.to_string(), name).await?,
        ));
    }
    Ok(RawData {
        courses,
        meta,
        manual,
        programs,
    })
}

async fn fetch_text(url: &str, file: &str) -> Result<String, DataError> {
    let fetch_error = |detail: String| DataError::Fetch {
        file: file.to_string(),
        detail,
    };
    let response = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|error| fetch_error(error.to_string()))?;
    if !response.ok() {
        return Err(fetch_error(format!("HTTP {}", response.status())));
    }
    response
        .text()
        .await
        .map_err(|error| fetch_error(error.to_string()))
}

// one macrotask: lets the browser paint the just-written state before a
// blocking computation starts (LAT-1: the acknowledgement lands first)
pub async fn next_frame() {
    gloo_timers::future::TimeoutFuture::new(0).await;
}

pub async fn sleep_ms(ms: u32) {
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

// --- localStorage ----------------------------------------------------------

fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|window| window.local_storage().ok().flatten())
}

pub fn local_get(key: &str) -> Option<String> {
    storage().and_then(|storage| storage.get_item(key).ok().flatten())
}

// a full or blocked storage must never crash the app: saving is
// best-effort, the in-memory state stays the truth
pub fn local_set(key: &str, value: &str) {
    if let Some(storage) = storage() {
        storage.set_item(key, value).ok();
    }
}

// removal is as best-effort as saving: an absent key is already the goal
pub fn local_remove(key: &str) {
    if let Some(storage) = storage() {
        storage.remove_item(key).ok();
    }
}

// a damaged save is copied under a fresh key before anything overwrites it
pub fn stash_backup(raw: &str) {
    local_set(&format!("gh.backup.{}", now_epoch_ms()), raw);
}

// The debounced save's last window: a reload right after an edit would
// lose it (rapport étudiante 2026-08-14) — flush on the way out.
// `pagehide` over `beforeunload`: it also fires on mobile tab discard.
pub fn on_page_hide(callback: impl FnMut() + 'static) {
    use wasm_bindgen::JsCast;
    let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(callback);
    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback(
                "pagehide",
                closure.as_ref().unchecked_ref(),
            )
            .ok();
    }
    // the listener lives as long as the page — never dropped
    closure.forget();
}

pub fn now_epoch_ms() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

// --- URL sharing -----------------------------------------------------------

// the `#…` payload of the current address, if any — the fragment never
// reaches any server (ADR
// `2026-08-partage-de-lorganigramme-complet-en-fragment`)
pub fn location_share() -> Option<String> {
    let hash = web_sys::window()?.location().hash().ok()?;
    let payload = hash.strip_prefix('#')?;
    if payload.is_empty() {
        None
    } else {
        Some(payload.to_string())
    }
}

// drop the fragment so a reload does not re-import (the import is undoable
// and idempotent anyway — belt and suspenders)
pub fn strip_query() {
    if let Some(window) = web_sys::window() {
        let path = window
            .location()
            .pathname()
            .unwrap_or_else(|_| "/".to_string());
        if let Ok(history) = window.history() {
            history
                .replace_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some(&path),
                )
                .ok();
        }
    }
}

// the share payload into the address bar (no history entry): copyable
// there even when the clipboard is blocked
pub fn set_fragment(payload: &str) {
    if let Some(window) = web_sys::window() {
        let path = window
            .location()
            .pathname()
            .unwrap_or_else(|_| "/".to_string());
        if let Ok(history) = window.history() {
            history
                .replace_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some(&format!("{path}#{payload}")),
                )
                .ok();
        }
    }
}

// the absolute link to put in a message: origin + path + #… (the payload
// is already base64url — every character is fragment-legal)
pub fn share_url(payload: &str) -> String {
    let base = web_sys::window()
        .and_then(|window| {
            let location = window.location();
            let origin = location.origin().ok()?;
            let path = location.pathname().ok()?;
            Some(format!("{origin}{path}"))
        })
        .unwrap_or_else(|| "/".to_string());
    format!("{base}#{payload}")
}

pub fn clipboard_write(text: &str) {
    if let Some(window) = web_sys::window() {
        // fire-and-forget: the UI also shows the link, so a blocked
        // clipboard loses nothing
        let _ = window.navigator().clipboard().write_text(text);
    }
}

pub fn encode_uri(text: &str) -> String {
    String::from(js_sys::encode_uri_component(text))
}

// --- offline (DEG-3) -------------------------------------------------------

// A service worker's scope is the directory it is served from, and
// `asset!()` would put it under `/assets/` — where it could cache the data
// but never control the page. A relative URL resolves against the page
// instead, so the scope is the whole app at any base path; the deploy drops
// the file beside the index (ADR
// `2026-08-interface-publiee-a-la-racine-de-pages`).
const SW_URL: &str = "sw.js";

// best-effort: telemetry-grade plumbing never blocks the app (OBS-6). Under
// `dx serve` the file is not beside the index, so registration fails
// silently — offline is a deployed-site feature.
pub fn register_service_worker() {
    if let Some(window) = web_sys::window() {
        let _ = window.navigator().service_worker().register(SW_URL);
    }
}

// --- the solver worker (AIR LAT-3: solver B never blocks this thread) -----

const WORKER_JS: Asset = asset!("/assets/worker.js");
const CALC_JS: Asset = asset!("/assets/calc/calc.js");
const CALC_WASM: Asset = asset!("/assets/calc/calc_bg.wasm");

pub struct Solver {
    worker: web_sys::Worker,
    // kept alive: dropping the closure would detach onmessage
    _on_message:
        wasm_bindgen::closure::Closure<dyn FnMut(web_sys::MessageEvent)>,
}

// Boot a fresh worker: it loads the calc module and the snapshot itself
// (HTTP cache — the 8,6 Mo never ride a postMessage), then answers request
// strings. `on_message` receives every answer, ready/error included.
pub fn spawn_solver(
    manual_json: &str,
    overrides_json: &str,
    mut on_message: impl FnMut(String) + 'static,
) -> Option<Solver> {
    let options = web_sys::WorkerOptions::new();
    options.set_type(web_sys::WorkerType::Module);
    let worker =
        web_sys::Worker::new_with_options(&WORKER_JS.to_string(), &options)
            .ok()?;
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
        move |event: web_sys::MessageEvent| {
            if let Some(text) = event.data().as_string() {
                on_message(text);
            }
        },
    )
        as Box<dyn FnMut(web_sys::MessageEvent)>);
    use wasm_bindgen::JsCast;
    worker.set_onmessage(Some(closure.as_ref().unchecked_ref()));
    send_init(&worker, manual_json, overrides_json);
    Some(Solver {
        worker,
        _on_message: closure,
    })
}

fn send_init(
    worker: &web_sys::Worker,
    manual_json: &str,
    overrides_json: &str,
) {
    let message = js_sys::Object::new();
    let set = |key: &str, value: &str| {
        js_sys::Reflect::set(
            &message,
            &wasm_bindgen::JsValue::from_str(key),
            &wasm_bindgen::JsValue::from_str(value),
        )
        .ok();
    };
    set("calcJs", &CALC_JS.to_string());
    set("calcWasm", &CALC_WASM.to_string());
    set("coursesUrl", &COURSES.to_string());
    // the hand-maintained Courses join the worker's catalogue too — the
    // repo's and the student's alike, so both catalogues hold the same list
    set("manualJson", manual_json);
    // and the prerequisites his program vintage rewrote, applied there
    // exactly as they are on the main thread
    set("overridesJson", overrides_json);
    worker.post_message(&message).ok();
}

impl Solver {
    pub fn send(&self, request: &str) {
        self.worker
            .post_message(&wasm_bindgen::JsValue::from_str(request))
            .ok();
    }

    // a real cancel: the crunching worker dies, the caller boots a new one
    pub fn terminate(&self) {
        self.worker.terminate();
    }
}
