use dioxus::prelude::*;

use super::{Alert, AlertBody, SolverHandle, SolverState};
use crate::data::Snapshot;
use crate::solve;
use crate::state::{self, History, Plan, View};

// LAT-4: a visible search is never a bare spinner — elapsed seconds and a
// real cancel (the worker dies, a fresh one boots)
#[component]
fn SolverStatus() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let alerts = use_context::<Signal<Vec<Alert>>>();
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let mut now_ms = use_signal(crate::browser::now_epoch_ms);
    use_future(move || async move {
        // bounded ticker; idles harmlessly when nothing runs
        for _ in 0..86_400u32 {
            crate::browser::sleep_ms(1_000).await;
            now_ms.set(crate::browser::now_epoch_ms());
        }
    });
    let Some(running) = solver.read().running else {
        return rsx! {};
    };
    let elapsed = now_ms().saturating_sub(running.started_ms) / 1_000;
    let what = match running.kind {
        super::QueryKind::Propose => "recherche d'un organigramme",
        super::QueryKind::Verify => "vérification du cheminement",
    };
    rsx! {
        span { class: "status-running",
            "{what} - {elapsed} s"
            button {
                class: "status-undo",
                onclick: move |_| {
                    super::cancel_search(
                        &handle, solver, plan, history, alerts, manual,
                    );
                },
                "Annuler la recherche"
            }
        }
    }
}

#[component]
pub fn HeaderBar() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let alerts = use_context::<Signal<Vec<Alert>>>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    // note 9: the link carries the whole organigramme — the button lives
    // up here because its scope is everything, not one session's grid
    let share = move |_| {
        let (url, payload) = {
            let plan_read = plan.read();
            let manual_read = manual.read();
            let payload =
                crate::persist::encode_organigramme(&plan_read, &manual_read);
            (crate::browser::share_url(&payload), payload)
        };
        crate::browser::clipboard_write(&url);
        // the link also lands in the address bar — copyable there even if
        // the clipboard was blocked, without flooding the status strip
        crate::browser::set_fragment(&payload);
        super::push_alert(
            alerts,
            AlertBody::Success(
                "Lien copié (aussi dans la barre d'adresse) — il rouvre \
                 tout l'organigramme tel quel."
                    .to_string(),
            ),
        );
    };
    // (code, millésime) — two vintages of one program are two programs,
    // and the chosen one stays named on screen (rapport étudiante)
    let chosen = crate::panel::chosen_program(snapshot, &plan.read());
    let program_title = chosen
        .map(|program| {
            format!(
                "{} ({} version {})",
                program.title, program.code, program.semester
            )
        })
        .unwrap_or_else(|| "aucun programme choisi".to_string());
    let has_program = chosen.is_some();
    let history = use_context::<Signal<crate::state::History>>();
    // note 8: the whole bac's tally next to the session's — counted by
    // `ui-calculations`, en-sus and préparatoire credits kept out
    let bac = chosen.map(|program| {
        let plan_read = plan.read();
        let granted = crate::panel::effective_program(snapshot, &plan_read);
        let summary =
            ulaval_scheduler_ui_calculations::credits::credit_summary(
                granted.as_ref(),
                &crate::panel::selection(&plan_read),
                &snapshot.courses,
            );
        (summary.counted, program.credits_required)
    });
    let credits =
        solve::session_credits(snapshot, &plan.read(), view.read().session);
    // a Range counted at its lower bound says so (TRU-6: never a false
    // precision)
    let range_mark = if credits.has_range { " (min.)" } else { "" };
    let cap = plan.read().credit_cap;
    let over_cap = credits.total > cap;
    rsx! {
        header { class: "header-bar",
            div { class: "header-logo", aria_hidden: true }
            h1 { "Générateur d'horaire" }
            span { class: "header-subtitle", "{program_title}" }
            if has_program {
                button {
                    class: "status-undo",
                    title: "Revenir au choix de programme (annulable)",
                    onclick: move |_| {
                        super::edit_plan(
                            plan,
                            history,
                            "Choix de programme rouvert",
                            |plan| plan.program = None,
                        );
                    },
                    "changer"
                }
            }
            span { class: "header-credits",
                if let Some((counted, required)) = bac {
                    span { "{counted}/{required} cr au bac - " }
                }
                b {
                    class: if over_cap { "header-credits--over" },
                    "{credits.total} cr{range_mark} cette session"
                }
                if over_cap {
                    span { class: "header-credits--over",
                        " ⚠ plafond de {cap} cr dépassé"
                    }
                }
            }
            button {
                class: "grid-share",
                title: "Copier un lien qui rouvre tout l'organigramme",
                onclick: share,
                "Partager"
            }
        }
    }
}

// The reserved status region (ALR-6): always present, same place, holding
// the undo/redo controls and every alert until its explicit dismissal
// (ALR-4). Expansion pushes content down, never reorders it (LAY-2).
#[component]
pub fn StatusStrip() -> Element {
    let mut plan = use_context::<Signal<Plan>>();
    let mut history = use_context::<Signal<History>>();
    let mut alerts = use_context::<Signal<Vec<Alert>>>();
    let undo_label = history
        .read()
        .undo_label()
        .map(|label| format!("Annuler : {label}"))
        .unwrap_or_else(|| "Rien à annuler".to_string());
    let redo_label = history
        .read()
        .redo_label()
        .map(|label| format!("Rétablir : {label}"))
        .unwrap_or_else(|| "Rien à rétablir".to_string());
    let can_undo = history.read().undo_label().is_some();
    let can_redo = history.read().redo_label().is_some();
    rsx! {
        div { class: "status-strip", role: "status",
            SolverStatus {}
            button {
                class: "status-undo",
                disabled: !can_undo,
                title: "{undo_label}",
                onclick: move |_| {
                    let mut plan = plan.write();
                    let mut history = history.write();
                    state::undo(&mut plan, &mut history);
                },
                "↶ Annuler"
            }
            button {
                class: "status-undo",
                disabled: !can_redo,
                title: "{redo_label}",
                onclick: move |_| {
                    let mut plan = plan.write();
                    let mut history = history.write();
                    state::redo(&mut plan, &mut history);
                },
                "↷ Rétablir"
            }
            ul { class: "status-alerts",
                for alert in alerts.read().iter().cloned() {
                    li {
                        key: "{alert.key}",
                        class: "status-alert",
                        // note 12: the whole message is its own dismiss —
                        // any link inside would stop the propagation
                        onclick: {
                            let key = alert.key;
                            move |_| {
                                alerts
                                    .write()
                                    .retain(|kept| kept.key != key);
                            }
                        },
                        match &alert.body {
                            AlertBody::Note(note) => rsx! {
                                span { "⚠ {note}" }
                            },
                            AlertBody::Success(note) => rsx! {
                                span { class: "status-alert-ok", "✓ {note}" }
                            },
                            AlertBody::Error(error) => rsx! {
                                span { class: "status-alert-error",
                                    "⚠ {error.what} {error.action} "
                                    code { "{error.id}" }
                                }
                            },
                        }
                        button {
                            class: "status-dismiss",
                            aria_label: "Rejeter cette alerte",
                            onclick: move |_| {
                                alerts
                                    .write()
                                    .retain(|kept| kept.key != alert.key);
                            },
                            "✕"
                        }
                    }
                }
            }
        }
    }
}
