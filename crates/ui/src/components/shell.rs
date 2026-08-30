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
    let hover = use_signal(|| None::<usize>);
    use_context_provider(|| super::DropHover(hover));
    let plan = use_context::<Signal<crate::state::Plan>>();
    let history = use_context::<Signal<crate::state::History>>();
    rsx! {
        div {
            class: "shell",
            // Échap backs out of the ghost display wherever focus sits;
            // Ctrl+Z / Ctrl+Y walk the labelled history (LAY-5 : the
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
                        "y" => {
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
            // la bande porte sa zone de retrait en frère du `role="status"`,
            // jamais en enfant (ADR `2026-08-retrait-par-glissement`)
            div { class: "status-band",
                header::StatusStrip {}
                header::RemovalDropZone {}
            }
            div { class: "main-split",
                panel::LeftPanel {}
                grid::WeeklyGrid {}
            }
            Footer {}
            header::Toasts {}
        }
    }
}

const REPO: &str = "https://github.com/antoinelb/ulaval-generateur-horaire";

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
    // the site deploys a tag's code with main's data, so the two commits
    // differ and each needs naming (ADR
    // `2026-08-le-pied-nomme-les-donnees-par-leur-commit`)
    let data = option_env!("DATA_HASH").unwrap_or("dev");
    let version = env!("CARGO_PKG_VERSION");
    rsx! {
        footer { class: "footer",
            p {
                "v{version} - code "
                Commit { sha: build }
                " - données : "
                "{scraped}, "
                Commit { sha: data }
            }
            // Le seul point de contact de l'application : sans lui, un
            // étudiant qui trouve un bogue n'a nulle part où le dire.
            // Écart INP-1 assumé : les cibles font ~28 px, pas 48 — la
            // bande reste compacte pour ne pas voler de hauteur à la
            // grille.
            p { class: "footer-contact",
                "Pour tout problème, contacter "
                a { href: "mailto:antoinelb@proton.me",
                    "antoinelb@proton.me"
                }
                " ou créer un issue à "
                a {
                    href: "{REPO}",
                    target: "_blank",
                    rel: "noopener",
                    "{REPO}"
                }
            }
        }
    }
}

// A commit is only provenance if it resolves: linked to its GitHub page
// so a screenshot leads to the exact code or data that produced it. A
// local build has no commit to point at — « dev » stays plain text
// rather than becoming a link that goes nowhere (TRU-1: never claim more
// than is known).
#[component]
fn Commit(sha: String) -> Element {
    if sha == "dev" {
        return rsx! {
            code { "{sha}" }
        };
    }
    rsx! {
        a {
            href: "{REPO}/commit/{sha}",
            target: "_blank",
            rel: "noopener",
            title: "Ouvre ce commit sur GitHub",
            code { "{sha}" }
        }
    }
}
