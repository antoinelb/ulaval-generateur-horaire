// scraper-level wiremock tests for fetch/pagination logic, never part of the
// parser move (the parser round-trip tests live in
// crates/core/tests/integration/parser_catalogue.rs)
mod catalogue;
mod cli;
mod fetch;
mod manual;
