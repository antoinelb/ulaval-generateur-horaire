use dioxus::prelude::*;

use super::{edit_plan, print, SelectedCourse};
use crate::data::Snapshot;
use crate::present::{self, Block};
use crate::solve;
use crate::state::{self, History, Plan, View};

// The weekly grid: absolute-positioned blocks inside relative day columns
// (the design's own technique — CSS grid rows cannot host the half-width
// conflict lanes). Every block is a real button: keyboard reachable
// (INP-4), activated by click or Enter, never by hover (INP-5).
#[component]
pub fn WeeklyGrid() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let SelectedCourse(selected) = use_context::<SelectedCourse>();
    let schedule = use_memo(move || {
        let read = snapshot.read();
        let Some(snapshot) = read.as_ref() else {
            return solve::WeeklySchedule {
                report: ulaval_scheduler_core::ScheduleReport {
                    valid: true,
                    courses: Vec::new(),
                },
                excluded: Vec::new(),
                notes: Vec::new(),
            };
        };
        solve::weekly_schedule(snapshot, &plan.read(), view.read().session)
    });
    let grid = use_memo(move || {
        let read = snapshot.read();
        let snapshot = read.as_ref()?;
        Some(present::grid_model(
            &schedule.read(),
            snapshot,
            selected.read().as_deref(),
        ))
    });
    let session = view.read().session;
    let title = solve::session_semester(&plan.read(), session)
        .map(|semester| {
            let long = long_semester(semester);
            // « H4 » + « Hiver 2026 » say it all — the word « Horaire »
            // said nothing the grid below doesn't, and repeating « H26 »
            // between them said it twice (note 15) ; an été's short form
            // IS its semester, so the long form alone suffices
            if semester.season == ulaval_scheduler_core::Season::Summer {
                return long;
            }
            let seasons = ulaval_scheduler_core::horizon_sessions(
                plan.read().start.season,
                plan.read().study_sessions,
            );
            let semesters =
                state::session_semesters(plan.read().start, &seasons);
            format!(
                "{} — {long}",
                state::session_short(&semesters, session - 1),
            )
        })
        .unwrap_or_else(|| "Horaire".to_string());
    // forced sections change what the status may honestly claim, and get
    // their way back to automatic (rapport étudiante 2026-08-13)
    let forced = plan
        .read()
        .chosen
        .get(&session)
        .is_some_and(|chosen| !chosen.is_empty());
    let history = use_context::<Signal<History>>();
    let print_target = use_context::<super::PrintTarget>().0;
    let status = present::schedule_status(&schedule.read(), forced);
    // what is not drawn must be announced where the eyes are, not only
    // under the fold
    let off_grid = schedule.read().excluded.len();
    let Some(grid) = grid() else {
        return rsx! {};
    };
    let off_grid = off_grid + grid.unplaced.len();
    rsx! {
        section { class: "grid-panel", aria_label: "Horaire hebdomadaire",
            div { class: "grid-head",
                h2 { "{title}" }
                span {
                    class: "grid-status",
                    class: if grid.conflict { "grid-status--conflict" },
                    "{status}"
                }
                button {
                    class: "grid-share grid-export",
                    title: "Ouvre l'aperçu d'impression — choisissez \
                            « Enregistrer en PDF »",
                    onclick: move |_| {
                        print::start_print(
                            print_target,
                            print::PrintKind::Organigramme,
                        );
                    },
                    "Exporter l'organigramme"
                }
                button {
                    class: "grid-share grid-export",
                    title: "Ouvre l'aperçu d'impression — choisissez \
                            « Enregistrer en PDF »",
                    onclick: move |_| {
                        print::start_print(
                            print_target,
                            print::PrintKind::Horaire,
                        );
                    },
                    "Exporter l'horaire"
                }
                if off_grid > 0 {
                    span { class: "grid-status grid-status--conflict",
                        "⚠ {off_grid} cours hors grille — détail sous \
                         l'horaire"
                    }
                }
                if forced {
                    button {
                        class: "grid-share",
                        title: "Retirer les sections forcées de cette \
                                session — l'horaire rechoisit tout seul",
                        onclick: move |_| {
                            edit_plan(
                                plan,
                                history,
                                "Sections forcées libérées",
                                |plan| {
                                    plan.chosen.remove(&session);
                                },
                            );
                        },
                        "Libérer les sections forcées"
                    }
                }
            }
            if !schedule.read().report.courses.is_empty() {
                p { class: "grid-legend",
                    "Plein = plage retenue - pointillé = autre plage \
                     possible (cliquer pour la forcer) - hachuré = conflit"
                }
            }
            if grid.days.iter().all(|day| day.blocks.is_empty())
                && grid.unplaced.is_empty()
                && schedule.read().excluded.is_empty()
            {
                p { class: "grid-empty",
                    "Aucun cours avec horaire publié pour cette session. \
                     Ajoutez des cours par code dans le panneau de gauche."
                }
            } else {
                div {
                    class: "grid",
                    style: "grid-template-columns: 3rem repeat({grid.days.len()}, 1fr);",
                    div { class: "grid-axis",
                        for hour in grid.hours.iter() {
                            span { "{hour}" }
                        }
                    }
                    for day in grid.days.iter() {
                        div { class: "grid-day",
                            div {
                                class: "grid-day-head",
                                class: if day.conflict { "grid-day-head--conflict" },
                                if day.conflict {
                                    "{day.label} ⚠"
                                } else {
                                    "{day.label}"
                                }
                            }
                            div { class: "grid-day-col",
                                for block in day.blocks.iter().cloned() {
                                    GridBlock {
                                        block,
                                        session,
                                        selections: current_selections(
                                            &schedule.read(),
                                        ),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            GridFootnotes { schedule: schedule.read().clone(), unplaced: grid.unplaced.clone() }
        }
    }
}

// the selected NRCs of every drawn course — what a swap must hold still
fn current_selections(
    schedule: &solve::WeeklySchedule,
) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
    schedule
        .report
        .courses
        .iter()
        .map(|course| {
            (
                course.code.clone(),
                course
                    .selected
                    .iter()
                    .map(|section| section.nrc.clone())
                    .collect(),
            )
        })
        .collect()
}

#[component]
fn GridBlock(
    block: Block,
    session: usize,
    selections: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<String>,
    >,
) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let super::DraggedCourse(mut dragged) =
        use_context::<super::DraggedCourse>();
    let super::DropHover(mut hover) = use_context::<super::DropHover>();
    let drag_code = block.code.clone();
    let style = format!(
        "top:{:.3}%;height:{:.3}%;left:{:.3}%;width:{:.3}%;",
        block.top, block.height, block.left, block.width
    );
    let code = block.code.clone();
    let nrcs = block.nrcs.clone();
    let ghost = block.ghost;
    let label = if ghost {
        format!("Forcer la section {} de {}", nrcs.join("+"), block.code)
    } else {
        format!("Voir les autres plages de {}", block.code)
    };
    rsx! {
        button {
            class: "grid-block grid-block--c{block.color}",
            class: if block.ghost { "grid-block--ghost" },
            class: if block.clash { "grid-block--conflict" },
            style: "{style}",
            title: "{label}",
            onclick: move |_| {
                if ghost {
                    // clicking a ghost is the swap the report promised:
                    // this course moves, every other course keeps its
                    // current selection — so pin them all (immediate +
                    // undoable, ACT-2)
                    let code = code.clone();
                    let set: std::collections::BTreeSet<String> =
                        nrcs.iter().cloned().collect();
                    let mut pinned = selections.clone();
                    pinned.insert(code.clone(), set);
                    edit_plan(
                        plan,
                        history,
                        &format!("Section de {code} forcée"),
                        |plan| {
                            plan.chosen
                                .entry(session)
                                .or_default()
                                .extend(pinned);
                        },
                    );
                    selected.set(None);
                    return;
                }
                // the read borrow must die before the set below
                let already =
                    selected.read().as_deref() == Some(code.as_str());
                if already {
                    selected.set(None);
                } else {
                    selected.set(Some(code.clone()));
                }
            },
            // note 16: a solid block drags toward the ribbon — the cards
            // mark the sessions whose season offers it; the panel's
            // choice strip remains the keyboard path (INP-4)
            draggable: !ghost,
            ondragstart: move |event| {
                // Firefox refuses to carry a drag whose DataTransfer is
                // empty — the signal stays the payload the drop reads,
                // this token is the browser's fee (best-effort: the
                // signal still carries the code if the write fails)
                let transfer = event.data_transfer();
                let _ = transfer.set_data("text/plain", &drag_code);
                transfer.set_effect_allowed("move");
                dragged.set(Some(drag_code.clone()));
            },
            ondragend: move |_| {
                dragged.set(None);
                hover.set(None);
            },
            div { class: "grid-block-title", "{block.title}" }
            div { class: "grid-block-detail",
                "{block.detail}"
                if block.clash {
                    span { class: "grid-block-warn", " ⚠ conflit" }
                }
            }
        }
    }
}

// what could not be drawn, and why — right under the grid, never dropped
#[component]
fn GridFootnotes(
    schedule: solve::WeeklySchedule,
    unplaced: Vec<String>,
) -> Element {
    rsx! {
        div { class: "grid-notes",
            for excluded in schedule.excluded.iter() {
                p { class: "warning",
                    "⚠ {excluded.code} — {excluded.reason} : gardé dans la \
                     liste, rien n'est dessiné."
                }
            }
            for note in schedule.notes.iter() {
                p { class: "warning", "⚠ {note}" }
            }
            for code in unplaced.iter() {
                p { class: "grid-unplaced",
                    "{code} — sans plage hebdomadaire (à distance) : suivi \
                     hors grille."
                }
            }
        }
    }
}

fn long_semester(semester: ulaval_scheduler_core::Semester) -> String {
    let season = match semester.season {
        ulaval_scheduler_core::Season::Fall => "Automne",
        ulaval_scheduler_core::Season::Winter => "Hiver",
        ulaval_scheduler_core::Season::Summer => "Été",
    };
    format!("{season} {}", semester.year)
}
