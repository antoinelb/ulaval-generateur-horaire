use dioxus::prelude::*;

use super::{edit_plan, SelectedCourse};
use crate::data::Snapshot;
use crate::panel::{
    self, Badge, Fit, PanelGroup, PanelModel, Row, RowState, Section,
};
use crate::solve;
use crate::state::{self, History, Plan, ProgramChoice, View};
use ulaval_scheduler_core::MAX_STUDY_SESSIONS;

// The single left panel (notes 2026-08-13 : plus d'onglets) : the
// program's rules and organigramme controls, the catalogue search, the
// add-by-code field and the manual-course form. The session's own courses
// live in the schedule and the ribbon — no list here. While no program is
// chosen the picker *replaces* all of it: nothing to add courses with is
// worth showing before the only expected click. Everything shown comes
// from `crate::panel` (pure, tested); this file only wires clicks and
// signals.
#[component]
pub fn LeftPanel() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let model = use_memo(move || {
        let read = snapshot.read();
        let snapshot = read.as_ref()?;
        Some(panel::panel_model(snapshot, &plan.read()))
    });
    // one probe per (plan, session) — every row then costs a mask overlap
    let fit = use_memo(move || {
        let read = snapshot.read();
        let snapshot = read.as_ref()?;
        panel::fit_probe(snapshot, &plan.read(), view.read().session)
    });
    use_context_provider(|| fit);
    let Some(model) = model() else {
        return rsx! {};
    };
    rsx! {
        aside { class: "panel", aria_label: "Choix des cours",
            if plan.read().program.is_none() {
                ProgramPicker {}
            } else {
                PanelBody { model }
                ManualCourseForm {}
            }
        }
    }
}

// Choosing a program swaps documents: the shelf snapshot of this
// (code, millésime) comes back exactly if one exists, a fresh document
// starts otherwise — the picker document itself has nothing to shelve
// (ADR `2026-08-instantane-de-plan-par-programme-et-millesime`).
#[component]
fn ProgramPicker() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let solver = use_context::<Signal<super::SolverState>>();
    let handle = use_context::<super::SolverHandle>();
    let super::ManualCourses(manual) = use_context::<super::ManualCourses>();
    let super::LocalPrograms(local_programs) =
        use_context::<super::LocalPrograms>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    // the Signal itself, kept from the shadowing below: the choose click
    // reads the snapshot again to pick the default concentration
    let snapshot_signal = snapshot;
    // the vintage a select is on but no click has confirmed, by code — a
    // setting with no effect yet is nothing to undo, so it stays out of the
    // plan and its history. Absent = the select still shows its first
    // option, the newest vintage.
    let mut pending =
        use_signal(std::collections::HashMap::<String, String>::new);
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    rsx! {
        div { class: "panel-picker",
            p { class: "panel-picker-lead",
                "Choisissez un programme pour voir ses règles :"
            }
            for row in panel::program_vintages(snapshot) {
                div { class: "panel-picker-item", key: "{row.code}",
                    div { class: "panel-picker-title", "{row.title}" }
                    if let Some(mark) = &row.local {
                        span { class: "panel-picker-badge", "{mark.badge}" }
                        div { class: "panel-picker-provenance",
                            "{mark.provenance} "
                            if let Some(link) = &mark.link {
                                a {
                                    href: "{link}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    "{link}"
                                }
                            }
                        }
                        if !mark.anomalies.is_empty() {
                            details { class: "panel-picker-anomalies",
                                summary {
                                    "{mark.anomalies.len()} anomalie(s) de lecture"
                                }
                                ul {
                                    for anomaly in mark.anomalies.iter() {
                                        li { "{anomaly}" }
                                    }
                                }
                            }
                        }
                        // no confirmation dialog (pattern ACT, plan item
                        // 9) — the toast pushed by `remove_local_program`
                        // is what makes this undoable
                        button {
                            class: "panel-picker-remove",
                            r#type: "button",
                            aria_label: "Supprimer le programme importé {row.code}",
                            onclick: {
                                let code = row.code.clone();
                                let semester = mark.semester.clone();
                                let handle = handle.clone();
                                move |_| {
                                    super::remove_local_program(
                                        snapshot_signal,
                                        local_programs,
                                        plan,
                                        view,
                                        history,
                                        alerts,
                                        solver,
                                        &handle,
                                        manual,
                                        &code,
                                        &semester,
                                    );
                                }
                            },
                            "Supprimer"
                        }
                    }
                    div { class: "panel-picker-row",
                        span { class: "panel-picker-sub",
                            "{row.code} - {row.credits_required} cr"
                        }
                        select {
                            class: "panel-picker-vintage",
                            aria_label: "Version de {row.code}",
                            onchange: {
                                let code = row.code.clone();
                                move |event: Event<FormData>| {
                                    pending
                                        .write()
                                        .insert(code.clone(), event.value());
                                }
                            },
                            for vintage in row.vintages.iter() {
                                option { value: "{vintage}", "{vintage}" }
                            }
                        }
                        button {
                            class: "panel-picker-choose",
                            // seven buttons all reading « Choisir » say
                            // nothing to a screen reader or a tab-through
                            aria_label: "Choisir {row.code}",
                            onclick: {
                                let code = row.code.clone();
                                let newest = row.vintages.first().cloned();
                                let handle = handle.clone();
                                move |_| {
                                    let code = code.clone();
                                    // nothing touched the select yet: the
                                    // browser shows the first option, so
                                    // that is what the click means
                                    let Some(semester) = pending
                                        .read()
                                        .get(&code)
                                        .cloned()
                                        .or_else(|| newest.clone())
                                    else {
                                        return;
                                    };
                                    // défaut expert-sûr (AIR LAY-3, parité
                                    // avec la version JS) : la première
                                    // concentration du millésime choisi,
                                    // jamais de profil imposé — l'instantané
                                    // de l'étagère, s'il existe, garde son
                                    // propre choix
                                    let (concentration, study_sessions) = {
                                        let read = snapshot_signal.read();
                                        let snapshot = read.as_ref();
                                        let concentration =
                                            snapshot.and_then(|snapshot| {
                                                panel::default_concentration(
                                                    snapshot, &code, &semester,
                                                )
                                            });
                                        let study_sessions = snapshot
                                            .and_then(|snapshot| {
                                                panel::program_credits_required(
                                                    snapshot, &code, &semester,
                                                )
                                            })
                                            .map(state::default_study_sessions)
                                            .unwrap_or(
                                                state::DEFAULT_STUDY_SESSIONS,
                                            );
                                        (concentration, study_sessions)
                                    };
                                    let choice = ProgramChoice {
                                        code,
                                        semester,
                                        concentration,
                                        profile: None,
                                    };
                                    let stored = crate::browser::local_get(
                                        &crate::persist::snapshot_key(&choice),
                                    );
                                    let swap = crate::persist::enter_document(
                                        &plan.peek(),
                                        choice,
                                        stored.as_deref(),
                                        study_sessions,
                                        state::semester_of_epoch_ms(
                                            crate::browser::now_epoch_ms(),
                                        ),
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
                            "Choisir"
                        }
                    }
                    // a local vintage the `select` above is not currently
                    // showing (plan item 8's card only ever talks about the
                    // preselection) still needs a way out, named by its own
                    // semester so it never claims to be about the row's
                    // preselected vintage
                    for mark in row.other_local.iter() {
                        div {
                            class: "panel-picker-other-local",
                            key: "{mark.semester}",
                            span {
                                "{mark.badge} ({mark.semester}) — "
                                if let Some(link) = &mark.link {
                                    a {
                                        href: "{link}",
                                        target: "_blank",
                                        rel: "noopener noreferrer",
                                        "{link}"
                                    }
                                }
                            }
                            button {
                                class: "panel-picker-remove",
                                r#type: "button",
                                aria_label: "Supprimer le programme importé {row.code} ({mark.semester})",
                                onclick: {
                                    let code = row.code.clone();
                                    let semester = mark.semester.clone();
                                    let handle = handle.clone();
                                    move |_| {
                                        super::remove_local_program(
                                            snapshot_signal,
                                            local_programs,
                                            plan,
                                            view,
                                            history,
                                            alerts,
                                            solver,
                                            &handle,
                                            manual,
                                            &code,
                                            &semester,
                                        );
                                    }
                                },
                                "Supprimer"
                            }
                        }
                    }
                }
            }
            ImportDrawer {}
        }
    }
}

// The « Votre programme n'est pas là ? » drawer (plan item 7): collapsed
// at rest, forced open and locked while an import runs, never closed on
// error (INP-7/ERR-6). Every label, phase and error comes from
// `crate::import`/`crate::present` (already tested natively) — this
// component only wires signals and clicks (AP-5); it carries none of its
// own coverage since `components/` is wasm32-only.
#[component]
fn ImportDrawer() -> Element {
    let mut snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let super::LocalPrograms(mut local_programs) =
        use_context::<super::LocalPrograms>();

    let mut open = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut phase = use_signal(|| None::<crate::import::ImportPhase>);
    let mut elapsed = use_signal(|| 0u32);
    let mut error = use_signal(|| None::<crate::present::UiError>);
    let mut fetch =
        use_signal(|| None::<std::rc::Rc<crate::browser::ImportFetch>>);
    // bumped at every commit, checked once the fetch settles: a result
    // that outlives its own click — an Annuler, then a fresh Importer
    // before the aborted fetch resolved — belongs to a cycle nobody is
    // showing any more (same discipline as `SolverState.next_id`)
    let mut generation = use_signal(|| 0u64);

    // the file-picker sibling of the URL import above (plan item 6): the
    // chosen `{code}-{semestre}.json`, read by `browser::read_file_text`
    // only once « Charger » is clicked — picking a file is not itself an
    // act worth undoing
    let mut json_program_file = use_signal(|| None::<dioxus::html::FileData>);
    let mut json_loading = use_signal(|| false);
    // LAT-4/5: `use_future` spawns its closure once at mount and never
    // restarts it when a signal read inside changes (unlike `use_resource`)
    // — gating start/stop on `phase` would freeze after the first import.
    // Instead this single ticker runs for the drawer's whole lifetime
    // (mirrors `SolverStatus`, components/header.rs:18-25) and rechecks
    // `phase` on every tick, incrementing `elapsed` only while a phase is
    // running; `commit` resets it to 0 at the start of each import.
    use_future(move || async move {
        for _ in 0..86_400u32 {
            crate::browser::sleep_ms(1_000).await;
            if phase().is_some() {
                *elapsed.write() += 1;
            }
        }
    });

    let importing = phase().is_some();
    // one drawer, one import at a time: a JSON load in flight disables the
    // URL form and vice versa, so the two paths never race writing the same
    // `snapshot`/`local_programs`
    let busy = importing || json_loading();
    let expanded = open() || busy;
    // the state → glyph mapping is a closed, static table, decided once
    // here rather than inside `rsx!` (AP-5) — same shape as `SectionView`'s
    // badge match just above
    let phase_rows: Vec<(String, &'static str, bool)> =
        crate::import::phase_rows(phase())
            .into_iter()
            .map(|row| {
                let glyph = match row.state {
                    crate::import::PhaseState::Done => "✓",
                    crate::import::PhaseState::Running => "…",
                    crate::import::PhaseState::Pending => "·",
                };
                let running = row.state == crate::import::PhaseState::Running;
                (row.label, glyph, running)
            })
            .collect();

    let mut commit = move || {
        let my_generation = {
            let mut guard = generation.write();
            *guard += 1;
            *guard
        };
        spawn(async move {
            let raw = draft.peek().clone();
            let url = match crate::import::validate_program_url(&raw) {
                Ok(url) => url,
                Err(why) => {
                    error
                        .set(Some(crate::present::present_import_error(&why)));
                    return;
                }
            };
            phase.set(Some(crate::import::ImportPhase::Download));
            elapsed.set(0);
            error.set(None);
            let started = match crate::browser::start_import_fetch() {
                Ok(started) => std::rc::Rc::new(started),
                Err(why) => {
                    phase.set(None);
                    error
                        .set(Some(crate::present::present_import_error(&why)));
                    return;
                }
            };
            fetch.set(Some(started.clone()));
            let fetched =
                crate::browser::fetch_program_html(&url, &started).await;
            if *generation.peek() != my_generation {
                return;
            }
            let html = match fetched {
                Ok(html) => html,
                Err(crate::import::ImportError::Cancelled) => {
                    phase.set(None);
                    return;
                }
                Err(why) => {
                    phase.set(None);
                    error
                        .set(Some(crate::present::present_import_error(&why)));
                    return;
                }
            };
            phase.set(Some(crate::import::ImportPhase::Parse));
            // paint the phase line before the parse (HTML parse +
            // preparatory-rule BFS over the whole catalogue) blocks the
            // thread — same discipline as `load`'s own comment above, which
            // this path had dropped while still claiming LAT-4/5 phase
            // feedback
            crate::browser::next_frame().await;
            let iso = crate::browser::now_iso();
            let secs = crate::browser::now_secs();
            let local = {
                let read = snapshot.peek();
                let courses = read
                    .as_ref()
                    .map(|snapshot| snapshot.courses.as_slice())
                    .unwrap_or_default();
                crate::import::build_local_program(
                    &html, &url, iso, secs, courses,
                )
            };
            let local = match local {
                Ok(local) => local,
                Err(why) => {
                    phase.set(None);
                    error
                        .set(Some(crate::present::present_import_error(&why)));
                    return;
                }
            };
            phase.set(Some(crate::import::ImportPhase::Save));
            let saved = {
                let mut write = snapshot.write();
                write.as_mut().map(|snapshot_mut| {
                    crate::data::add_local_program(snapshot_mut, local.clone())
                })
            };
            match saved {
                None => {
                    phase.set(None);
                    error.set(Some(crate::present::present_import_error(
                        &crate::import::ImportError::CatalogueUnavailable,
                    )));
                    return;
                }
                Some(Err(detail)) => {
                    phase.set(None);
                    error.set(Some(
                        crate::present::present_local_program_conflict(
                            &detail,
                        ),
                    ));
                    return;
                }
                Some(Ok(())) => {}
            }
            let title = local.program.title.clone();
            local_programs.write().push(local);
            crate::browser::local_set(
                crate::persist::LOCAL_PROGRAMS_KEY,
                &crate::persist::encode_local_programs(&local_programs.read()),
            );
            phase.set(None);
            draft.set(String::new());
            open.set(false);
            super::push_alert(
                alerts,
                super::AlertBody::Success(format!("{title} importé.")),
            );
        });
    };

    let cancel = move |_| {
        // bumped first: the in-flight task's only guard is `*generation.peek()
        // != my_generation` (line 326), so a fetch already resolved and
        // sitting in the task queue at click time must still see itself as
        // stale, or Annuler would not stop the import it just told the
        // student it stopped
        *generation.write() += 1;
        if let Some(started) = fetch.peek().as_ref() {
            started.abort();
        }
        phase.set(None);
    };

    // The JSON sibling of `commit` above: no phases (a local file read has
    // nothing worth naming in stages), same discipline otherwise — every
    // step must succeed before anything is written, any failure lands in
    // the shared `error` signal, never a partially added program.
    let mut json_commit = move || {
        let program_file = json_program_file.peek().clone();
        let Some(program_file) = program_file else {
            return;
        };
        json_loading.set(true);
        error.set(None);
        spawn(async move {
            let program_text =
                match crate::browser::read_file_text(&program_file).await {
                    Ok(text) => text,
                    Err(why) => {
                        json_loading.set(false);
                        error.set(Some(crate::present::present_import_error(
                            &why,
                        )));
                        return;
                    }
                };
            let file_name = program_file.name();
            let iso = crate::browser::now_iso();
            let local = crate::import::build_local_program_from_json(
                &program_text,
                &file_name,
                iso,
            );
            let local = match local {
                Ok(local) => local,
                Err(why) => {
                    json_loading.set(false);
                    error
                        .set(Some(crate::present::present_import_error(&why)));
                    return;
                }
            };
            let saved = {
                let mut write = snapshot.write();
                write.as_mut().map(|snapshot_mut| {
                    crate::data::add_local_program(snapshot_mut, local.clone())
                })
            };
            match saved {
                None => {
                    json_loading.set(false);
                    error.set(Some(crate::present::present_import_error(
                        &crate::import::ImportError::CatalogueUnavailable,
                    )));
                    return;
                }
                Some(Err(detail)) => {
                    json_loading.set(false);
                    error.set(Some(
                        crate::present::present_local_program_conflict(
                            &detail,
                        ),
                    ));
                    return;
                }
                Some(Ok(())) => {}
            }
            let title = local.program.title.clone();
            local_programs.write().push(local);
            crate::browser::local_set(
                crate::persist::LOCAL_PROGRAMS_KEY,
                &crate::persist::encode_local_programs(&local_programs.read()),
            );
            json_loading.set(false);
            // `open.set(false)` unmounts the whole form below, file input
            // included — that remount, not any bookkeeping here, is what
            // clears the browser's own memory of the chosen file
            json_program_file.set(None);
            open.set(false);
            super::push_alert(
                alerts,
                super::AlertBody::Success(format!("{title} importé.")),
            );
        });
    };

    // pre-computed for the button below (AP-5): the stated reason it is
    // disabled, or what the click will do
    let json_ready = json_program_file.read().is_some();
    let json_disabled = busy || !json_ready;
    let json_load_title = if json_loading() {
        "Chargement en cours…"
    } else if !json_ready {
        "Choisissez d'abord un fichier programme (.json)"
    } else {
        "Charger le programme depuis le fichier choisi"
    };
    // the native file control is hidden under the styled row below, so the
    // chosen name has to be rendered by us — never an unlabelled state
    let json_file_label = json_program_file
        .read()
        .as_ref()
        .map(|file| file.name())
        .unwrap_or_else(|| "Aucun fichier choisi".to_string());

    rsx! {
        div { class: "panel-import",
            button {
                class: "panel-import-toggle",
                r#type: "button",
                aria_expanded: expanded,
                disabled: busy,
                onclick: move |_| open.set(!open()),
                "Votre programme n'est pas là ?"
            }
            if expanded {
                div { class: "panel-import-form",
                    label {
                        class: "panel-import-label",
                        r#for: "panel-import-url",
                        "Adresse de la page programme"
                    }
                    input {
                        id: "panel-import-url",
                        class: "panel-import-input",
                        r#type: "url",
                        placeholder: "https://www.ulaval.ca/etudes/programmes/…",
                        disabled: busy,
                        value: "{draft}",
                        oninput: move |event| draft.set(event.value()),
                        onkeydown: move |event| {
                            if event.key() == Key::Enter {
                                commit();
                            }
                        },
                    }
                    button {
                        class: "panel-import-submit",
                        r#type: "button",
                        disabled: busy,
                        onclick: move |_| commit(),
                        "Importer"
                    }
                }
                // the file-picker sibling, à côté de l'import par URL (plan
                // item 6): the native `<input type="file">` renders in the
                // browser's own chrome, foreign to the rest of the panel —
                // it stays for the click target and the keyboard focus, but
                // sits invisible over a row styled like every other control
                // (opacity 0, full-size), and the chosen name is printed by
                // the row itself. A fixed shape either way (LAY-1).
                div { class: "panel-import-json",
                    label {
                        class: "panel-import-label",
                        r#for: "panel-import-json-program",
                        "Fichier programme ({{code}}-{{semestre}}.json)"
                    }
                    div { class: "panel-import-file",
                        input {
                            id: "panel-import-json-program",
                            class: "panel-import-file-input",
                            r#type: "file",
                            accept: ".json,application/json",
                            aria_label: "Fichier programme JSON",
                            disabled: busy,
                            onchange: move |event: Event<FormData>| {
                                json_program_file
                                    .set(event.files().into_iter().next());
                            },
                        }
                        span { class: "panel-import-file-button",
                            "Choisir un fichier…"
                        }
                        span { class: "panel-import-file-name",
                            "{json_file_label}"
                        }
                    }
                    button {
                        class: "panel-import-submit",
                        r#type: "button",
                        aria_label: "Charger le programme depuis un fichier JSON",
                        title: "{json_load_title}",
                        disabled: json_disabled,
                        onclick: move |_| json_commit(),
                        "Charger"
                    }
                }
                if importing {
                    ol { class: "panel-import-phases",
                        for (label, glyph, running) in phase_rows.iter() {
                            li {
                                key: "{label}",
                                class: "panel-import-phase",
                                span { class: "panel-import-phase-glyph", "{glyph}" }
                                span { class: "panel-import-phase-label", "{label}" }
                                if *running {
                                    span { class: "panel-import-phase-elapsed",
                                        "{elapsed} s écoulées"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "panel-import-cancel",
                        r#type: "button",
                        onclick: cancel,
                        "Annuler"
                    }
                }
            }
            if let Some(error) = error() {
                // demande d'Antoine (2026-08-26) : le bloc en cinq parties
                // (ERR-1) est réduit ici au titre et au détail technique —
                // ce qui a été refusé, et pourquoi, sans le reste
                div { class: "panel-import-error", role: "alert",
                    p { class: "panel-import-error-what", "{error.what}" }
                    pre { class: "panel-import-error-detail",
                        "{error.detail}"
                    }
                }
            }
        }
    }
}

// Les deux menus du cheminement (décision 2026-08-19) : la concentration
// et le profil se changent ici, à tout moment — les sections en dessous et
// le bilan recomposent, la grille placée ne bouge pas (parité avec la
// version JS). Un menu sans choix réel n'est pas rendu (B-GEX n'a pas de
// concentrations); un programme qui n'offre ni l'un ni l'autre n'a pas la
// rangée (M-GEX).
#[component]
fn CheminementKnobs() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let choices = use_memo(move || {
        let read = snapshot.read();
        let snapshot = read.as_ref()?;
        panel::cheminement_choices(snapshot, &plan.read())
    });
    let Some(choices) = choices() else {
        return rsx! {};
    };
    rsx! {
        div { class: "panel-knobs",
            if !choices.concentrations.is_empty() {
                label { class: "panel-knob panel-knob--cheminement",
                    "Concentration"
                    select {
                        onchange: move |event: Event<FormData>| {
                            set_scope(
                                plan,
                                history,
                                alerts,
                                snapshot,
                                'c',
                                event.value(),
                            );
                        },
                        if choices.offers_none {
                            option {
                                value: "",
                                selected: choices.concentration.is_none(),
                                "Aucune"
                            }
                        }
                        for title in choices.concentrations.iter() {
                            option {
                                key: "{title}",
                                value: "{title}",
                                selected: choices.concentration.as_deref()
                                    == Some(title.as_str()),
                                "{title}"
                            }
                        }
                    }
                }
            }
            if !choices.profiles.is_empty() {
                label { class: "panel-knob panel-knob--cheminement",
                    "Profil"
                    select {
                        onchange: move |event: Event<FormData>| {
                            set_scope(
                                plan,
                                history,
                                alerts,
                                snapshot,
                                'f',
                                event.value(),
                            );
                        },
                        option {
                            value: "",
                            selected: choices.profile.is_none(),
                            "Aucun"
                        }
                        for title in choices.profiles.iter() {
                            option {
                                key: "{title}",
                                value: "{title}",
                                selected: choices.profile.as_deref()
                                    == Some(title.as_str()),
                                "{title}"
                            }
                        }
                    }
                }
            }
        }
    }
}

// One door for both menus: the act is labelled and undoable, the grid is
// left untouched, and the ententes attached to the outgoing block are
// retired and announced — their rule changed meaning with the block.
fn set_scope(
    plan: Signal<Plan>,
    history: Signal<History>,
    alerts: Signal<super::AlertStack>,
    snapshot: Signal<Option<Snapshot>>,
    prefix: char,
    value: String,
) {
    let title = (!value.is_empty()).then_some(value);
    let current = {
        let read = plan.read();
        let (concentration, profile) = panel::scope_of(&read);
        if prefix == 'c' {
            concentration.map(str::to_string)
        } else {
            profile.map(str::to_string)
        }
    };
    if current == title {
        return;
    }
    // everything that leaves with the departing block — what nothing under
    // the new scope lists, plus what the student chose under it;
    // materialized before the edit, purged inside it
    let departures = {
        let read = snapshot.read();
        let plan_read = plan.read();
        read.as_ref()
            .and_then(|snapshot| panel::chosen_program(snapshot, &plan_read))
            .map(|program| {
                let (concentration, profile) = panel::scope_of(&plan_read);
                let (departing, next_c, next_f) = if prefix == 'c' {
                    (concentration, title.as_deref(), profile)
                } else {
                    (profile, concentration, title.as_deref())
                };
                panel::block_departures(
                    program, &plan_read, departing, next_c, next_f, prefix,
                )
            })
            .unwrap_or_default()
    };
    let label = match (prefix, title.as_deref()) {
        ('c', Some(title)) => format!("Concentration : {title}"),
        ('c', None) => "Concentration retirée".to_string(),
        (_, Some(title)) => format!("Profil : {title}"),
        (_, None) => "Profil retiré".to_string(),
    };
    let mut dropped = Vec::new();
    edit_plan(plan, history, &label, |plan| {
        if let Some(choice) = plan.program.as_mut() {
            if prefix == 'c' {
                choice.concentration = title;
            } else {
                choice.profile = title;
            }
        }
        dropped = state::purge_scope_grants(plan, prefix);
        state::purge_codes(plan, &departures);
    });
    // Document-caused: after a program swap the history is fresh and the
    // « Annuler » these advertise no longer applies
    if !dropped.is_empty() {
        super::push_caused_alert(
            alerts,
            super::AlertBody::Note(format!(
                "Ententes retirées avec l'ancien choix : {} — « Annuler » \
                 les restaure.",
                dropped.join(", ")
            )),
            super::AlertCause::Document,
        );
    }
    if !departures.is_empty() {
        super::push_caused_alert(
            alerts,
            super::AlertBody::Note(format!(
                "Cours retirés avec l'ancien bloc : {} — « Annuler » les \
                 restaure.",
                departures.join(", ")
            )),
            super::AlertCause::Document,
        );
    }
}

#[component]
fn PanelBody(model: PanelModel) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let search = view.read().search.clone();
    let searching = !search.trim().is_empty();
    let has_program = plan.read().program.is_some();
    // armed by typing only: a restored page mounts a saved search too,
    // and an auto-refresh never scrolls (LAT-7)
    let mut scroll_to_results = use_signal(|| false);
    rsx! {
        div { class: "panel-body",
            if let Some(error) = model.coverage_error.as_ref() {
                p { class: "warning", "⚠ {error}" }
            }
            for warning in model.warnings.iter() {
                p { class: "warning", "⚠ {warning}" }
            }
            if has_program {
                CheminementKnobs {}
                OrganigrammeControls { rules_missing: missing_rules(&model) }
                for group in model.groups.iter().cloned() {
                    GroupView { key: "{group.title}", group }
                }
                if let Some(preparatory) = model.preparatory.clone() {
                    SectionView { section: preparatory }
                }
                if let Some(note) = model.language_note.as_ref() {
                    p { class: "panel-note", b { "{note}" } }
                }
                for note in model.notes.iter() {
                    p { class: "panel-note", "{note}" }
                }
            }
            div { class: "panel-search-wrap",
                input {
                    class: "panel-search",
                    r#type: "search",
                    placeholder: "Chercher dans tout le catalogue…",
                    aria_label: "Chercher dans tout le catalogue…",
                    value: "{search}",
                    oninput: move |event| {
                        scroll_to_results.set(true);
                        view.write().search = event.value();
                    },
                }
                if searching {
                    button {
                        class: "panel-search-clear",
                        aria_label: "Effacer la recherche",
                        onclick: move |_| view.write().search.clear(),
                        "✕"
                    }
                }
            }
            if searching {
                div {
                    // the field sits at the foot of the panel, so its
                    // results open under the fold (rapport étudiante-gex
                    // 2026-08-19) — the first typed letter brings them
                    // into view
                    // deliberately stays Nearest, unlike SectionView and
                    // Disclosure's Start (F2, 2026-08-28): this scroll
                    // re-arms on every keystroke rather than once per open,
                    // so Start would re-scroll the results list to the top
                    // of the panel after each letter typed — Nearest holds
                    // still once the results are already in view
                    onmounted: move |event: Event<MountedData>| {
                        if !scroll_to_results() {
                            return;
                        }
                        scroll_to_results.set(false);
                        spawn(async move {
                            let _ = event
                                .data()
                                .scroll_to_with_options(ScrollToOptions {
                                    behavior: ScrollBehavior::Smooth,
                                    vertical: ScrollLogicalPosition::Nearest,
                                    horizontal:
                                        ScrollLogicalPosition::Nearest,
                                })
                                .await;
                        });
                    },
                    SearchResults {}
                }
            }
        }
    }
}

// The organigramme block (jalons 7–9): the horizon's knobs, the propose /
// verify buttons, and the verify verdict. The search itself runs in the
// worker — this thread never blocks (LAT-3).
// how many counted sections still miss something — the verdict must never
// say « complet » while a red badge sits right under it
fn missing_rules(model: &PanelModel) -> usize {
    model
        .groups
        .iter()
        .flat_map(|group| group.sections.iter())
        .chain(model.preparatory.iter())
        .filter(|section| {
            matches!(section.badge, Badge::Missing(_) | Badge::Partial(_))
        })
        .count()
}

#[component]
fn GroupView(group: PanelGroup) -> Element {
    rsx! {
        section { class: "panel-group",
            div { class: "panel-group-head",
                h2 { class: "panel-group-title", "{group.title}" }
                if let Some(progress) = group.progress.as_ref() {
                    span { class: "panel-group-progress", "{progress}" }
                }
            }
            if let Some(note) = group.note.as_ref() {
                p { class: "panel-note", "{note}" }
            }
            for section in group.sections.iter().cloned() {
                SectionView { key: "{section.key}", section }
            }
        }
    }
}

#[component]
fn OrganigrammeControls(rules_missing: usize) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let solver = use_context::<Signal<super::SolverState>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let mut horizon_epoch = use_signal(|| 0usize);
    let start = plan.read().start.to_string();
    // the span follows the real clock instead of a frozen A24-A31 window,
    // and always widens to the plan's own start — a relevé Capsule import
    // (`capsule::apply_to_plan`) anchors it years back for a 4th/5th-year
    // student, and an option list missing that year would leave `selected`
    // matching nothing and the select silently showing the wrong session
    let today =
        crate::state::semester_of_epoch_ms(crate::browser::now_epoch_ms());
    let (year_lo, year_hi) =
        state::start_year_window(plan.read().start, today);
    let study_sessions = plan.read().study_sessions;
    let horizon_floor = state::horizon_floor(&plan.read());
    let credit_cap = plan.read().credit_cap;
    let summers_open = plan.read().summers_open;
    let concomitant = plan.read().concomitant;
    let left_out = solver.read().left_out.clone();
    // jamais un ✓ à côté d'un état provisoire (rapport directeur-gci
    // 2026-08-29 ; ADR `2026-08-recalcul-visible-sur-la-grille`)
    let searching = solver.read().running.is_some();
    let (verdict_class, verdict_label) =
        crate::present::verification_verdict(searching);
    let verification = solver.read().verification.clone();
    let credit_shortfalls = solver.read().credit_shortfalls.clone();
    let shortfall_messages: Vec<String> = credit_shortfalls
        .iter()
        .map(|shortfall| {
            crate::solve::credit_shortfall_message(shortfall, &plan.read())
        })
        .collect();
    let nothing_placed = {
        let plan_read = plan.read();
        plan_read.displayed_placement.is_empty()
            && plan_read.manual.values().all(Vec::is_empty)
    };
    // the placement and the verification both run by themselves (ADR
    // `2026-08-organigramme-en-continu-sans-bouton`) — this only explains
    // why no verdict shows yet: courses still floating, or an unreadable
    // input
    let readiness = {
        let read = snapshot.read();
        let plan_read = plan.read();
        read.as_ref().map(|snapshot| {
            let program = panel::effective_program(snapshot, &plan_read);
            solve::unplaced_codes(snapshot, &plan_read, program.as_ref())
        })
    };
    // facts the verdict must never contradict: which sessions clash or
    // overflow, by name (rapport étudiante : « quelle session ? »)
    let (conflicted, overloaded) = {
        let read = snapshot.read();
        let plan_read = plan.read();
        match read.as_ref() {
            None => (String::new(), String::new()),
            Some(snapshot) => {
                let seasons = ulaval_scheduler_core::horizon_sessions(
                    plan_read.start.season,
                    plan_read.study_sessions,
                );
                let semesters =
                    state::session_semesters(plan_read.start, &seasons);
                let label = |session: usize| {
                    state::session_label(&semesters, session - 1)
                };
                let conflicted: Vec<String> =
                    solve::conflicted_sessions(snapshot, &plan_read)
                        .into_iter()
                        .map(label)
                        .collect();
                let overloaded: Vec<String> = (1..=semesters.len())
                    .filter(|&session| {
                        solve::session_credits(snapshot, &plan_read, session)
                            .total
                            > plan_read.credit_cap
                    })
                    .map(label)
                    .collect();
                (conflicted.join(", "), overloaded.join(", "))
            }
        }
    };
    rsx! {
        div { class: "panel-organigramme",
            div { class: "panel-knobs",
                label { class: "panel-knob",
                    "Début"
                    select {
                        onchange: move |event| {
                            let value = event.value();
                            if let Ok(semester) = value.parse() {
                                edit_plan(
                                    plan,
                                    history,
                                    &format!("Début déplacé à {value}"),
                                    |plan| plan.start = semester,
                                );
                            }
                        },
                        for year in year_lo..=year_hi {
                            option {
                                value: "A{year}",
                                selected: start == format!("A{year}"),
                                "A{year}"
                            }
                            option {
                                value: "H{year}",
                                selected: start == format!("H{year}"),
                                "H{year}"
                            }
                        }
                    }
                }
                label { class: "panel-knob",
                    "Sessions"
                    input {
                        // remounted whenever a typed value was clamped, so
                        // the field snaps back to the horizon actually set
                        // instead of displaying a number the plan refused
                        key: "sessions-{horizon_epoch}",
                        r#type: "number",
                        min: "{horizon_floor}",
                        max: "{MAX_STUDY_SESSIONS}",
                        value: "{study_sessions}",
                        onchange: move |event| {
                            if let Ok(count) = event.value().parse::<usize>()
                            {
                                // the floor holds the pinned, manual and
                                // completed sessions; the label names the
                                // horizon actually set, not the one typed
                                let clamped = count.clamp(
                                    state::horizon_floor(&plan.peek()),
                                    MAX_STUDY_SESSIONS,
                                );
                                if clamped != count {
                                    // a clamped knob explains itself —
                                    // never a silent refusal
                                    super::push_alert(
                                        alerts,
                                        super::AlertBody::Note(
                                            state::horizon_floor_note(
                                                &plan.peek(),
                                            ),
                                        ),
                                    );
                                    horizon_epoch += 1;
                                }
                                edit_plan(
                                    plan,
                                    history,
                                    &format!("Horizon à {clamped} sessions"),
                                    |plan| {
                                        state::set_horizon(plan, clamped);
                                    },
                                );
                            }
                        },
                    }
                }
                label {
                    class: "panel-knob",
                    title: "Nombre maximal de crédits par session. Le \
                            placement automatique ne dépasse jamais ce \
                            plafond ; un placement à la main peut le \
                            dépasser et l'outil l'avertit. 17 cr est la \
                            charge pleine usuelle — ajustable.",
                    "Plafond (cr)"
                    input {
                        r#type: "number",
                        min: "3",
                        max: "30",
                        value: "{credit_cap}",
                        onchange: move |event| {
                            if let Ok(cap) = event.value().parse::<u32>() {
                                let clamped = cap.clamp(3, 30);
                                edit_plan(
                                    plan,
                                    history,
                                    &format!("Plafond à {clamped} cr"),
                                    |plan| plan.credit_cap = clamped,
                                );
                            }
                        },
                    }
                }
            }
            CapsuleDrawer {}
            label { class: "panel-fit",
                input {
                    r#type: "checkbox",
                    checked: summers_open,
                    onchange: move |event| {
                        let open = event.checked();
                        let label = if open {
                            "Étés ouverts aux cours réguliers"
                        } else {
                            "Étés refermés"
                        };
                        edit_plan(plan, history, label, |plan| {
                            plan.summers_open = open;
                        });
                    },
                }
                "Ouvrir les étés aux cours réguliers"
            }
            label {
                class: "panel-fit",
                title: "Concomitance : le préalable peut être suivi en \
                        même temps (même session) que le cours qui \
                        l'exige, plutôt qu'avant.",
                input {
                    r#type: "checkbox",
                    checked: concomitant,
                    onchange: move |event| {
                        let allowed = event.checked();
                        let label = if allowed {
                            "Concomitance permise"
                        } else {
                            "Concomitance refusée"
                        };
                        edit_plan(plan, history, label, |plan| {
                            plan.concomitant = allowed;
                        });
                    },
                }
                "Permettre un préalable en concomitance"
            }
            // Tous les verdicts asynchrones (propose/verify, 500 ms
            // après la dernière saisie) se regroupent ici, au même
            // endroit — sans hauteur réservée : le panneau défile de
            // toute façon, exception LAY-2 assumée (ADR
            // `2026-08-verdicts-du-panneau-sans-hauteur-reservee`).
            div { class: "panel-verdicts",
                if !conflicted.is_empty() {
                    p { class: "panel-verdict panel-verdict--bad",
                        "⚠ Conflit d'horaire en {conflicted} — plages \
                         hachurées dans la grille de ces sessions."
                    }
                }
                if !overloaded.is_empty() {
                    p { class: "panel-verdict panel-verdict--bad",
                        "⚠ Plafond de crédits dépassé en {overloaded}."
                    }
                }
                for message in shortfall_messages.iter() {
                    p { class: "panel-verdict panel-verdict--bad",
                        "⚠ {message}"
                    }
                }
                match readiness {
                    Some(Err(why)) => rsx! {
                        p { class: "warning",
                            "⚠ Vérification impossible : {why}"
                        }
                    },
                    // an empty grid is a verdict of its own — « rempli au
                    // mieux » would minimize a total failure (rapport
                    // étudiante-cegep 2026-08-19, B-GMC à 0/120)
                    Some(Ok(unplaced))
                        if !unplaced.is_empty()
                            && !left_out.is_empty()
                            && nothing_placed =>
                    {
                        rsx! {
                            p { class: "panel-verdict panel-verdict--bad",
                                "Aucun cours n'a pu être placé. Montez le \
                                 plafond de crédits, ajoutez des sessions ou \
                                 retirez des cours — le placement repartira \
                                 de lui-même."
                            }
                        }
                    }
                    // « proposez un organigramme » is false once the solver
                    // has just tried and reported what does not fit — the
                    // 2026-08-14 report's « c'est justement ce que je viens de
                    // faire » (ADR `2026-08-placement-au-mieux-en-repli`)
                    Some(Ok(unplaced))
                        if !unplaced.is_empty() && !left_out.is_empty() =>
                    {
                        rsx! {
                            p { class: "panel-verdict",
                                "{unplaced.len()} cours sans session : le \
                                 solveur a rempli au mieux et n'a pas pu les \
                                 placer — voyez les messages pour la raison, \
                                 puis ajustez le plafond, les sessions ou les \
                                 cours, ou placez-les à la main."
                            }
                        }
                    }
                    Some(Ok(unplaced)) if !unplaced.is_empty() => rsx! {
                        p { class: "panel-verdict",
                            "{unplaced.len()} cours sans session — placement \
                             automatique en cours…"
                        }
                    },
                    _ => rsx! {},
                }
                if let Some(verification) = verification {
                    if !verification.solutions.is_empty() {
                        // un cheminement complet ne dit plus rien : seuls les
                        // manques parlent (décision d'Antoine, 2026-08-26)
                        if rules_missing > 0 || !credit_shortfalls.is_empty() {
                            p { class: "{verdict_class}", "{verdict_label}" }
                            p { class: "panel-verdict panel-verdict--bad",
                                "⚠ mais {rules_missing} sections de règles \
                                 restent à combler ci-dessous — le bac n'est \
                                 pas complet."
                            }
                        }
                    } else {
                        p { class: "panel-verdict panel-verdict--bad",
                            "⚠ Le cheminement affiché brise une contrainte "
                            "(préalable, plafond, été fermé ou conflit \
                             d'horaire) — les avertissements ci-dessus et \
                             l'en-tête nomment ce qui dépasse."
                        }
                    }
                    for blocked in verification.blocked.iter() {
                        p { class: "warning",
                            "⚠ {crate::solve::blocked_note(blocked)}"
                        }
                    }
                }
            }
        }
    }
}

// Le tiroir « Charger depuis Capsule » (plan item 5) : replié au repos,
// comme `ImportDrawer` — jamais de modal (ADR
// `2026-08-collage-capsule-en-tiroir-non-bloquant`). Toute la logique
// (parseur, application au plan, bilan) vit dans `crate::capsule`, déjà
// testée nativement; ce composant ne fait que coller le texte, appeler
// `capsule::load`/`apply_to_plan` et afficher ce qu'ils renvoient (AP-5).
#[component]
fn CapsuleDrawer() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();

    let mut open = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut error = use_signal(|| None::<crate::present::UiError>);
    let mut summary = use_signal(|| None::<crate::capsule::CapsuleSummary>);
    let mut detail_open = use_signal(|| false);

    // `commit` below wraps the import in one undoable `edit_plan` act
    // (ACT-2), but `summary` is a signal local to this drawer that no undo
    // touches on its own — left alone, ctrl-z reverts the plan while the
    // drawer keeps claiming the old bilan (« 18 cours placés… ») for a plan
    // that no longer holds them. As long as this import's act still sits
    // on top of the undo stack, the bilan describes the current plan; the
    // moment it doesn't — undone, or buried under a later edit — the bilan
    // is stale and clears itself.
    use_effect(move || {
        let current_top = history.read().undo_label().map(str::to_string);
        if current_top.as_deref() != Some("Import Capsule") {
            summary.set(None);
        }
    });

    let empty = draft.read().trim().is_empty();
    let load_title = if empty {
        "Collez d'abord le texte du relevé"
    } else {
        "Charger le relevé collé"
    };

    let mut commit = move || {
        let text = draft.peek().clone();
        let minimum = plan.peek().study_sessions;
        // the catalogue gate: `load` refuses any sigle the snapshot does
        // not carry, so the plan the import writes is one every solver
        // request can resolve (ADR
        // `2026-08-sigles-inconnus-du-releve-ignores`)
        let known = snapshot.peek().as_ref().map(|snapshot| {
            snapshot
                .by_code
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<String>>()
        });
        let Some(known) = known else {
            summary.set(None);
            error.set(Some(crate::present::present_capsule_error(
                &crate::capsule::CapsuleError::CatalogueUnavailable,
            )));
            return;
        };
        match crate::capsule::load(&text, minimum, &known) {
            Err(why) => {
                summary.set(None);
                error.set(Some(crate::present::present_capsule_error(&why)));
            }
            Ok(loaded) => {
                error.set(None);
                // one labelled, reversible act (ACT-2): a single ctrl-z
                // undoes the whole import
                edit_plan(plan, history, "Import Capsule", |plan| {
                    crate::capsule::apply_to_plan(plan, &loaded.application);
                });
                // document-scoped: leaves with the document at a swap
                super::push_caused_alert(
                    alerts,
                    super::AlertBody::Success(loaded.summary.headline.clone()),
                    super::AlertCause::Document,
                );
                summary.set(Some(loaded.summary));
                draft.set(String::new());
                // relevé chargé : le formulaire a fini son travail — le
                // tiroir se replie comme `ImportDrawer`, le bilan reste
                open.set(false);
            }
        }
    };

    rsx! {
        div { class: "panel-capsule",
            button {
                class: "panel-import-toggle",
                r#type: "button",
                aria_expanded: open(),
                onclick: move |_| open.set(!open()),
                "Charger depuis Capsule"
            }
            if open() {
                div { class: "panel-capsule-form",
                    p { class: "panel-capsule-instructions",
                        "Ouvrez votre relevé de notes dans Capsule, faites "
                        code { "ctrl-u" }
                        " pour afficher la source de la page, puis "
                        code { "ctrl-a" }
                        " et "
                        code { "ctrl-c" }
                        " pour tout copier, et collez le résultat ci-dessous."
                    }
                    label {
                        class: "panel-import-label",
                        r#for: "panel-capsule-textarea",
                        "Source de la page du relevé"
                    }
                    textarea {
                        id: "panel-capsule-textarea",
                        class: "panel-capsule-textarea",
                        placeholder: "Collez ici la source de la page…",
                        value: "{draft}",
                        oninput: move |event| draft.set(event.value()),
                    }
                    button {
                        class: "panel-import-submit",
                        r#type: "button",
                        title: "{load_title}",
                        disabled: empty,
                        onclick: move |_| commit(),
                        "Charger"
                    }
                    if let Some(error) = error() {
                        div { class: "panel-import-error", role: "alert",
                            p { class: "panel-import-error-what", "{error.what}" }
                            p { "{error.reaction}" }
                            p { "{error.affected}" }
                            p { class: "panel-import-error-action", "{error.action}" }
                            p { class: "panel-import-error-id",
                                "Identifiant : "
                                code { "{error.id}" }
                            }
                            Disclosure {
                                open: detail_open(),
                                on_toggle: move |_| {
                                    detail_open.set(!detail_open())
                                },
                                class: "panel-capsule-error-detail",
                                summary: rsx! { "Détail technique" },
                                pre { "{error.detail}" }
                            }
                        }
                    }
                }
            }
            // hors du formulaire : le tiroir se replie au chargement, mais
            // le bilan — dont les lignes ignorées ou non reconnues, jamais
            // tues silencieusement — doit rester visible sans le rouvrir
            // the headline and the problems only: the grid already shows
            // the placed courses, and an ignored row must be in plain
            // sight, never folded away (demande d'Antoine, 2026-08-26)
            if let Some(summary) = summary() {
                div { class: "panel-capsule-summary",
                    p { class: "panel-capsule-headline", "{summary.headline}" }
                    if !summary.ignored.is_empty() {
                        div { class: "panel-capsule-bucket",
                            p { class: "panel-capsule-bucket-title",
                                "Cours ignorés ({summary.ignored.len()})"
                            }
                            ul {
                                for item in summary.ignored.iter() {
                                    li { key: "{item}", "{item}" }
                                }
                            }
                        }
                    }
                    if !summary.unrecognized.is_empty() {
                        div { class: "panel-capsule-bucket",
                            p { class: "panel-capsule-bucket-title",
                                "Lignes non reconnues ({summary.unrecognized.len()})"
                            }
                            ul {
                                for item in summary.unrecognized.iter() {
                                    li { key: "{item}", "{item}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// A controlled native `<details>`: the caller owns `open` and reacts to a
// click on the summary itself (`on_toggle`) — the state signal stays in
// the caller, this component only wires the click and the scroll. Unlike
// `SectionView`'s own accordion (a plain div/button whose content div is
// conditionally rendered, so a fresh `onmounted` fires on every open), a
// native `<details>` element is never unmounted by toggling its `open`
// attribute — the browser just hides its content — so `onmounted` here
// only captures the node handle once, at first mount, and every summary
// click reuses that handle to scroll when the click is the one that
// opens the disclosure (never on mount or restore, LAT-7), the same
// Start+Smooth as `SectionView` (ADR
// `2026-08-defilement-vers-la-section-depliee`).
#[component]
fn Disclosure(
    open: bool,
    on_toggle: EventHandler<()>,
    class: String,
    summary: Element,
    children: Element,
) -> Element {
    let mut node = use_signal(|| None::<std::rc::Rc<MountedData>>);
    rsx! {
        details {
            class: "{class}",
            open,
            onmounted: move |event: Event<MountedData>| {
                node.set(Some(event.data()));
            },
            summary {
                onclick: move |event| {
                    event.prevent_default();
                    let opening = !open;
                    on_toggle.call(());
                    if !opening {
                        return;
                    }
                    let Some(handle) = node.peek().clone() else {
                        return;
                    };
                    spawn(async move {
                        let _ = handle
                            .scroll_to_with_options(ScrollToOptions {
                                behavior: ScrollBehavior::Smooth,
                                vertical: ScrollLogicalPosition::Start,
                                horizontal: ScrollLogicalPosition::Nearest,
                            })
                            .await;
                    });
                },
                {summary}
            }
            {children}
        }
    }
}

// one accordion: header button, expansion strictly in place (LAY-2)
#[component]
fn SectionView(section: Section) -> Element {
    let mut view = use_context::<Signal<View>>();
    let expanded =
        view.read().expanded_rule.as_deref() == Some(section.key.as_str());
    // only the click below arms the scroll: a restored page mounts the
    // open section too, and an auto-refresh never scrolls (LAT-7)
    let mut scroll_on_open = use_signal(|| false);
    let key = section.key.clone();
    let (badge_class, badge_text) = match &section.badge {
        Badge::Ok(text) => ("panel-badge--ok", text.clone()),
        Badge::Partial(text) => ("panel-badge--partial", text.clone()),
        Badge::Missing(text) => ("panel-badge--missing", text.clone()),
        Badge::Neutral(text) => ("panel-badge--neutral", text.clone()),
    };
    let chevron = if expanded { "▾" } else { "▸" };
    // F8: a filter for rules with many rows (anglais, électifs…) — local
    // and volatile, never persisted, decided by the pure module (AP-5)
    let mut filter = use_signal(String::new);
    let filter_text = filter.read().clone();
    let filterable = panel::rule_filterable(&section.rows);
    let filtered_rows = panel::filter_rows(&section.rows, &filter_text);
    let filter_empty_result = filterable
        && !filter_text.trim().is_empty()
        && filtered_rows.is_empty();
    // a course taken here was chosen *under* this block, and leaves with it
    let origin = panel::section_origin(&section.key);
    rsx! {
        div {
            class: "panel-rule",
            class: if expanded { "panel-rule--open" },
            class: if matches!(section.badge, Badge::Missing(_)) { "panel-rule--missing" },
            button {
                class: "panel-rule-head",
                aria_expanded: expanded,
                onclick: move |_| {
                    if !expanded {
                        scroll_on_open.set(true);
                    }
                    let mut view = view.write();
                    view.expanded_rule = if expanded {
                        None
                    } else {
                        Some(key.clone())
                    };
                },
                span { class: "panel-rule-title",
                    "{section.title}"
                }
                span { class: "panel-badge {badge_class}", "{badge_text}" }
                span { class: "panel-rule-chevron", "{chevron}" }
            }
            if expanded {
                div { class: "panel-rule-content",
                    // bring what just opened into view — the panel is a
                    // long internal scroller and the content often lands
                    // under the fold (rapport étudiante-gex 2026-08-19);
                    // Start, not Nearest: a section taller than the panel
                    // has its top (heading, first course) still out of
                    // view under Nearest, which stops once *any* edge is
                    // visible (rapport étudiante-gex 2026-08-27)
                    onmounted: move |event: Event<MountedData>| {
                        if !scroll_on_open() {
                            return;
                        }
                        scroll_on_open.set(false);
                        spawn(async move {
                            let _ = event
                                .data()
                                .scroll_to_with_options(ScrollToOptions {
                                    behavior: ScrollBehavior::Smooth,
                                    vertical: ScrollLogicalPosition::Start,
                                    horizontal:
                                        ScrollLogicalPosition::Nearest,
                                })
                                .await;
                        });
                    },
                    if let Some(lead) = section.lead.as_ref() {
                        p { class: "panel-empty", "{lead}" }
                    }
                    if section.key
                        == format!(
                            "p/{}",
                            ulaval_scheduler_core::PREPARATORY_RULE_TITLE
                        )
                    {
                        PreparatoryToggle {}
                    }
                    if let Some(raw) = section.raw.as_ref() {
                        p { class: "panel-rule-raw", "{raw}" }
                    }
                    for note in section.notes.iter() {
                        p { class: "panel-rule-raw", "{note}" }
                    }
                    if filterable {
                        div { class: "panel-search-wrap",
                            input {
                                class: "panel-search panel-rule-filter",
                                r#type: "search",
                                placeholder: "Filtrer cette règle (sigle \
                                              ou titre)…",
                                aria_label: "Filtrer cette règle (sigle \
                                             ou titre)…",
                                value: "{filter_text}",
                                oninput: move |event| {
                                    filter.set(event.value())
                                },
                            }
                        }
                    }
                    if filter_empty_result {
                        p { class: "panel-empty",
                            "Aucun cours ne correspond — effacez le filtre."
                        }
                    }
                    // a free section lists what its ententes attached,
                    // then keeps browsing — the browse is how a course
                    // gets attached in the first place
                    for row in filtered_rows.iter().cloned() {
                        RowView { key: "{row.code}", row, origin }
                    }
                    if section.free {
                        FreeBrowse { grant_key: section.key.clone() }
                    }
                }
            }
        }
    }
}

// The prerequisites as the student's own program vintage wrote them. The
// field starts on what the solver currently reads — his correction if one
// is in force, his vintage's if the shared file carries one, the
// répertoire's otherwise — and commits on blur or Enter, never on a
// keystroke (INP-7); a rejected expression keeps the field intact. The live
// echo says what the grammar understood before the commit (INP-6), in words
// as well as in colour (INP-3).
#[component]
fn PrereqField(code: String) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let current = use_memo({
        let code = code.clone();
        move || {
            snapshot
                .read()
                .as_ref()
                .map(|snapshot| snapshot.prerequisites_draft(&code))
                .unwrap_or_default()
        }
    });
    let official = use_memo({
        let code = code.clone();
        move || {
            snapshot
                .read()
                .as_ref()
                .map(|snapshot| snapshot.official_prerequisites(&code))
                .unwrap_or_default()
        }
    });
    let mut prereq_open = use_signal(|| false);
    let mut draft = use_signal(|| current.read().0.clone());
    // an undo — or the vintage's own correction landing — moves the text
    // under the field; a stale draft would then commit what nobody asked
    use_effect(move || {
        let (text, _) = current.read().clone();
        draft.set(text);
    });
    let mine = plan.read().prereq_overrides.contains_key(&code);
    let text = draft.read().clone();
    let verdict = crate::present::present_prereq_draft(&text);
    let corrected = current.read().1;
    let summary = if corrected {
        "Préalables - corrigés"
    } else {
        "Préalables"
    };

    let commit = {
        let code = code.clone();
        move || {
            let text = draft.peek().trim().to_string();
            // rejected on commit, never on a keystroke — and the field is
            // left exactly as typed so nothing has to be retyped (INP-7)
            if !crate::present::present_prereq_draft(&text).valid {
                return;
            }
            if text == current.peek().0 {
                return;
            }
            let official = official.peek().clone();
            let code = code.clone();
            edit_plan(
                plan,
                history,
                &format!("Préalables de {code} corrigés"),
                move |plan| {
                    plan.prereq_overrides.insert(
                        code,
                        ulaval_scheduler_core::PrereqOverride {
                            text,
                            official: Some(official),
                        },
                    );
                },
            );
        }
    };

    rsx! {
        Disclosure {
            open: prereq_open(),
            on_toggle: move |_| prereq_open.set(!prereq_open()),
            class: "panel-prereq",
            summary: rsx! { "{summary}" },
            div { class: "panel-prereq-body",
                input {
                    class: "panel-prereq-input",
                    class: if !verdict.valid { "panel-prereq-input--invalid" },
                    "aria-invalid": if verdict.valid { "false" } else { "true" },
                    value: "{text}",
                    placeholder: "aucun préalable",
                    oninput: move |event| draft.set(event.value()),
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            commit();
                        }
                    },
                }
                // reserved whether or not it has something to say: the row
                // below never jumps as the student types (Core-5)
                div {
                    class: "panel-prereq-echo",
                    class: if !verdict.valid { "panel-prereq-echo--invalid" },
                    "{verdict.echo}"
                }
                if mine {
                    button {
                        class: "panel-prereq-reset",
                        title: "Rétablir les préalables du répertoire",
                        onclick: {
                            let code = code.clone();
                            move |_| {
                                let code = code.clone();
                                edit_plan(
                                    plan,
                                    history,
                                    &format!("Préalables de {code} rétablis"),
                                    move |plan| {
                                        plan.prereq_overrides.remove(&code);
                                    },
                                );
                            }
                        },
                        "✕ rétablir"
                    }
                }
            }
        }
    }
}

// « Scolarité préparatoire déjà faite » (notes 2026-08-13) — checked by
// default; checked, the 0xxx courses count as acquired (they ride as
// `passed`, ADR `2026-08-retrait-de-la-notion-de-cours-reussi`)
#[component]
fn PreparatoryToggle() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let done = plan.read().preparatory_done;
    rsx! {
        label { class: "panel-fit",
            input {
                r#type: "checkbox",
                checked: done,
                onchange: move |event| {
                    let done = event.checked();
                    let label = if done {
                        "Scolarité préparatoire marquée faite"
                    } else {
                        "Scolarité préparatoire à faire"
                    };
                    edit_plan(plan, history, label, |plan| {
                        plan.preparatory_done = done;
                    });
                },
            }
            "Scolarité préparatoire déjà faite"
        }
    }
}

// the « 3 cr libres » rule (design 8a): matière select over the whole
// first-cycle catalogue, plus the shared search and fit filter
// `grant_key` names the « tous les cours » rule this browse belongs to:
// taking a course here attaches it to that rule in the same act
#[component]
fn FreeBrowse(grant_key: String) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let subjects = panel::subjects(snapshot);
    let subject = view.read().subject.clone();
    let results = panel::search_courses(
        snapshot,
        &plan.read(),
        panel::SearchScope {
            subject: subject.as_deref(),
            first_cycle_only: true,
        },
        &view.read().search,
    );
    rsx! {
        div { class: "panel-free",
            select {
                class: "panel-subject",
                aria_label: "Matière",
                onchange: move |event| {
                    let value = event.value();
                    view.write().subject =
                        (value != "*").then_some(value);
                },
                option { value: "*", selected: subject.is_none(),
                    "Toutes les matières"
                }
                for (sigle, count) in subjects {
                    option {
                        value: "{sigle}",
                        selected: subject.as_deref() == Some(sigle.as_str()),
                        "{sigle} — {count} cours"
                    }
                }
            }
            ResultRows { results, grant_key }
        }
    }
}

#[component]
fn SearchResults() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let results = panel::search_courses(
        snapshot,
        &plan.read(),
        panel::SearchScope {
            subject: None,
            first_cycle_only: false,
        },
        &view.read().search,
    );
    rsx! {
        ResultRows { results }
    }
}

#[component]
fn ResultRows(
    results: panel::SearchResults,
    grant_key: Option<String>,
) -> Element {
    // a « tous les cours » browse belongs to its section's block; the
    // global search at the foot of the panel belongs to none
    let origin = grant_key.as_deref().and_then(panel::section_origin);
    rsx! {
        div { class: "panel-results",
            for row in results.rows.iter().cloned() {
                RowView {
                    key: "{row.code}",
                    row,
                    grant_key: grant_key.clone(),
                    origin,
                }
            }
            p { class: "panel-results-count",
                if results.matched == 0 {
                    "Aucun cours ne correspond — vérifiez le sigle ou le filtre."
                } else if results.matched > results.rows.len() {
                    "{results.rows.len()} premiers affichés sur {results.matched} correspondances"
                } else {
                    "{results.matched} cours"
                }
            }
        }
    }
}

// one course row: state carried by text and shape, the choice strip takes
// the course (automatique) or takes and freezes it (a session), the ✕
// drops it — immediate and undoable, never a dialog (ACT-2)
#[component]
fn RowView(
    row: Row,
    grant_key: Option<String>,
    origin: Option<char>,
) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    // the advisory fit marker, swap semantics — the probe comes from the
    // shared memo, each row costs one mask overlap
    let probe = use_context::<Memo<Option<panel::FitProbe>>>();
    let fit = {
        let read = snapshot.read();
        match (read.as_ref(), &*probe.read()) {
            (Some(snapshot), Some(probe))
                if row.state == RowState::Available =>
            {
                snapshot.by_code.get(&row.code).map(|&index| {
                    panel::quick_fit(probe, snapshot, &snapshot.courses[index])
                })
            }
            _ => None,
        }
    };
    // the note names the displayed session, which is no longer where a
    // click lands — advisory only, so it never dims the row either
    let fit_note = match fit {
        Some(Fit::Fits) => " - rentrerait dans la session affichée",
        Some(Fit::Conflicts) => " - conflit dans la session affichée",
        _ => "",
    };
    // Acquired, Credited and Unknown hold no session and offer no choice;
    // the rest get the strip, unmet prerequisites included — those warn,
    // they never wall (the student may know better — une entente, un cours
    // fait ailleurs)
    let strip = {
        let read = snapshot.read();
        read.as_ref()
            .filter(|_| {
                !matches!(
                    row.state,
                    RowState::Acquired
                        | RowState::Credited
                        | RowState::Unknown
                        // no choice strip for a row already counted by an
                        // earlier rule of its scope — the sub-text carries
                        // the state (« compté dans la Règle N »), the
                        // border only echoes it (AIR INP-3)
                        | RowState::CountedElsewhere
                )
            })
            .map(|snapshot| {
                panel::choice_strip(snapshot, &plan.read(), &row.code)
            })
    };
    // CountedElsewhere has no strip to read a choice from, but it is still
    // shown retained (panel-course--chosen) since a rule elsewhere already
    // counts it
    let chosen = row.state == RowState::CountedElsewhere
        || strip
            .as_ref()
            .is_some_and(|strip| strip.choice != panel::Choice::Not);
    let dimmed =
        matches!(row.state, RowState::PrereqUnmet | RowState::Unknown);
    let assumed = row.assumed.join(", ");
    rsx! {
        div {
            class: "panel-course",
            class: if dimmed { "panel-course--dimmed" },
            class: if chosen { "panel-course--chosen" },
            div { class: "panel-course-text",
                div { class: "panel-course-title", "{row.title}" }
                div { class: "panel-course-sub",
                    "{row.code} - {row.credits} - {row.sub}{fit_note}"
                }
                if !assumed.is_empty() {
                    div { class: "panel-course-sub",
                        "présumé acquis : {assumed}"
                    }
                }
                if !matches!(row.state, RowState::Unknown) {
                    PrereqField { code: row.code.clone() }
                }
            }
            // an acquired préparatoire row offers nothing — granting it
            // would resurrect it as ordinary work while the box says done
            if !matches!(row.state, RowState::Unknown | RowState::Acquired) {
                RuleAttach { code: row.code.clone() }
                CreditedToggle { code: row.code.clone() }
            }
            if let Some(strip) = strip {
                CourseChoice {
                    code: row.code.clone(),
                    strip,
                    grant_key,
                    origin,
                }
            }
        }
    }
}

// The choice strip (note d'Antoine 2026-08-17) : « automatique » takes the
// course and leaves the solver its session, a session chip takes it and
// freezes it there, and clicking another chip changes the choice — one
// labelled, undoable act each. The ✕ drops the course outright; a
// mandatory one has none, it is always taken (ADR
// `2026-08-choix-automatique-ou-session-gelee`).
#[component]
fn CourseChoice(
    code: String,
    strip: panel::ChoiceStrip,
    grant_key: Option<String>,
    // the block this course is being chosen under, if any — it leaves with
    // it (ADR `2026-08-electifs-choisis-sous-le-bloc-partent-avec-lui`)
    origin: Option<char>,
) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let choice = strip.choice;
    let auto = choice == panel::Choice::Auto;
    rsx! {
        div { class: "panel-course-choice",
            button {
                class: "panel-chip",
                class: if auto { "panel-chip--chosen" },
                aria_pressed: "{auto}",
                title: "Prendre {code} et laisser le solveur choisir sa \
                        session (le cours occupe une session, au \
                        contraire d'un cours crédité)",
                onclick: {
                    let code = code.clone();
                    let grant_key = grant_key.clone();
                    move |_| {
                        let code = code.clone();
                        if auto {
                            return;
                        }
                        let Some(warning) = take_verdict(
                            snapshot, plan, alerts, &code, choice, None,
                        ) else {
                            return;
                        };
                        // taken from a « tous les cours » browse: the
                        // entente rides in the same undoable act
                        let grant = panel::grant_on_take(
                            &plan.read(),
                            &code,
                            choice,
                            grant_key.as_deref(),
                        );
                        edit_plan(
                            plan,
                            history,
                            &format!("{code} laissé au solveur"),
                            |plan| {
                                if !plan.electives.contains(&code) {
                                    plan.electives.push(code.clone());
                                }
                                state::tag_elective_origin(
                                    plan, &code, origin,
                                );
                                if let Some(key) = grant {
                                    plan.rule_grants
                                        .insert(code.clone(), key);
                                }
                                // the pin falls, the placement stays: the
                                // grid keeps showing where it sits until
                                // the next solve may move it
                                plan.pinned_sessions.remove(&code);
                                for held in plan.manual.values_mut() {
                                    held.retain(|kept| kept != &code);
                                }
                            },
                        );
                        if let Some(warning) = warning {
                            super::push_alert(
                                alerts,
                                super::AlertBody::Note(warning),
                            );
                        }
                    }
                },
                "automatique"
            }
            for (session, label) in strip.sessions.clone() {
                {
                    let here = choice == panel::Choice::Pinned(session);
                    rsx! {
                        button {
                            key: "{session}",
                            class: "panel-chip",
                            class: if here { "panel-chip--chosen" },
                            aria_pressed: "{here}",
                            title: "Prendre {code} et le geler en {label}",
                            onclick: {
                                let code = code.clone();
                                let label = label.clone();
                                let grant_key = grant_key.clone();
                                move |_| {
                                    let code = code.clone();
                                    if here {
                                        return;
                                    }
                                    let Some(warning) = take_verdict(
                                        snapshot,
                                        plan,
                                        alerts,
                                        &code,
                                        choice,
                                        Some(session),
                                    ) else {
                                        return;
                                    };
                                    let grant = panel::grant_on_take(
                                        &plan.read(),
                                        &code,
                                        choice,
                                        grant_key.as_deref(),
                                    );
                                    place_course(
                                        plan, history, &code, session, &label,
                                        grant, origin,
                                    );
                                    if let Some(warning) = warning {
                                        super::push_alert(
                                            alerts,
                                            super::AlertBody::Note(warning),
                                        );
                                    }
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
            if strip.sessions.is_empty() {
                span { class: "panel-chip-none",
                    "aucune session de l'horizon ne l'offre"
                }
            }
            if choice != panel::Choice::Not && !strip.mandatory {
                button {
                    class: "panel-course-action",
                    title: "Retirer {code}",
                    onclick: {
                        let code = code.clone();
                        move |_| {
                            let code = code.clone();
                            edit_plan(
                                plan,
                                history,
                                &format!("{code} retiré"),
                                |plan| {
                                    crate::state::remove_course(plan, &code)
                                },
                            );
                            selected.set(None);
                        }
                    },
                    "✕"
                }
            }
        }
    }
}

// The same gate as the typed entry (saison, doublon, préalables) — a first
// take must not be a silent side door (rapport étudiante). Switching from
// one choice to another is a move: already accepted once, never re-judged.
// `None` means refused (the alert is already pushed); `Some(warning)` means
// go ahead, saying that if it is there.
fn take_verdict(
    snapshot: Signal<Option<Snapshot>>,
    plan: Signal<Plan>,
    alerts: Signal<super::AlertStack>,
    code: &str,
    choice: panel::Choice,
    session: Option<usize>,
) -> Option<Option<String>> {
    if choice != panel::Choice::Not {
        return Some(None);
    }
    // the read borrows must die before the caller opens the write one
    let verdict = {
        let read = snapshot.read();
        let plan_read = plan.read();
        read.as_ref().map(|snapshot| {
            solve::validate_new_code(snapshot, &plan_read, session, code)
        })
    };
    match verdict {
        Some(Ok(accepted)) => Some(accepted.warning),
        Some(Err(why)) => {
            super::push_alert(alerts, super::AlertBody::Note(why));
            None
        }
        None => None,
    }
}

// The « crédité » toggle (note d'Antoine 2026-08-17) : an agreement can
// credit a course outright — it counts in the credits, in the coverage and
// as a prerequisite, and takes no session. Marking one purges it from the
// placement in the *same* undoable act, so « Annuler » gives both back.
// It stacks with the entente select: crediting says the course is held,
// the select says which rule it counts toward.
#[component]
fn CreditedToggle(code: String) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let credited = plan.read().credited.contains(&code);
    let title = if credited {
        format!("Retirer le crédit de {code}")
    } else {
        format!(
            "Créditer {code} : compté sans occuper de session — \
             contrairement à « automatique », qui le place dans une \
             session"
        )
    };
    rsx! {
        button {
            class: "panel-credited",
            class: if credited { "panel-credited--on" },
            aria_pressed: "{credited}",
            title: "{title}",
            onclick: {
                let code = code.clone();
                move |_| {
                    let code = code.clone();
                    let label = if credited {
                        format!("Crédit retiré pour {code}")
                    } else {
                        format!("{code} crédité (entente)")
                    };
                    edit_plan(plan, history, &label, |plan| {
                        if credited {
                            crate::state::uncredit_code(plan, &code);
                        } else {
                            crate::state::credit_code(plan, &code);
                        }
                    });
                }
            },
            if credited { "crédité ✓" } else { "créditer" }
        }
    }
}

// The entente select (notes 2026-08-13) : attach the course to a rule it
// is not normally admitted in — an agreement with the direction, pure
// data (`panel::granted_program`), reversible from the same control.
#[component]
fn RuleAttach(code: String) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let rules = read
        .as_ref()
        .and_then(|snapshot| {
            let plan_read = plan.read();
            let program = panel::chosen_program(snapshot, &plan_read)?;
            let (concentration, profile) = panel::scope_of(&plan_read);
            Some(panel::grantable_rules(program, concentration, profile))
        })
        .unwrap_or_default();
    if rules.is_empty() {
        return rsx! {};
    }
    let current = plan.read().rule_grants.get(&code).cloned();
    let labels: std::collections::BTreeMap<String, String> =
        rules.iter().cloned().collect();
    rsx! {
        select {
            class: "panel-attach",
            aria_label: "Rattacher {code} à une règle (entente avec la direction)",
            title: "Entente : compter {code} dans une règle",
            onchange: {
                let code = code.clone();
                let labels = labels.clone();
                move |event: Event<FormData>| {
                    let value = event.value();
                    let code = code.clone();
                    if value == "-" {
                        edit_plan(
                            plan,
                            history,
                            &format!("Entente retirée pour {code}"),
                            |plan| {
                                plan.rule_grants.remove(&code);
                            },
                        );
                        return;
                    }
                    let title = labels
                        .get(&value)
                        .cloned()
                        .unwrap_or_else(|| value.clone());
                    edit_plan(
                        plan,
                        history,
                        &format!("{code} rattaché à « {title} » (entente)"),
                        |plan| {
                            plan.rule_grants.insert(code, value);
                        },
                    );
                }
            },
            option { value: "-", selected: current.is_none(),
                "entente avec la direction…"
            }
            for (key, title) in rules {
                option {
                    value: "{key}",
                    selected: current.as_deref() == Some(key.as_str()),
                    "{title}"
                }
            }
        }
    }
}

// Shared by the choice strip and the ribbon drop (note 16): place — or
// move — `code` into `session`, every previous trace of it cleared first;
// one labelled, undoable step. `grant` carries the entente a « tous les
// cours » browse decided (`panel::grant_on_take`) so it lands in the same
// act, and `origin` the block the course is being chosen under — the
// ribbon and the grid pass None for both: moving a course is not choosing
// it again, and must never overwrite the tag it already carries.
#[allow(clippy::too_many_arguments)]
pub fn place_course(
    plan: Signal<Plan>,
    history: Signal<History>,
    code: &str,
    session: usize,
    label: &str,
    grant: Option<String>,
    origin: Option<char>,
) {
    // the read borrow must die before edit_plan opens the write
    let already = plan.read().pinned_sessions.get(code) == Some(&session);
    if already {
        // same guard as the chip strip: re-pinning where the course
        // already sits would only stack an empty undo entry
        return;
    }
    let held = holding_session(&plan.read(), code);
    let action = if held.is_some() {
        format!("{code} déplacé vers {label}")
    } else {
        format!("{code} épinglé en {label}")
    };
    // the purge below drops the origin tag with the rest: a real choice
    // replaces it, a move gives back the one the course already carried
    let held = crate::state::elective_origin(&plan.read(), code);
    let code = code.to_string();
    edit_plan(plan, history, &action, |plan| {
        // a move leaves nothing behind: pin, placement, hand-added entry
        // and forced sections all go, then the course is laid down anew
        crate::state::purge_codes(plan, std::slice::from_ref(&code));
        crate::state::tag_elective_origin(plan, &code, origin.or(held));
        // placing a catalogue course IS choosing it: without the elective
        // the solver would see a pin with no Course behind it (rapport
        // étudiante : « MED-1100 is passed or pinned but has no Course »)
        if !plan.electives.contains(&code) {
            plan.electives.push(code.clone());
        }
        if let Some(key) = grant {
            plan.rule_grants.insert(code.clone(), key);
        }
        plan.pinned_sessions.insert(code.clone(), session);
        plan.displayed_placement.insert(code, session);
    });
}

// the session currently holding the course — placed by the solver or
// added by hand — if any
fn holding_session(plan: &Plan, code: &str) -> Option<usize> {
    plan.displayed_placement.get(code).copied().or_else(|| {
        plan.manual
            .iter()
            .find(|(_, codes)| codes.iter().any(|held| held == code))
            .map(|(&session, _)| session)
    })
}

// A course the catalogue does not carry (jalon 4, ADR
// `2026-07-contribution-de-cours-manuels`): entered whole (code, titre,
// crédits, plages), validated by the same serde types as the snapshot,
// marked « manuel » everywhere, shareable to the common catalogue through
// a prefilled GitHub issue — a plain anchor, no network, no token.
#[component]
fn ManualCourseForm() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let mut snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<super::AlertStack>>();
    let solver = use_context::<Signal<super::SolverState>>();
    let handle = use_context::<super::SolverHandle>();
    let super::ManualCourses(mut manual) =
        use_context::<super::ManualCourses>();
    let mut code = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut credits = use_signal(String::new);
    let mut nrc = use_signal(String::new);
    let mut slots = use_signal(|| {
        vec![crate::data::ManualSlot {
            day: "monday".to_string(),
            start: String::new(),
            end: String::new(),
        }]
    });
    let mut rejection = use_signal(|| None::<String>);
    // « fait partie de quelle règle ? » — the entente, granted on submit
    let mut rule = use_signal(|| "-".to_string());
    let mut manual_open = use_signal(|| false);
    let submit = {
        let handle = handle.clone();
        move |_| {
            let session = view.read().session;
            let Some(semester) =
                solve::session_semester(&plan.read(), session)
            else {
                rejection.set(Some(
                    "La session affichée est hors de l'horizon.".to_string(),
                ));
                return;
            };
            let draft = crate::data::ManualDraft {
                code: code.read().clone(),
                title: title.read().clone(),
                credits: credits.read().clone(),
                nrc: nrc.read().clone(),
                slots: slots.read().clone(),
            };
            let course = match crate::data::build_manual_course(
                &draft,
                semester.season,
                semester.year,
            ) {
                Ok(course) => course,
                Err(why) => {
                    rejection.set(Some(why));
                    return;
                }
            };
            let added = {
                let mut snapshot = snapshot.write();
                match snapshot.as_mut() {
                    None => Err("Catalogue pas encore chargé.".to_string()),
                    Some(snapshot) => crate::data::add_manual_course(
                        snapshot,
                        course.clone(),
                    ),
                }
            };
            if let Err(why) = added {
                rejection.set(Some(why));
                return;
            }
            let course_code = course.code.clone();
            manual.write().push(course);
            crate::browser::local_set(
                crate::persist::MANUAL_KEY,
                &crate::persist::encode_manual(&manual.read()),
            );
            let granted_rule =
                Some(rule.read().clone()).filter(|value| value != "-");
            // the rule the form attaches it to names the block it is
            // chosen under, exactly as a section row does
            let origin =
                granted_rule.as_deref().and_then(panel::section_origin);
            edit_plan(
                plan,
                history,
                &format!("Cours manuel {course_code} ajouté"),
                |plan| {
                    state::tag_elective_origin(plan, &course_code, origin);
                    if let Some(key) = granted_rule {
                        plan.rule_grants.insert(course_code.clone(), key);
                    }
                    // created and taken in one act; the strip freezes it
                    // to a session afterwards if the student wants
                    if !plan.electives.contains(&course_code) {
                        plan.electives.push(course_code);
                    }
                },
            );
            // the worker's catalogue must learn the new course too — and
            // the last proposal's fingerprint answered against the old
            // one, so it no longer counts
            let mut solver = solver;
            solver.write().proposed = None;
            super::cancel_search(
                &handle, solver, plan, alerts, manual, snapshot,
            );
            code.write().clear();
            title.write().clear();
            credits.write().clear();
            nrc.write().clear();
            slots.set(vec![crate::data::ManualSlot {
                day: "monday".to_string(),
                start: String::new(),
                end: String::new(),
            }]);
            rejection.set(None);
        }
    };
    let existing = manual.read().clone();
    rsx! {
        Disclosure {
            open: manual_open(),
            on_toggle: move |_| manual_open.set(!manual_open()),
            class: "panel-manual",
            summary: rsx! { "Cours absent du catalogue ?" },
            p { class: "panel-note",
                "Entrez-le à la main : il sera ajouté à la session \
                 affichée, marqué « manuel », et reste sur cet appareil."
            }
            div { class: "panel-manual-grid",
                input {
                    class: "panel-add-input",
                    placeholder: "Code (GEX-1234)",
                    aria_label: "Code du cours manuel",
                    value: "{code}",
                    // resuming the form retires its old refusal — a stale
                    // one stayed on screen all session (rapport 2026-08-14)
                    oninput: move |event| {
                        code.set(event.value());
                        rejection.set(None);
                    },
                }
                input {
                    class: "panel-add-input",
                    placeholder: "Titre",
                    aria_label: "Titre du cours manuel",
                    value: "{title}",
                    oninput: move |event| title.set(event.value()),
                }
                input {
                    class: "panel-add-input",
                    placeholder: "Crédits (3)",
                    aria_label: "Crédits du cours manuel",
                    value: "{credits}",
                    oninput: move |event| credits.set(event.value()),
                }
                input {
                    class: "panel-add-input",
                    placeholder: "NRC (optionnel)",
                    aria_label: "NRC du cours manuel",
                    value: "{nrc}",
                    oninput: move |event| nrc.set(event.value()),
                }
            }
            // the direction can admit the course in a rule (entente)
            {
                let read = snapshot.read();
                let rules = read
                    .as_ref()
                    .and_then(|snapshot| {
                        let plan_read = plan.read();
                        let program =
                            panel::chosen_program(snapshot, &plan_read)?;
                        let (concentration, profile) =
                            panel::scope_of(&plan_read);
                        Some(panel::grantable_rules(
                            program,
                            concentration,
                            profile,
                        ))
                    })
                    .unwrap_or_default();
                let chosen_rule = rule.read().clone();
                rsx! {
                    if !rules.is_empty() {
                        select {
                            class: "panel-attach",
                            aria_label: "Règle où compter ce cours (entente)",
                            onchange: move |event| rule.set(event.value()),
                            option {
                                value: "-",
                                selected: chosen_rule == "-",
                                "Ne compte dans aucune règle"
                            }
                            for (key, rule_title) in rules {
                                option {
                                    value: "{key}",
                                    selected: chosen_rule == key,
                                    "Compte dans « {rule_title} » (entente)"
                                }
                            }
                        }
                    }
                }
            }
            for (i, slot) in slots.read().iter().cloned().enumerate() {
                div { class: "panel-manual-slot", key: "{i}",
                    select {
                        aria_label: "Jour",
                        onchange: move |event| {
                            slots.write()[i].day = event.value();
                        },
                        for (value, label) in [
                            ("monday", "Lundi"),
                            ("tuesday", "Mardi"),
                            ("wednesday", "Mercredi"),
                            ("thursday", "Jeudi"),
                            ("friday", "Vendredi"),
                            ("saturday", "Samedi"),
                            ("sunday", "Dimanche"),
                        ] {
                            option {
                                value,
                                selected: slot.day == value,
                                "{label}"
                            }
                        }
                    }
                    input {
                        class: "panel-add-input",
                        placeholder: "8:30",
                        aria_label: "Heure de début",
                        value: "{slot.start}",
                        oninput: move |event| {
                            slots.write()[i].start = event.value();
                        },
                    }
                    input {
                        class: "panel-add-input",
                        placeholder: "11:20",
                        aria_label: "Heure de fin",
                        value: "{slot.end}",
                        oninput: move |event| {
                            slots.write()[i].end = event.value();
                        },
                    }
                    button {
                        class: "panel-course-action",
                        aria_label: "Retirer cette plage",
                        onclick: move |_| {
                            slots.write().remove(i);
                        },
                        "✕"
                    }
                }
            }
            div { class: "panel-add-row",
                button {
                    class: "panel-verify-button",
                    onclick: move |_| {
                        slots.write().push(crate::data::ManualSlot {
                            day: "monday".to_string(),
                            start: String::new(),
                            end: String::new(),
                        });
                    },
                    "+ plage"
                }
                button {
                    class: "panel-add-button",
                    onclick: submit,
                    "Créer le cours"
                }
            }
            if let Some(why) = rejection.read().as_ref() {
                p { class: "panel-add-error", role: "alert", "{why}" }
            }
            for course in existing {
                ManualCourseActions { course }
            }
        }
    }
}

// contribution path (ADR): a prefilled issue — plain anchor — plus a
// « Copier le JSON » fallback for a student without a GitHub account
#[component]
fn ManualCourseActions(course: ulaval_scheduler_core::Course) -> Element {
    let alerts = use_context::<Signal<super::AlertStack>>();
    // expect over `?`: serializing a Course provably cannot fail
    let json = serde_json::to_string_pretty(&course)
        .expect("Course serialization always succeeds");
    let issue = format!(
        "https://github.com/antoinelb/ulaval-generateur-horaire/issues/new?title={}&body={}",
        crate::browser::encode_uri(&format!(
            "Cours manuel : {}",
            course.code
        )),
        crate::browser::encode_uri(&format!(
            "Cours entré à la main, à considérer pour \
             `data/cours.manuel.json` :\n\n```json\n{json}\n```"
        )),
    );
    let copy_json = json.clone();
    rsx! {
        div { class: "panel-manual-existing",
            span { class: "panel-course-sub", "{course.code} - manuel" }
            a {
                class: "panel-chip",
                href: "{issue}",
                target: "_blank",
                rel: "noopener",
                title: "Ouvre une page GitHub prérendue — un compte \
                        GitHub est requis pour l'envoyer",
                "Proposer au catalogue (GitHub)"
            }
            button {
                class: "panel-chip",
                title: "Copie la fiche du cours, à coller dans un \
                        courriel si vous n'avez pas de compte GitHub",
                onclick: move |_| {
                    crate::browser::clipboard_write(&copy_json);
                    super::push_alert(
                        alerts,
                        super::AlertBody::Note(
                            "Fiche du cours copiée — collez-la dans un \
                             courriel ou une issue GitHub."
                                .to_string(),
                        ),
                    );
                },
                "Copier la fiche du cours"
            }
        }
    }
}
