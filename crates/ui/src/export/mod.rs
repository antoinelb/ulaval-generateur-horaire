// Pure document models for the exports (organigramme, horaire): no Dioxus,
// no web-sys, tested natively. The view components in `components/print/`
// render the PDF models to HTML/CSS for `window.print()`; `menu` is the
// table of what the « Exporter » control offers.

pub mod horaire;
pub mod menu;
pub mod organigramme;
pub mod provenance;
