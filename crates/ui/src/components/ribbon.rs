use dioxus::prelude::*;

use super::SelectedCourse;
use crate::data::Snapshot;
use crate::present::{self, RibbonCard};
use crate::state::{Plan, View};

// The A1→H8 ribbon: one card per study session, narrow strips for the
// étés. A whole card is one button (INP-1) that displays that session;
// states are carried by glyphs and wording, never colour alone (INP-3).
#[component]
pub fn SessionRibbon() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let today =
        crate::state::semester_of_epoch_ms(crate::browser::now_epoch_ms());
    let cards = present::ribbon_model(
        snapshot,
        &plan.read(),
        view.read().session,
        today,
    );
    rsx! {
        nav { class: "ribbon", aria_label: "Sessions du cheminement",
            for card in cards {
                if card.summer
                    && card.codes.is_empty()
                    && card.special.is_none()
                {
                    SummerStrip { card }
                } else {
                    // a busy été deserves a readable card, not a sliver
                    SessionCard { card }
                }
            }
        }
    }
}

// where the dragged course may land, according to the probe's cache:
// `Some(true)` = admissible target, `Some(false)` = barred, `None` = no
// drag or probe still running (neutral)
fn drop_state(
    dragged: &Option<String>,
    solver: &super::SolverState,
    index: usize,
) -> Option<bool> {
    let code = dragged.as_ref()?;
    let sessions = solver.admissible.get(code)?;
    Some(sessions.contains(&index))
}

#[component]
fn SessionCard(card: RibbonCard) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<crate::state::History>>();
    let mut view = use_context::<Signal<View>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let super::DraggedCourse(mut dragged) =
        use_context::<super::DraggedCourse>();
    let solver = use_context::<Signal<super::SolverState>>();
    let index = card.index;
    let target = drop_state(&dragged.read(), &solver.read(), index);
    let dragging = dragged.read().is_some();
    let drop_label = card.label.clone();
    let credits = if card.codes.is_empty() {
        "—".to_string()
    } else if card.passed {
        format!("{}✓", card.credits)
    } else if card.over_cap {
        format!("{} ⚠", card.credits)
    } else {
        format!("{}", card.credits)
    };
    let range_mark = if card.has_range { " (min.)" } else { "" };
    rsx! {
        button {
            class: "ribbon-card",
            class: if card.current { "ribbon-card--current" },
            class: if card.passed { "ribbon-card--passed" },
            class: if card.summer { "ribbon-card--ete" },
            class: if card.over_cap { "ribbon-card--over" },
            class: if target == Some(true) { "ribbon-card--target" },
            class: if target == Some(false) { "ribbon-card--barred" },
            aria_current: if card.current { "true" },
            onclick: move |_| {
                view.write().session = index;
                selected.set(None);
            },
            // note 16: a dragged block lands here — barred cards refuse
            ondragover: move |event| {
                if dragging && target != Some(false) {
                    event.prevent_default();
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                // the read borrow must die before place_course writes
                let code = dragged.read().clone();
                let Some(code) = code else {
                    return;
                };
                dragged.set(None);
                super::panel::place_course(
                    plan, history, &code, index, &drop_label,
                );
            },
            div { class: "ribbon-card-head",
                span { class: "ribbon-card-label", "{card.label}" }
                span { class: "ribbon-card-credits",
                    "{credits}{range_mark}"
                }
            }
            if card.conflict {
                div {
                    class: "ribbon-card-special header-credits--over",
                    title: "Conflit d'horaire dans cette session — plages \
                            hachurées dans sa grille",
                    "⚠ conflit d'horaire"
                }
            }
            if card.codes.is_empty() {
                div { class: "ribbon-card-empty", "à planifier" }
            } else {
                div { class: "ribbon-card-codes",
                    for code in card.codes.iter() {
                        span { "{code}" }
                    }
                }
            }
            if let Some(special) = card.special.as_ref() {
                div { class: "ribbon-card-special", "{special}" }
            }
        }
    }
}

#[component]
fn SummerStrip(card: RibbonCard) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<crate::state::History>>();
    let mut view = use_context::<Signal<View>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let super::DraggedCourse(mut dragged) =
        use_context::<super::DraggedCourse>();
    let solver = use_context::<Signal<super::SolverState>>();
    let index = card.index;
    let target = drop_state(&dragged.read(), &solver.read(), index);
    let dragging = dragged.read().is_some();
    let drop_label = card.label.clone();
    let content = card
        .special
        .clone()
        .or_else(|| (!card.codes.is_empty()).then(|| card.codes.join(" ")))
        .unwrap_or_else(|| "—".to_string());
    rsx! {
        button {
            class: "ribbon-summer",
            class: if card.current { "ribbon-summer--current" },
            class: if !card.codes.is_empty() || card.special.is_some() {
                "ribbon-summer--busy"
            },
            class: if target == Some(true) { "ribbon-card--target" },
            class: if target == Some(false) { "ribbon-card--barred" },
            aria_current: if card.current { "true" },
            onclick: move |_| {
                view.write().session = index;
                selected.set(None);
            },
            ondragover: move |event| {
                if dragging && target != Some(false) {
                    event.prevent_default();
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                // the read borrow must die before place_course writes
                let code = dragged.read().clone();
                let Some(code) = code else {
                    return;
                };
                dragged.set(None);
                super::panel::place_course(
                    plan, history, &code, index, &drop_label,
                );
            },
            span { "{card.label} - {content}" }
        }
    }
}
