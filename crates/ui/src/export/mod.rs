// Pure document models for the two PDF exports (organigramme, horaire):
// no Dioxus, no web-sys, tested natively. The view components in
// `components/print/` render these models to HTML/CSS for `window.print()`.

pub mod horaire;
pub mod organigramme;
pub mod provenance;
