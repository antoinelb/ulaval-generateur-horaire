// `asset!()` is a macro over a literal path: the list of program snapshots
// cannot be a runtime directory read. It is generated here instead, so
// `make ui-data` alone decides what ships and no hand-maintained manifest
// can fall behind the data (ADR `2026-08-manifeste-de-programmes-genere`).
use std::{env, fs, path::Path};

const PROGRAMS_DIR: &str = "assets/data/programmes";

fn main() {
    println!("cargo:rerun-if-changed={PROGRAMS_DIR}");
    let names = program_files();
    // native builds never see the assets: `browser.rs` is wasm32-only, so
    // an empty directory is normal there. Under wasm32 it would ship an app
    // with no program at all — a silent hole, so fail loudly instead.
    assert!(
        !(names.is_empty()
            && env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32")),
        "{PROGRAMS_DIR} is empty: run `make ui-data` before building for \
         the browser"
    );
    let entries = names
        .iter()
        .map(|name| {
            format!("    (\"{name}\", asset!(\"/{PROGRAMS_DIR}/{name}\")),\n")
        })
        .collect::<String>();
    let out = Path::new(&env::var("OUT_DIR").expect("cargo sets OUT_DIR"))
        .join("programmes.rs");
    fs::write(
        &out,
        format!("const PROGRAMS: &[(&str, Asset)] = &[\n{entries}];\n"),
    )
    .expect("writing the generated program manifest");
}

// every snapshot the scraper writes, sorted so the build is reproducible
// and `parse_data`'s first-wins deduplication is deterministic. The
// hand-maintained `*.manuel.json` files carry a `cheminement_type`, not a
// `Program`: fetching them would be a hard parse error.
fn program_files() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(PROGRAMS_DIR)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.ends_with(".json") && !name.ends_with(".manuel.json")
        })
        .collect();
    names.sort();
    names
}
