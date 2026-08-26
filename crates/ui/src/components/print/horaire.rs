// The weekly-schedule print sheet (plan item 7): a thin renderer over
// `crate::export::horaire::schedule_document` — `components/` is excluded
// from `make test`, so every decision (pagination, the unpublished
// fallback, ghost suppression) stays in that pure model; this file only
// maps its fields to markup. No clock, no business logic.

use dioxus::prelude::*;

use crate::data::Snapshot;
use crate::export::horaire::{schedule_document, CourseLine};
use crate::present::{self, GridModel};
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

    // French fallback: a plan with no program chosen yet can still export
    // its (empty) weekly grids — the document names that state honestly
    // rather than showing a blank heading (TRU-1).
    let program_title = crate::panel::chosen_program(snapshot, &plan_read)
        .map(|program| program.title.clone())
        .unwrap_or_else(|| "aucun programme choisi".to_string());
    let generated_at = crate::browser::now_local();
    let document =
        schedule_document(snapshot, &plan_read, &program_title, &generated_at);
    let last_page = document.pages.len().saturating_sub(1);
    // the same whole-organigramme link the header's share button copies
    // (note 9): a reader of the saved PDF clicks straight back into this
    // very schedule
    let share_link = {
        let manual_read = manual.read();
        crate::browser::share_url(&crate::persist::encode_organigramme(
            &plan_read,
            &manual_read,
        ))
    };

    rsx! {
        div { class: "print-horaire-sheet",
            // The document head lives inside the first page div and the
            // provenance inside the last: `@page` prints with no margin (so
            // the browser adds no header/footer of its own), and each page
            // div carries the paper margin as padding instead — anything
            // outside a page div would sit at the paper's very edge.
            for (index, page) in document.pages.iter().enumerate() {
                div {
                    key: "{index}",
                    class: "print-horaire-page",
                    class: if index != last_page { "print-break" },
                    if index == 0 {
                        header { class: "print-horaire-head",
                            h1 { "{document.title}" }
                            p { class: "print-horaire-program", "{document.program_title}" }
                        }
                    }
                    for sheet in page.sheets.iter() {
                        SessionHalf {
                            title: sheet.title.clone(),
                            grid: sheet.grid.clone(),
                            courses: course_lines(&sheet.courses),
                            notes: sheet.notes.clone(),
                        }
                    }
                    // Provenance renders once, on the last page — EXP-1
                    // only requires the document to carry it, and the line
                    // already spells out code, données and the repo URL, so
                    // nothing is repeated after it.
                    if index == last_page {
                        footer { class: "print-horaire-provenance",
                            p { class: "print-horaire-share",
                                "Vous pouvez accéder à cet horaire en cliquant "
                                a { href: "{share_link}", "ici." }
                            }
                            p { "{document.provenance.line}" }
                        }
                    }
                }
            }
        }
    }
}

// `CourseLine` carries no `Clone`/`PartialEq` (it belongs to the pure
// model, task 6) — component props must be both (AP-9), so this reshapes
// the borrowed slice into owned tuples. Pure data-shape plumbing, no
// decision: one line in, one tuple out.
fn course_lines(courses: &[CourseLine]) -> Vec<(String, String, String)> {
    courses
        .iter()
        .map(|course| {
            (
                course.code.clone(),
                course.title.clone(),
                course.detail.clone(),
            )
        })
        .collect()
}

// One half-page: a session's title, its weekly grid when the model drew
// one, then the course list (only what the grid cannot show — the model
// decides) and any notes, so nothing the model carried is lost to the
// reader.
#[component]
fn SessionHalf(
    title: String,
    grid: Option<GridModel>,
    courses: Vec<(String, String, String)>,
    notes: Vec<String>,
) -> Element {
    rsx! {
        section { class: "print-horaire-half",
            h2 { "{title}" }
            if let Some(grid) = grid {
                PrintGrid { grid }
            }
            if !courses.is_empty() {
                ul { class: "print-horaire-courses",
                    for (code, course_title, detail) in courses.iter() {
                        li { key: "{code}",
                            "{code} — {course_title} ({detail})"
                        }
                    }
                }
            }
            for note in notes.iter() {
                p { class: "warning", "⚠ {note}" }
            }
        }
    }
}

// The same absolute-positioned-block technique `components/grid.rs`'s
// `WeeklyGrid` uses on screen, restated as plain `div`s: nothing printed is
// interactive, and the model already drew no ghost block (`ghosts_for:
// None` in `export::horaire::session_sheet`), so there is none to skip
// here.
#[component]
fn PrintGrid(grid: GridModel) -> Element {
    // The hour lines are real positioned elements, not a background
    // gradient like the screen's: browsers rasterize backgrounds
    // unreliably at print scale, while a border always prints. One line
    // per axis tick, top and bottom included.
    let intervals = grid.hours.len().saturating_sub(1).max(1);
    let hour_lines: Vec<String> = (0..=intervals)
        .map(|line| {
            format!("top:{:.3}%;", line as f32 * 100.0 / intervals as f32)
        })
        .collect();
    // ULaval slots end at :20/:50 and the next course starts :30/:00 — a
    // fixed 10-minute break. Extending every block through it leaves the
    // same hairline between stacked courses as between days (the shave in
    // PrintBlock), instead of a proportional ~3px band of white.
    let span = grid.end.saturating_sub(grid.start).max(1);
    let extend_pct = 10.0 * 100.0 / f32::from(span);
    rsx! {
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
                        // INP-3: the conflict is a glyph, never colour alone
                        if day.conflict {
                            "{day.label} ⚠"
                        } else {
                            "{day.label}"
                        }
                    }
                    div { class: "grid-day-col",
                        for (line, position) in hour_lines.iter().enumerate() {
                            div {
                                key: "{line}",
                                class: "grid-hour-line",
                                style: "{position}",
                            }
                        }
                        for block in day.blocks.iter() {
                            PrintBlock { block: block.clone(), extend_pct }
                        }
                    }
                }
            }
        }
    }
}

// One line per block — « MAT-1900 — Mathématiques… », code first, no
// section/mode detail: on a printed week the code is what a student cross
// checks, and the second line only ate the little height a block has. One
// single text node: mixing an element and a text sibling makes rsx emit
// whitespace between them, which printed as a gap around the dash.
#[component]
fn PrintBlock(block: present::Block, extend_pct: f32) -> Element {
    // `extend_pct` stretches the block through the standard 10-minute
    // break (see PrintGrid), then a hair is shaved off its bottom and
    // right — stacked courses and side-by-side conflict lanes end up
    // separated by the same 0.0625rem the day gap uses
    let style = format!(
        "top:{:.3}%;height:calc({:.3}% - 0.0625rem);\
         left:{:.3}%;width:calc({:.3}% - 0.0625rem);--course-h:{:.1};",
        block.top,
        block.height + extend_pct,
        block.left,
        block.width,
        block.hue
    );
    rsx! {
        div {
            class: "grid-block",
            class: if block.clash { "grid-block--conflict" },
            style: "{style}",
            div { class: "grid-block-title",
                "{block.code} — {block.title}"
                // INP-3: the conflict stays textual, never colour alone
                if block.clash {
                    span { class: "grid-block-warn", " ⚠ conflit" }
                }
            }
        }
    }
}
