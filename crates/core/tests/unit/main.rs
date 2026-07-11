// Entry point for the `unit` test binary; each submodule holds one unit's
// tests. Declared as a `[[test]]` target in Cargo.toml because Cargo only
// auto-discovers `.rs` files directly under `tests/`, not subdirectories.
mod course;
