// The organigramme print sheet (plan item 7): a thin renderer over
// `crate::export::organigramme::organigramme_document` — every decision
// (columns, token letters, option-box placement, spans, rules table) lives
// in that pure model (task 3/4); this file only maps its fields to markup.
// No clock, no business logic (AP-5).

use dioxus::prelude::*;

use crate::data::Snapshot;
use crate::export::organigramme::{
    organigramme_document, Column, CourseBox, LanguageBox, OptionBox,
    OptionRule, RulesRow, SpanBox, Token,
};
use crate::state::Plan;

#[component]
pub fn Sheet() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();

    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let plan_read = plan.read();
    let program = crate::panel::effective_program(snapshot, &plan_read);
    let generated_at = crate::browser::now_local();
    let document = organigramme_document(
        snapshot,
        &plan_read,
        program.as_ref(),
        &generated_at,
    );
    let column_count = document.columns.len();
    let grid_columns =
        format!("grid-template-columns: repeat({column_count}, 1fr);");

    rsx! {
        div { class: "print-organigramme-sheet",
            header { class: "print-organigramme-head",
                h1 { "{document.title}" }
                p { class: "print-organigramme-program", "{document.program_title}" }
                p { class: "print-organigramme-version", "{document.version}" }
            }
            div {
                class: "print-organigramme-grid",
                style: "{grid_columns}",
                for column in document.columns.iter() {
                    ColumnView { key: "{column.label}", column: column.clone() }
                }
            }
            if !document.spans.is_empty() {
                div {
                    class: "print-organigramme-spans",
                    style: "{grid_columns}",
                    for span in document.spans.iter() {
                        SpanView { key: "{span.label}", span: span.clone() }
                    }
                }
            }
            if let Some(language) = &document.language {
                LanguageView { language: language.clone() }
            }
            for band in document.bands.iter() {
                p { key: "{band}", class: "print-organigramme-band", "{band}" }
            }
            footer { class: "print-organigramme-notes",
                for note in document.notes.iter() {
                    p { key: "{note}", "{note}" }
                }
                for line in document.legend.iter() {
                    p { key: "{line}", "{line}" }
                }
            }
            if !document.rules_table.is_empty() {
                RulesTable { rows: document.rules_table.clone() }
            }
            // The provenance line already spells out code, données and the
            // repo URL (EXP-1) — repeating them as a second line of links
            // after it would only print the same facts twice.
            footer { class: "print-organigramme-provenance",
                p { "{document.provenance.line}" }
            }
        }
    }
}

// One session column: its short head-bar label (`semester`, e.g. « A1 »),
// the full session identity as a `title` tooltip, its mandatory boxes, then
// its « Cours option » box(es) below them.
#[component]
fn ColumnView(column: Column) -> Element {
    rsx! {
        div { class: "print-organigramme-column",
            div {
                class: "print-organigramme-column-head",
                title: "{column.label}",
                "{column.semester}"
            }
            div { class: "print-organigramme-column-boxes",
                for course_box in column.boxes.iter() {
                    CourseBoxView {
                        key: "{course_box.code}",
                        course_box: course_box.clone(),
                    }
                }
            }
            for option_box in column.options.iter() {
                OptionBoxView {
                    key: "{option_box.heading}",
                    option_box: option_box.clone(),
                }
            }
        }
    }
}

// A course box: code (bold) and title in the body, entry tokens flowing in
// the left gutter, the (single, optional) exit token in the right gutter —
// the gutters flow with however many tokens the model gave them, never a
// hand-tuned offset (they are plain flex columns, in `print-organigramme.css`).
#[component]
fn CourseBoxView(course_box: CourseBox) -> Element {
    rsx! {
        div { class: "print-organigramme-box",
            div { class: "print-organigramme-box-gutter print-organigramme-box-gutter--entry",
                for (index, token) in course_box.entry.iter().enumerate() {
                    TokenView { key: "{index}", token: token.clone() }
                }
            }
            div { class: "print-organigramme-box-body",
                div { class: "print-organigramme-box-code", "{course_box.code}" }
                div { class: "print-organigramme-box-title", "{course_box.title}" }
            }
            div { class: "print-organigramme-box-gutter print-organigramme-box-gutter--exit",
                if let Some(token) = &course_box.exit {
                    TokenView { token: token.clone() }
                }
            }
        }
    }
}

// A letter token renders inside a small oval, shaded when the source is
// concomitant (INP-3: the shading is never the only carrier — the legend
// explains it and the letter itself stays readable either way). A credits
// token renders its number inside a small square.
#[component]
fn TokenView(token: Token) -> Element {
    match token {
        Token::Letter { letter, shaded } => rsx! {
            span {
                class: "print-organigramme-token print-organigramme-token--letter",
                class: if shaded { "print-organigramme-token--shaded" },
                "{letter}"
            }
        },
        Token::Credits { credits } => rsx! {
            span {
                class: "print-organigramme-token print-organigramme-token--credits",
                "{credits}"
            }
        },
    }
}

// The grey « Cours option » box: its heading (the model embeds the two
// lines — « Cours option » and « Choix possibles en … » — separated by a
// newline; split here since rsx text nodes do not wrap on `\n`), then one
// block per unsatisfied rule.
#[component]
fn OptionBoxView(option_box: OptionBox) -> Element {
    rsx! {
        div { class: "print-organigramme-option-box",
            div { class: "print-organigramme-option-heading",
                for line in option_box.heading.lines() {
                    p { key: "{line}", "{line}" }
                }
            }
            for rule in option_box.rules.iter() {
                OptionRuleView { key: "{rule.title}", rule: rule.clone() }
            }
        }
    }
}

// `constraint` already carries the rule's own title (`export::organigramme
// ::constraint_text` prefixes it, e.g. « Règle 1 – 1 à 3 cours »), so it is
// the single bold line the official document itself shows — rendering
// `title` again beside it would only repeat the same words a second time.
#[component]
fn OptionRuleView(rule: OptionRule) -> Element {
    rsx! {
        div { class: "print-organigramme-option-rule",
            p { class: "print-organigramme-option-rule-constraint", "{rule.constraint}" }
            if !rule.choices.is_empty() {
                ul { class: "print-organigramme-option-rule-choices",
                    for choice in rule.choices.iter() {
                        li { key: "{choice}", "{choice}" }
                    }
                }
            } else if let Some(raw) = &rule.raw {
                p { class: "print-organigramme-option-rule-raw", "{raw}" }
            }
        }
    }
}

// A microprogramme-de-stage band, spanning from the session preceding its
// placement to its own session. `grid-column` is the one place a model
// index becomes a style (the task's own exception) — the CSS grid line
// numbering is 1-indexed and exclusive on its end, so a 0-indexed
// `[first_column, last_column]` inclusive range becomes `first+1 / last+2`.
#[component]
fn SpanView(span: SpanBox) -> Element {
    let style = format!(
        "grid-column: {} / {};",
        span.first_column + 1,
        span.last_column + 2
    );
    rsx! {
        div { class: "print-organigramme-span", style: "{style}", "{span.label}" }
    }
}

#[component]
fn LanguageView(language: LanguageBox) -> Element {
    rsx! {
        div { class: "print-organigramme-language",
            p { class: "print-organigramme-language-label", "{language.label}" }
            p { class: "print-organigramme-language-detail", "{language.detail}" }
        }
    }
}

#[component]
fn RulesTable(rows: Vec<RulesRow>) -> Element {
    rsx! {
        table { class: "print-organigramme-rules",
            thead {
                tr {
                    th { "Règle" }
                    th { "Contrainte" }
                    th { "Choisis" }
                }
            }
            tbody {
                for row in rows.iter() {
                    tr { key: "{row.rule}",
                        td { "{row.rule}" }
                        td { "{row.constraint}" }
                        td { "{row.chosen}" }
                    }
                }
            }
        }
    }
}
