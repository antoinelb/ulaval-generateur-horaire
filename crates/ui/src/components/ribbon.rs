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

// where the dragged course may land: `Some(true)` = its season is offered
// there (kept at full face), `Some(false)` = not offered (faded, refuses
// the drop), `None` = no drag in progress. The chips' season filter, never
// the solver probe — instant, and at parity with the keyboard path
// (retour d'Antoine, 2026-08-19).
fn drop_state(
    snapshot: &Option<Snapshot>,
    plan: &Plan,
    dragged: &Option<String>,
    index: usize,
) -> Option<bool> {
    let code = dragged.as_ref()?;
    let snapshot = snapshot.as_ref()?;
    Some(crate::panel::offered_sessions(snapshot, plan, code).contains(&index))
}

#[component]
fn SessionCard(card: RibbonCard) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<crate::state::History>>();
    let mut view = use_context::<Signal<View>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let super::DraggedCourse(mut dragged) =
        use_context::<super::DraggedCourse>();
    let super::DropHover(mut hover) = use_context::<super::DropHover>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let index = card.index;
    let dragged_code = dragged.read().clone();
    let target =
        drop_state(&snapshot.read(), &plan.read(), &dragged_code, index);
    let landing = dragged_code.is_some() && *hover.read() == Some(index);
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
            class: if landing { "ribbon-card--landing" },
            aria_current: if card.current { "true" },
            onclick: move |_| {
                view.write().session = index;
                selected.set(None);
            },
            // note 16: a dragged course lands here — non-offered cards
            // refuse. Everything is read live: the render-time capture may
            // predate this drag.
            ondragover: move |event| {
                let code = dragged.read().clone();
                let refused = drop_state(
                    &snapshot.read(), &plan.read(), &code, index,
                ) == Some(false);
                if code.is_some() && !refused {
                    event.prevent_default();
                    let stale = *hover.peek() != Some(index);
                    if stale {
                        hover.set(Some(index));
                    }
                }
            },
            // a leave toward a child is re-set by the same burst's
            // dragover, so clearing here never flickers
            ondragleave: move |_| {
                let here = *hover.peek() == Some(index);
                if here {
                    hover.set(None);
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                hover.set(None);
                // the read borrow must die before place_course writes
                let code = dragged.read().clone();
                let Some(code) = code else {
                    return;
                };
                dragged.set(None);
                super::panel::place_course(
                    plan, history, &code, index, &drop_label, None,
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
                        RibbonCode { key: "{code}", code: code.clone() }
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
    let super::DropHover(mut hover) = use_context::<super::DropHover>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let index = card.index;
    let dragged_code = dragged.read().clone();
    let target =
        drop_state(&snapshot.read(), &plan.read(), &dragged_code, index);
    let landing = dragged_code.is_some() && *hover.read() == Some(index);
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
            class: if landing { "ribbon-card--landing" },
            aria_current: if card.current { "true" },
            onclick: move |_| {
                view.write().session = index;
                selected.set(None);
            },
            ondragover: move |event| {
                let code = dragged.read().clone();
                let refused = drop_state(
                    &snapshot.read(), &plan.read(), &code, index,
                ) == Some(false);
                if code.is_some() && !refused {
                    event.prevent_default();
                    let stale = *hover.peek() != Some(index);
                    if stale {
                        hover.set(Some(index));
                    }
                }
            },
            ondragleave: move |_| {
                let here = *hover.peek() == Some(index);
                if here {
                    hover.set(None);
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                hover.set(None);
                // the read borrow must die before place_course writes
                let code = dragged.read().clone();
                let Some(code) = code else {
                    return;
                };
                dragged.set(None);
                super::panel::place_course(
                    plan, history, &code, index, &drop_label, None,
                );
            },
            span { "{card.label} - {content}" }
        }
    }
}

// A course code inside a card is itself a drag source: the other half of
// note 16 — the code rides `DraggedCourse`, same circuit as a grid block,
// and the panel's chips stay the keyboard path (INP-4).
#[component]
fn RibbonCode(code: String) -> Element {
    let super::DraggedCourse(mut dragged) =
        use_context::<super::DraggedCourse>();
    let super::DropHover(mut hover) = use_context::<super::DropHover>();
    let drag_code = code.clone();
    rsx! {
        span {
            draggable: true,
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
            "{code}"
        }
    }
}
