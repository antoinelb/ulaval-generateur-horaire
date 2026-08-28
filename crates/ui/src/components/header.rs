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
    let alerts = use_context::<Signal<Vec<Alert>>>();
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let snapshot = use_context::<Signal<Option<crate::data::Snapshot>>>();
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
            "{what} - "
            // LAY-2 : largeur réservée pour 3 chiffres (999 s ≈ 16 min,
            // bien au-delà d'une recherche réelle avant annulation) —
            // au-delà le texte s'élargirait quand même, ce cas est assumé
            span { class: "status-running-elapsed", "{elapsed} s" }
            button {
                class: "status-undo",
                onclick: move |_| {
                    super::cancel_search(
                        &handle, solver, plan, alerts, manual, snapshot,
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
    // the Signal itself, kept from the shadowing below: « changer » swaps
    // documents and needs the signals, not one render's borrow
    let snapshot_signal = snapshot;
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
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
    // and the chosen one stays named on screen, concentration et profil
    // compris (rapport étudiante ; décision 2026-08-19)
    let chosen = crate::panel::chosen_program(snapshot, &plan.read());
    let program_title = crate::panel::program_subtitle(snapshot, &plan.read())
        .unwrap_or_else(|| "aucun programme choisi".to_string());
    let has_program = chosen.is_some();
    let history = use_context::<Signal<crate::state::History>>();
    // note 8: the whole bac's tally next to the session's — counted by
    // `ui-calculations`, en-sus and préparatoire credits kept out
    let bac = chosen.map(|program| {
        let plan_read = plan.read();
        let granted = crate::panel::effective_program(snapshot, &plan_read);
        let (concentration, profile) = crate::panel::scope_of(&plan_read);
        let summary = ulaval_scheduler_wasm::credits::credit_summary(
            granted.as_ref(),
            concentration,
            profile,
            &crate::panel::selection(&plan_read),
            &snapshot.courses,
        );
        let note = crate::present::bac_credit_note(&summary);
        (summary.counted, program.credits_required, note)
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
                    title: "Revenir au choix de programme — ce cheminement \
                            est conservé et revient en le rechoisissant",
                    onclick: {
                        let handle = handle.clone();
                        move |_| {
                            // the document goes to its shelf whole; the
                            // picker takes over with an empty grid (US-10)
                            let swap = crate::persist::leave_document(
                                &plan.peek(),
                            );
                            super::swap_document(
                                plan,
                                view,
                                history,
                                alerts,
                                solver,
                                &handle,
                                manual,
                                snapshot_signal,
                                swap,
                            );
                        }
                    },
                    "changer"
                }
            }
            span { class: "header-notice",
                "⚠ Horaires à titre indicatif, d'après le site web de \
                 l'Université. Vérifiez-les et revalidez l'organigramme \
                 à chaque session au cas où des horaires de cours \
                 auraient changé."
            }
            span { class: "header-credits",
                if let Some((counted, required, note)) = bac {
                    span {
                        title: "{note.tooltip}",
                        "{counted}/{required} cr au bac{note.suffix} - "
                    }
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
            ResetButton {}
        }
    }
}

// « Réinitialiser » : one click, undoable — never a confirmation (ACT-2).
// The current document goes back to zero through the history, and its
// shelf copy goes with it — re-choosing the program must not resurrect the
// pre-reset grid; the other programs' shelves survive (ADR
// `2026-08-reinitialiser-le-document-courant-et-son-etagere`). The
// hand-entered course fiches stay, they extend the catalogue, not the
// document — an undo may restore a plan that references them (ADR
// `2026-08-bouton-tout-reinitialiser`).
#[component]
fn ResetButton() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let alerts = use_context::<Signal<Vec<Alert>>>();
    let solver = use_context::<Signal<SolverState>>();
    let handle = use_context::<SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let snapshot = use_context::<Signal<Option<crate::data::Snapshot>>>();
    rsx! {
        button {
            class: "status-undo",
            title: "Repartir de zéro — ce programme seulement, annulable \
                    avec « Annuler »",
            onclick: move |_| {
                // a search in flight must not land its proposal in the
                // fresh plan
                if solver.peek().running.is_some() {
                    super::cancel_search(
                        &handle, solver, plan, alerts, manual, snapshot,
                    );
                }
                // a shared link left in the address bar would reimport
                // everything at the next reload
                crate::browser::strip_query();
                // materialized before the edit erases the program: the
                // shelf copy must go too, or re-choosing the program
                // would resurrect the pre-reset grid
                let shelf = plan
                    .peek()
                    .program
                    .as_ref()
                    .map(crate::persist::snapshot_key);
                super::edit_plan(plan, history, "Réinitialisation", |plan| {
                    *plan = Plan::default();
                });
                if let Some(key) = shelf {
                    crate::browser::local_remove(&key);
                }
                view.set(View::default());
                super::push_alert(
                    alerts,
                    AlertBody::Success(
                        "Ce programme a été réinitialisé — « Annuler » \
                         restaure votre organigramme."
                            .to_string(),
                    ),
                );
            },
            "Réinitialiser"
        }
    }
}

// The reserved status region (ALR-6): always present, same place, one
// line tall forever — the undo/redo controls, then the solver status. The
// buttons come first so the status's appearance and disappearance never has
// anything of its own to displace (LAY-2). The alerts float apart in
// `Toasts`, so this strip never pushes the panel or the grid down.
#[component]
pub fn StatusStrip() -> Element {
    let mut plan = use_context::<Signal<Plan>>();
    let mut history = use_context::<Signal<History>>();
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
                kbd { "Ctrl+Z" }
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
                kbd { "Ctrl+Y" }
            }
            SolverStatus {}
        }
    }
}

// Les alertes flottent en coin bas-droite, par-dessus la grille — le
// panneau et l'horaire ne bougent jamais (ADR
// `2026-08-alertes-en-toasts-flottants`). Les ⚠ persistent jusqu'au clic
// (ALR-4) ; seuls les ✓ s'auto-effacent ; au-delà de TOASTS_VISIBLE la
// pile se résume en « +N autres » et se déplie sur demande (ALR-3).
const TOASTS_VISIBLE: usize = 3;
const SUCCESS_TOAST_MS: u32 = 5_000;

#[component]
pub fn Toasts() -> Element {
    let mut alerts = use_context::<Signal<Vec<Alert>>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let super::LocalPrograms(local_programs) =
        use_context::<super::LocalPrograms>();
    let mut expanded = use_signal(|| false);
    // chaque ✓ n'arme qu'une seule minuterie (peek : l'effet ne dépend
    // que de la liste, jamais de sa propre comptabilité)
    let mut timed = use_signal(std::collections::BTreeSet::<u64>::new);
    use_effect(move || {
        let successes: Vec<u64> = alerts
            .read()
            .iter()
            .filter(|alert| matches!(alert.body, AlertBody::Success(_)))
            .map(|alert| alert.key)
            .collect();
        for key in successes {
            if timed.peek().contains(&key) {
                continue;
            }
            timed.write().insert(key);
            let mut alerts = alerts;
            spawn(async move {
                crate::browser::sleep_ms(SUCCESS_TOAST_MS).await;
                alerts.write().retain(|kept| kept.key != key);
            });
        }
    });
    let all = alerts.read().clone();
    if all.is_empty() {
        return rsx! {};
    }
    let hidden = all.len().saturating_sub(TOASTS_VISIBLE);
    let show_all = expanded() || hidden == 0;
    let first_visible = if show_all {
        0
    } else {
        all.len() - TOASTS_VISIBLE
    };
    rsx! {
        div { class: "toasts", role: "status",
            if !show_all {
                button {
                    class: "toast toast--more",
                    onclick: move |_| expanded.set(true),
                    if hidden == 1 {
                        "+1 autre message - tout afficher"
                    } else {
                        "+{hidden} autres messages - tout afficher"
                    }
                }
            }
            for alert in all[first_visible..].iter().cloned() {
                div {
                    key: "{alert.key}",
                    class: "toast",
                    class: if matches!(
                        alert.body,
                        AlertBody::Success(_)
                            | AlertBody::LocalProgramRemoved(_)
                    ) {
                        "toast--success"
                    },
                    // note 12: the whole message is its own dismiss —
                    // any link inside would stop the propagation
                    onclick: {
                        let key = alert.key;
                        move |_| {
                            alerts.write().retain(|kept| kept.key != key);
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
                        AlertBody::LocalProgramRemoved(local) => {
                            let local = local.clone();
                            let key = alert.key;
                            rsx! {
                                span { class: "status-alert-ok",
                                    "✓ Programme supprimé — "
                                }
                                button {
                                    class: "toast-undo",
                                    onclick: move |event: Event<MouseData>| {
                                        // the toast is its own dismiss
                                        // (note 12) — undo must not also
                                        // trigger it before it has had a
                                        // chance to read `local`
                                        event.stop_propagation();
                                        super::restore_local_program(
                                            snapshot,
                                            local_programs,
                                            alerts,
                                            local.clone(),
                                        );
                                        alerts
                                            .write()
                                            .retain(|kept| kept.key != key);
                                    },
                                    "↶ Annuler"
                                }
                            }
                        }
                    }
                    button {
                        class: "status-dismiss",
                        aria_label: "Rejeter ce message",
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
