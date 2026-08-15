use dioxus::prelude::*;

use super::{
    grid, header, panel, ribbon, DraggedCourse, LoadState, SelectedCourse,
};
use crate::data::Snapshot;
use crate::present::UiError;

#[component]
pub fn Screen() -> Element {
    let state = use_context::<Signal<LoadState>>();
    match state() {
        LoadState::Downloading => rsx! {
            Loading { phase: "téléchargement du catalogue".to_string() }
        },
        LoadState::Parsing => rsx! {
            Loading { phase: "analyse du catalogue".to_string() }
        },
        LoadState::Failed(error) => rsx! {
            Failure { error }
        },
        LoadState::Ready => rsx! {
            Shell {}
        },
    }
}

// LAT-5/LAT-4: an explicit loading state — never a skeleton, never a bare
// spinner: the phase and the elapsed seconds say « slow », not « dead ».
#[component]
fn Loading(phase: String) -> Element {
    let mut elapsed = use_signal(|| 0u32);
    use_future(move || async move {
        // bounded: four hours of ticks, far past any real load
        for _ in 0..14_400u32 {
            crate::browser::sleep_ms(1_000).await;
            *elapsed.write() += 1;
        }
    });
    rsx! {
        main { class: "load-screen",
            h1 { "Générateur d'horaire" }
            p { class: "load-phase", "Chargement — {phase}…" }
            p { class: "load-elapsed", "{elapsed} s écoulées" }
        }
    }
}

// ERR-1: the five parts, in order, with the copyable id and the technical
// detail one click away (ERR-3) — never a blank screen
#[component]
fn Failure(error: UiError) -> Element {
    rsx! {
        main { class: "error-screen", role: "alert",
            h1 { "Le chargement a échoué" }
            p { class: "error-what", "{error.what}" }
            p { "{error.reaction}" }
            p { "{error.affected}" }
            p { class: "error-action", "{error.action}" }
            p { class: "error-id",
                "Identifiant : "
                code { "{error.id}" }
            }
            details {
                summary { "Détail technique" }
                pre { "{error.detail}" }
            }
        }
    }
}

// The one screen (LAY-1: every region always in the same place):
// header / session ribbon / status strip / (left panel | weekly grid) /
// provenance footer.
#[component]
fn Shell() -> Element {
    let mut selected = use_signal(|| None::<String>);
    use_context_provider(|| SelectedCourse(selected));
    let dragged = use_signal(|| None::<String>);
    use_context_provider(|| DraggedCourse(dragged));
    let plan = use_context::<Signal<crate::state::Plan>>();
    let history = use_context::<Signal<crate::state::History>>();
    rsx! {
        div {
            class: "shell",
            // Échap backs out of the ghost display wherever focus sits;
            // Ctrl+Z / Ctrl+Maj+Z walk the labelled history (LAY-5 : the
            // shortcut doubles the visible Annuler button, never replaces it)
            onkeydown: move |event| {
                if event.key() == Key::Escape {
                    selected.set(None);
                    return;
                }
                if !event.modifiers().ctrl() {
                    return;
                }
                if let Key::Character(letter) = event.key() {
                    let mut plan = plan;
                    let mut history = history;
                    match letter.as_str() {
                        "z" => {
                            let mut plan = plan.write();
                            let mut history = history.write();
                            crate::state::undo(&mut plan, &mut history);
                        }
                        "Z" | "y" => {
                            let mut plan = plan.write();
                            let mut history = history.write();
                            crate::state::redo(&mut plan, &mut history);
                        }
                        _ => {}
                    }
                }
            },
            header::HeaderBar {}
            ribbon::SessionRibbon {}
            header::StatusStrip {}
            div { class: "main-split",
                panel::LeftPanel {}
                grid::WeeklyGrid {}
            }
            Footer {}
            header::Toasts {}
        }
    }
}

// BLD-4: version, build hash and data provenance — visible on screen and
// carried by any screenshot
#[component]
fn Footer() -> Element {
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let scraped = snapshot
        .provenance
        .scraped_at
        .clone()
        .unwrap_or_else(|| "date de récolte inconnue".to_string());
    let build = option_env!("BUILD_HASH").unwrap_or("dev");
    let version = env!("CARGO_PKG_VERSION");
    rsx! {
        footer { class: "footer",
            p {
                "v{version} - build {build} - "
                "{snapshot.provenance.course_count} cours - données : "
                "{scraped} - empreinte "
                code { "{snapshot.provenance.data_hash}" }
            }
        }
    }
}
