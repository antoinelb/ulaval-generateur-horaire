// the binary has no native life — tests and lints run against the library

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus::launch(ulaval_scheduler_ui::components::App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
