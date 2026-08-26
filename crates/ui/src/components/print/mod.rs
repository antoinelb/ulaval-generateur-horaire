// The print mechanism: a hidden-by-default view, revealed by a body class
// that `@media print` (assets/print.css) keys on. No business logic here —
// `organigramme::Sheet`/`horaire::Sheet` render whatever plan item 7/6 build.

pub mod horaire;
pub mod organigramme;

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintKind {
    Organigramme,
    Horaire,
}

pub const ORGANIGRAMME_CLASS: &str = "print-organigramme";
pub const HORAIRE_CLASS: &str = "print-horaire";

// Ordering matters and is deliberate: `window.print()` is synchronous and
// blocks until the dialog closes, so the DOM it captures must already be
// final. Two frames are awaited first — one lets the browser paint the
// button's own acknowledgement (LAT-3: within the 100 ms budget) before
// anything else runs, the second lets Dioxus commit the just-mounted
// `PrintView` tree — only then is the body class added (so `@media print`
// reveals the right sheet), `print()` called, and the class removed once
// the (now-closed) dialog returns control here.
pub fn start_print(mut target: Signal<Option<PrintKind>>, kind: PrintKind) {
    spawn(async move {
        target.set(Some(kind));
        crate::browser::next_frame().await;
        crate::browser::next_frame().await;
        let (class, name) = match kind {
            PrintKind::Organigramme => (ORGANIGRAMME_CLASS, "organigramme"),
            PrintKind::Horaire => (HORAIRE_CLASS, "horaire"),
        };
        // the browser names a saved PDF after `document.title` (adding
        // `.pdf` itself): swap it for the document's name while the dialog
        // is open, then restore the app's own title
        let app_title = crate::browser::document_title();
        crate::browser::set_document_title(name);
        crate::browser::add_body_class(class);
        crate::browser::print();
        crate::browser::remove_body_class(class);
        crate::browser::set_document_title(&app_title);
        target.set(None);
    });
}

// LAY-2: mounted only while a print is in flight, and invisible on screen
// even then (`.print-root { display: none }` outside `@media print`) — it
// must never disturb the on-screen layout.
#[component]
pub fn PrintView() -> Element {
    let target = use_context::<crate::components::PrintTarget>().0;
    let kind = target();
    let is_organigramme = kind == Some(PrintKind::Organigramme);
    let is_horaire = kind == Some(PrintKind::Horaire);
    rsx! {
        div { class: "print-root",
            if is_organigramme {
                organigramme::Sheet {}
            } else if is_horaire {
                horaire::Sheet {}
            }
        }
    }
}
