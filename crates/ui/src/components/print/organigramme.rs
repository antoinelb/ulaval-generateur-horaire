// The organigramme print sheet: a thin renderer over
// `crate::export::organigramme::organigramme_document` — every decision
// (mode, labels, token letters, chips, stats, option-box placement, stage
// slots, rules table, meta lines) lives in that pure model; this file only maps
// its fields to markup. No clock, no business logic (AP-5): the boundary
// reads (instant, zone, share link) happen here and are handed in, exactly
// like `print/horaire.rs`.

use dioxus::prelude::*;

use crate::data::Snapshot;
use crate::export::organigramme::{
    organigramme_document, Column, CourseBox, LegendEntry, OptionBox,
    OptionRule, RulesRow, Stat, SummerGroup, Token,
};
use crate::state::Plan;

#[component]
pub fn Sheet() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let crate::components::ManualCourses(manual) =
        use_context::<crate::components::ManualCourses>();

    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let plan_read = plan.read();
    let program = crate::panel::effective_program(snapshot, &plan_read);

    let generated_at = crate::browser::now_local();
    // the same whole-organigramme link the header's share button copies: a
    // reader of the saved PDF clicks straight back into this very plan
    let share_url = {
        let manual_read = manual.read();
        crate::browser::share_url(&crate::persist::encode_organigramme(
            &plan_read,
            &manual_read,
        ))
    };
    let document = organigramme_document(
        snapshot,
        &plan_read,
        program.as_ref(),
        &generated_at,
        &share_url,
    );
    let column_count = document.columns.len().max(1);
    let grid_columns =
        format!("grid-template-columns: repeat({column_count}, 1fr);");

    rsx! {
        div { class: "print-organigramme-sheet",
            header { class: "print-organigramme-head",
                div {
                    p { class: "print-organigramme-kicker", "{document.kicker}" }
                    h1 { "{document.program_title}" }
                    p { class: "print-organigramme-subtitle", "{document.subtitle}" }
                }
                div { class: "print-organigramme-meta",
                    p { "{document.meta.generated}" }
                    p { "{document.meta.data}" }
                    p { "{document.meta.build}" }
                    p {
                        a { href: "{document.meta.repo_url}", "{document.meta.repo_label}" }
                    }
                    p {
                        a { href: "{document.meta.share_url}", "{document.meta.share_label}" }
                    }
                }
            }
            if !document.stats.is_empty() {
                StatsStrip { stats: document.stats.clone() }
            }
            // one grid, two rows: the columns span both so their separator
            // lines run down to the foot, and the summer groups sit in the
            // second row, straddling — in front of — those lines
            div {
                class: "print-organigramme-grid",
                style: "{grid_columns}",
                for (index, column) in document.columns.iter().enumerate() {
                    ColumnView {
                        key: "{column.label}",
                        column: column.clone(),
                        index,
                    }
                }
                for summer in document.summers.iter() {
                    SummerView {
                        key: "{summer.first_column}",
                        summer: summer.clone(),
                    }
                }
            }
            div { class: "print-organigramme-foot",
                section {
                    h2 { "Règles du programme" }
                    if !document.rules_table.is_empty() {
                        RulesTable { rows: document.rules_table.clone() }
                    }
                }
                section {
                    h2 { "Exigences" }
                    for (index, sidebox) in document.requirements.iter().enumerate() {
                        div { key: "{index}", class: "print-organigramme-sidebox",
                            for (line_index, line) in sidebox.lines.iter().enumerate() {
                                p { key: "{line_index}",
                                    if let Some(lead) = &line.lead {
                                        span { class: "print-organigramme-sidebox-lead",
                                            "{lead} "
                                        }
                                    }
                                    "{line.text}"
                                }
                            }
                        }
                    }
                }
                section {
                    h2 { "Légende" }
                    div { class: "print-organigramme-legend",
                        for (index, entry) in document.legend.iter().enumerate() {
                            LegendLine { key: "{index}", entry: entry.clone() }
                        }
                    }
                    p { class: "print-organigramme-disclaimer", "{document.disclaimer}" }
                }
            }
            if !document.notes.is_empty() {
                footer { class: "print-organigramme-notes",
                    for note in document.notes.iter() {
                        p { key: "{note}", "{note}" }
                    }
                }
            }
        }
    }
}

#[component]
fn StatsStrip(stats: Vec<Stat>) -> Element {
    rsx! {
        div { class: "print-organigramme-stats",
            for stat in stats.iter() {
                div { key: "{stat.label}", class: "print-organigramme-stat",
                    b { "{stat.value}" }
                    span { "{stat.label}" }
                }
            }
        }
    }
}

// One session column: its head bar (label left, placed credits right when
// the model gave them), its course boxes, then its « Cours option » box(es).
// The explicit `grid-column` matters: the summer groups carry explicit
// placements, and auto-placed columns would dodge the row-2 cells they
// occupy instead of lining up one per track.
#[component]
fn ColumnView(column: Column, index: usize) -> Element {
    let style = format!("grid-column: {};", index + 1);
    rsx! {
        div { class: "print-organigramme-column", style: "{style}",
            ColumnHead { label: column.label.clone(), credits: column.credits.clone() }
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

#[component]
fn ColumnHead(label: String, credits: Option<String>) -> Element {
    rsx! {
        div { class: "print-organigramme-column-head",
            span { class: "print-organigramme-column-label", "{label}" }
            if let Some(credits) = credits {
                span { class: "print-organigramme-column-credits", "{credits}" }
            }
        }
    }
}

// A summer group (a real été, or the cheminement type's stage slots),
// straddling the columns around it: the CSS grid line numbering is
// 1-indexed and exclusive on its end, so the model's 0-indexed inclusive
// `[first_column, last_column]` becomes `first+1 / last+2` — the one place
// a model index becomes a style.
#[component]
fn SummerView(summer: SummerGroup) -> Element {
    let style = format!(
        "grid-row: 2; grid-column: {} / {};",
        summer.first_column + 1,
        summer.last_column + 2
    );
    rsx! {
        div { class: "print-organigramme-summer", style: "{style}",
            if let Some(label) = &summer.label {
                ColumnHead { label: label.clone(), credits: summer.credits.clone() }
            }
            div { class: "print-organigramme-column-boxes",
                for course_box in summer.boxes.iter() {
                    CourseBoxView {
                        key: "{course_box.code}",
                        course_box: course_box.clone(),
                    }
                }
            }
        }
    }
}

// A course box: code (bold, in the course's own hue), chip and credits on
// the first line, the title under them; entry tokens flowing in the left
// gutter, the (single, optional) exit token in the right gutter — the
// gutters flow with however many tokens the model gave them, never a
// hand-tuned offset (they are plain flex columns, in
// `print-organigramme.css`).
#[component]
fn CourseBoxView(course_box: CourseBox) -> Element {
    let style = format!("--course-h:{:.1};", course_box.hue);
    rsx! {
        div {
            class: "print-organigramme-box",
            class: if course_box.optional { "print-organigramme-box--optional" },
            style: "{style}",
            div { class: "print-organigramme-box-gutter print-organigramme-box-gutter--entry",
                for (index, token) in course_box.entry.iter().enumerate() {
                    TokenView { key: "{index}", token: token.clone() }
                }
            }
            div { class: "print-organigramme-box-body",
                div { class: "print-organigramme-box-line",
                    span { class: "print-organigramme-box-code", "{course_box.code}" }
                    if let Some(tag) = &course_box.tag {
                        span { class: "print-organigramme-chip", "{tag}" }
                    }
                    if !course_box.credits.is_empty() {
                        span { class: "print-organigramme-box-credits", "{course_box.credits}" }
                    }
                }
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

// A letter token renders inside a small square, shaded pale grey when the
// source is concomitant (INP-3: the shading is never the only carrier —
// the legend explains it and the letter itself stays readable either way).
// A credits token renders its number inside a slightly taller square.
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

// The grey « Cours option » box: its heading, then one block per
// unsatisfied rule — title and constraint on the bold line, the choices or
// the raw text under them.
#[component]
fn OptionBoxView(option_box: OptionBox) -> Element {
    rsx! {
        div { class: "print-organigramme-option-box",
            p { class: "print-organigramme-option-heading", "{option_box.heading}" }
            for rule in option_box.rules.iter() {
                OptionRuleView { key: "{rule.title}", rule: rule.clone() }
            }
        }
    }
}

#[component]
fn OptionRuleView(rule: OptionRule) -> Element {
    rsx! {
        div { class: "print-organigramme-option-rule",
            p { class: "print-organigramme-option-rule-constraint",
                "{rule.title} — {rule.constraint}"
            }
            if !rule.choices.is_empty() {
                p { class: "print-organigramme-option-rule-choices",
                    "{rule.choices.join(\", \")}"
                }
            } else if let Some(raw) = &rule.raw {
                p { class: "print-organigramme-option-rule-raw", "{raw}" }
            }
        }
    }
}

// The legend: one line per typed entry, the matching swatch drawn beside
// it — the model says what each swatch means, this only picks the markup.
#[component]
fn LegendLine(entry: LegendEntry) -> Element {
    match entry {
        LegendEntry::Letter { text } => rsx! {
            p {
                span { class: "print-organigramme-token print-organigramme-token--letter", "a" }
                "{text}"
            }
        },
        LegendEntry::Shaded { text } => rsx! {
            p {
                span {
                    class: "print-organigramme-token print-organigramme-token--letter print-organigramme-token--shaded",
                    "a"
                }
                "{text}"
            }
        },
        LegendEntry::Credits { text } => rsx! {
            p {
                span { class: "print-organigramme-token print-organigramme-token--credits", "60" }
                "{text}"
            }
        },
        LegendEntry::Chip { chip, text } => rsx! {
            p {
                span { class: "print-organigramme-chip", "{chip}" }
                "{text}"
            }
        },
        LegendEntry::Optional { text } => rsx! {
            p {
                span { class: "print-organigramme-swatch print-organigramme-swatch--dashed" }
                "{text}"
            }
        },
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
                    th { "Cours retenus" }
                    th { class: "print-organigramme-rules-number", "Cr" }
                }
            }
            tbody {
                for row in rows.iter() {
                    tr { key: "{row.rule}",
                        class: if !row.resolved { "print-organigramme-rules-row--unresolved" },
                        td { "{row.rule}" }
                        td { "{row.constraint}" }
                        td { "{row.chosen}" }
                        td { class: "print-organigramme-rules-number", "{row.credits}" }
                    }
                }
            }
        }
    }
}
