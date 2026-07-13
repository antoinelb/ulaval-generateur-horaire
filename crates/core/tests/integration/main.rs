// Entry point for the `integration` test binary; each submodule holds one
// unit's fixture-backed tests. Declared as a `[[test]]` target in Cargo.toml
// because Cargo only auto-discovers `.rs` files directly under `tests/`, not
// subdirectories.
mod catalogue;
mod course;
mod program;
