use dioxus::prelude::*;

use super::{edit_plan, SelectedCourse};
use crate::data::Snapshot;
use crate::panel::{self, Badge, Fit, PanelModel, Row, RowState, Section};
use crate::solve;
use crate::state::{self, History, Plan, ProgramChoice, View};

// The single left panel (notes 2026-08-13 : plus d'onglets) : the
// program's rules and organigramme controls, the catalogue search, the
// add-by-code field and the manual-course form. The session's own courses
// live in the schedule and the ribbon — no list here. Everything shown
// comes from `crate::panel` (pure, tested); this file only wires clicks
// and signals.
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
            }
            PanelBody { model }
            ManualCourseForm {}
            AddByCode {}
        }
    }
}

// choosing the program is an ordinary, labelled, undoable edit — and the
// panel fills with its rules the moment it lands
#[component]
fn ProgramPicker() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    rsx! {
        div { class: "panel-picker",
            p { class: "panel-picker-lead",
                "Choisissez un programme pour voir ses règles :"
            }
            for program in snapshot.programs.iter() {
                button {
                    class: "panel-picker-item",
                    key: "{program.code}-{program.semester}",
                    onclick: {
                        let code = program.code.clone();
                        let semester = program.semester.to_string();
                        move |_| {
                            let code = code.clone();
                            let semester = semester.clone();
                            edit_plan(
                                plan,
                                history,
                                &format!("Programme {code} choisi"),
                                |plan| {
                                    plan.program = Some(ProgramChoice {
                                        code,
                                        semester,
                                        concentration: None,
                                        profile: None,
                                    });
                                },
                            );
                        }
                    },
                    div { class: "panel-picker-title", "{program.title}" }
                    div { class: "panel-picker-sub",
                        "{program.code} - version {program.semester} - "
                        "{program.credits_required} cr"
                    }
                }
            }
        }
    }
}

#[component]
fn PanelBody(model: PanelModel) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let search = view.read().search.clone();
    let searching = !search.trim().is_empty();
    let only_fitting = view.read().only_fitting;
    let has_program = plan.read().program.is_some();
    rsx! {
        div { class: "panel-body",
            input {
                class: "panel-search",
                r#type: "search",
                placeholder: "Chercher dans tout le catalogue…",
                aria_label: "Chercher dans tout le catalogue…",
                value: "{search}",
                oninput: move |event| view.write().search = event.value(),
            }
            label { class: "panel-fit",
                input {
                    r#type: "checkbox",
                    checked: only_fitting,
                    onchange: move |event| {
                        view.write().only_fitting = event.checked();
                    },
                }
                "Seulement les cours qui rentrent dans l'horaire affiché"
            }
            if let Some(error) = model.coverage_error.as_ref() {
                p { class: "warning", "⚠ {error}" }
            }
            for warning in model.warnings.iter() {
                p { class: "warning", "⚠ {warning}" }
            }
            if has_program {
                OrganigrammeControls { rules_missing: missing_rules(&model) }
            }
            if searching {
                SearchResults {}
            } else if has_program {
                if let Some(mandatory) = model.mandatory.clone() {
                    SectionView { section: mandatory }
                }
                for section in model.rules.iter().cloned() {
                    SectionView { section }
                }
                if let Some(note) = model.language_note.as_ref() {
                    p { class: "panel-note", b { "{note}" } }
                }
                for note in model.notes.iter() {
                    p { class: "panel-note", "{note}" }
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
        .mandatory
        .iter()
        .chain(model.rules.iter())
        .filter(|section| {
            matches!(section.badge, Badge::Missing(_) | Badge::Partial(_))
        })
        .count()
}

#[component]
fn OrganigrammeControls(rules_missing: usize) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let solver = use_context::<Signal<super::SolverState>>();
    let handle = use_context::<super::SolverHandle>();
    let start = plan.read().start.to_string();
    let study_sessions = plan.read().study_sessions;
    let credit_cap = plan.read().credit_cap;
    let summers_open = plan.read().summers_open;
    let concomitant = plan.read().concomitant;
    let ready = solver.read().ready;
    let busy = solver.read().running.is_some();
    let truncated = solver.read().truncated;
    let verification = solver.read().verification.clone();
    let request = {
        let handle = handle.clone();
        move |max_nodes: u64| {
            let read = snapshot.read();
            let plan_read = plan.read();
            let program = read.as_ref().and_then(|snapshot| {
                panel::effective_program(snapshot, &plan_read)
            });
            super::request_place(
                &handle,
                solver,
                &plan_read,
                program.as_ref(),
                max_nodes,
            );
        }
    };
    let request_full = request.clone();
    let request_quick = request;
    // the verification runs by itself (note 6) — this only explains why
    // it has not run yet: courses still floating, or an unreadable input
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
                        for year in 24..=31u16 {
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
                        r#type: "number",
                        min: "2",
                        max: "16",
                        value: "{study_sessions}",
                        onchange: move |event| {
                            if let Ok(count) = event.value().parse::<usize>()
                            {
                                let clamped = count.clamp(2, 16);
                                edit_plan(
                                    plan,
                                    history,
                                    &format!("Horizon à {clamped} sessions"),
                                    |plan| plan.study_sessions = clamped,
                                );
                            }
                        },
                    }
                }
                label { class: "panel-knob",
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
            label { class: "panel-fit",
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
            div { class: "panel-organigramme-actions",
                button {
                    class: "panel-solve-button",
                    disabled: !ready || busy,
                    title: if ready {
                        "Remplir les sessions automatiquement (annulable)"
                    } else {
                        "Le solveur démarre…"
                    },
                    onclick: move |_| {
                        request_quick(crate::solve::PROPOSE_MAX_NODES);
                    },
                    "Proposer un organigramme"
                }
                if truncated {
                    button {
                        class: "panel-verify-button",
                        disabled: !ready || busy,
                        onclick: move |_| {
                            request_full(crate::solve::FULL_MAX_NODES);
                        },
                        "Chercher plus longtemps"
                    }
                }
            }
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
            match readiness {
                Some(Err(why)) => rsx! {
                    p { class: "warning",
                        "⚠ Vérification impossible : {why}"
                    }
                },
                Some(Ok(unplaced)) if !unplaced.is_empty() => rsx! {
                    p { class: "panel-verdict",
                        "{unplaced.len()} cours sans session — proposez un \
                         organigramme ou placez-les ; la vérification se \
                         relancera d'elle-même."
                    }
                },
                _ => rsx! {},
            }
            if let Some(verification) = verification {
                if !verification.solutions.is_empty() {
                    if rules_missing == 0 {
                        p { class: "panel-verdict panel-verdict--ok",
                            "Cheminement vérifié ✓ — préalables, plafond, \
                             horaires et règles comptées : tout y est."
                        }
                    } else {
                        p { class: "panel-verdict panel-verdict--ok",
                            "Placement vérifié ✓ (préalables, plafond, une \
                             combinaison d'horaire possible par session)"
                        }
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

// one accordion: header button, expansion strictly in place (LAY-2)
#[component]
fn SectionView(section: Section) -> Element {
    let mut view = use_context::<Signal<View>>();
    let expanded =
        view.read().expanded_rule.as_deref() == Some(section.key.as_str());
    let key = section.key.clone();
    let (badge_class, badge_text) = match &section.badge {
        Badge::Ok(text) => ("panel-badge--ok", text.clone()),
        Badge::Partial(text) => ("panel-badge--partial", text.clone()),
        Badge::Missing(text) => ("panel-badge--missing", text.clone()),
        Badge::Neutral(text) => ("panel-badge--neutral", text.clone()),
    };
    let chevron = if expanded { "▾" } else { "▸" };
    rsx! {
        div {
            class: "panel-rule",
            class: if expanded { "panel-rule--open" },
            class: if matches!(section.badge, Badge::Missing(_)) { "panel-rule--missing" },
            button {
                class: "panel-rule-head",
                aria_expanded: expanded,
                onclick: move |_| {
                    let mut view = view.write();
                    view.expanded_rule = if expanded {
                        None
                    } else {
                        Some(key.clone())
                    };
                },
                span { class: "panel-rule-title",
                    "{section.title}"
                    if let Some(constraint) = section.constraint.as_ref() {
                        span { class: "panel-rule-constraint",
                            " - {constraint}"
                        }
                    }
                }
                if let Some((done, total)) = section.progress {
                    if total > 0 {
                        span {
                            class: "panel-progress",
                            role: "img",
                            aria_label: "{done} sur {total}",
                            span {
                                class: "panel-progress-fill",
                                style: "width:{done * 100 / total}%;",
                            }
                        }
                    }
                }
                span { class: "panel-badge {badge_class}", "{badge_text}" }
                span { class: "panel-rule-chevron", "{chevron}" }
            }
            if expanded {
                div { class: "panel-rule-content",
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
                    if section.free {
                        FreeBrowse {}
                    } else {
                        for row in section.rows.iter().cloned() {
                            RowView { row }
                        }
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
#[component]
fn FreeBrowse() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let mut view = use_context::<Signal<View>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let read = snapshot.read();
    let Some(snapshot) = read.as_ref() else {
        return rsx! {};
    };
    let subjects = panel::subjects(snapshot);
    let subject = view.read().subject.clone();
    let only_fitting = view.read().only_fitting;
    let results = panel::search_courses(
        snapshot,
        &plan.read(),
        panel::SearchScope {
            // the fit filter judges against the displayed session
            session: only_fitting.then(|| view.read().session),
            subject: subject.as_deref(),
            first_cycle_only: true,
            only_fitting,
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
            ResultRows { results }
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
    let only_fitting = view.read().only_fitting;
    let results = panel::search_courses(
        snapshot,
        &plan.read(),
        panel::SearchScope {
            // the fit filter judges against the displayed session
            session: only_fitting.then(|| view.read().session),
            subject: None,
            first_cycle_only: false,
            only_fitting,
        },
        &view.read().search,
    );
    rsx! {
        ResultRows { results }
    }
}

#[component]
fn ResultRows(results: panel::SearchResults) -> Element {
    rsx! {
        div { class: "panel-results",
            for row in results.rows.iter().cloned() {
                RowView { row }
            }
            p { class: "panel-results-count",
                if results.matched == 0 {
                    "Aucun cours ne correspond — vérifiez le sigle ou le filtre."
                } else if results.matched > results.rows.len() {
                    "{results.rows.len()} premiers affichés sur {results.matched} correspondances"
                } else {
                    "{results.matched} cours"
                }
                if results.masked_by_fit > 0 {
                    " - {results.masked_by_fit} masqués par le filtre horaire"
                }
            }
        }
    }
}

// one course row: state carried by text and shape, the + adds to the
// displayed session, the chips place (or move) it anywhere admissible,
// the ✕ removes it — immediate and undoable, never a dialog (ACT-2)
#[component]
fn RowView(row: Row) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let SelectedCourse(mut selected) = use_context::<SelectedCourse>();
    let session = view.read().session;
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
    let alerts = use_context::<Signal<Vec<super::Alert>>>();
    let fit_note = match fit {
        Some(Fit::Fits) => " - rentrerait ✓",
        Some(Fit::Conflicts) => " - l'ajouter ici créerait un conflit",
        _ => "",
    };
    // unmet prerequisites warn, they never wall (the student may know
    // better — an entente, un cours fait ailleurs) ; the + validates
    let addable =
        matches!(row.state, RowState::Available | RowState::PrereqUnmet);
    let dimmed =
        matches!(row.state, RowState::PrereqUnmet | RowState::Unknown)
            || matches!(fit, Some(Fit::Conflicts));
    let assumed = row.assumed.join(", ");
    rsx! {
        div {
            class: "panel-course",
            class: if dimmed { "panel-course--dimmed" },
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
            }
            // an acquired préparatoire row offers nothing — granting it
            // would resurrect it as ordinary work while the box says done
            if !matches!(row.state, RowState::Unknown | RowState::Acquired) {
                RuleAttach { code: row.code.clone() }
            }
            if addable {
                button {
                    class: "panel-course-action",
                    title: "Ajouter {row.code} à la session affichée",
                    onclick: {
                        let code = row.code.clone();
                        move |_| {
                            let code = code.clone();
                            // same gate as the typed entry (saison,
                            // doublon, préalables) — the + must not be a
                            // silent side door (rapport étudiante)
                            let verdict = {
                                let read = snapshot.read();
                                let plan_read = plan.read();
                                read.as_ref().map(|snapshot| {
                                    solve::validate_new_code(
                                        snapshot, &plan_read, session, &code,
                                    )
                                })
                            };
                            match verdict {
                                Some(Ok(accepted)) => {
                                    edit_plan(
                                        plan,
                                        history,
                                        &format!("Ajout de {code}"),
                                        |plan| {
                                            plan.manual
                                                .entry(session)
                                                .or_default()
                                                .push(accepted.code)
                                        },
                                    );
                                    if let Some(warning) = accepted.warning {
                                        super::push_alert(
                                            alerts,
                                            super::AlertBody::Note(warning),
                                        );
                                    }
                                }
                                Some(Err(why)) => super::push_alert(
                                    alerts,
                                    super::AlertBody::Note(why),
                                ),
                                None => {}
                            }
                        }
                    },
                    "+"
                }
                SessionChips { code: row.code.clone() }
            }
            if row.state == RowState::Placed {
                // an already-placed course (obligatoire compris) moves
                // through the same chips, and can be removed outright
                SessionChips { code: row.code.clone() }
                button {
                    class: "panel-course-action",
                    title: "Retirer {row.code} de son organigramme",
                    onclick: {
                        let code = row.code.clone();
                        move |_| {
                            let code = code.clone();
                            // the read borrow must die before edit_plan
                            let held = holding_session(&plan.read(), &code);
                            let Some(held) = held else {
                                return;
                            };
                            edit_plan(
                                plan,
                                history,
                                &format!("{code} retiré"),
                                |plan| remove_course(plan, held, &code),
                            );
                            selected.set(None);
                        }
                    },
                    "✕"
                }
            }
            if row.state == RowState::Chosen {
                button {
                    class: "panel-course-action",
                    title: "Retirer {row.code} des cours choisis",
                    onclick: {
                        let code = row.code.clone();
                        move |_| {
                            let code = code.clone();
                            edit_plan(
                                plan,
                                history,
                                &format!("{code} retiré des choisis"),
                                |plan| {
                                    plan.electives
                                        .retain(|kept| kept != &code)
                                },
                            );
                        }
                    },
                    "✕"
                }
            }
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
        .and_then(|snapshot| panel::chosen_program(snapshot, &plan.read()))
        .map(panel::grantable_rules)
        .unwrap_or_default();
    if rules.is_empty() {
        return rsx! {};
    }
    let current = plan.read().rule_grants.get(&code).cloned();
    rsx! {
        select {
            class: "panel-attach",
            aria_label: "Rattacher {code} à une règle (entente avec la direction)",
            title: "Entente : compter {code} dans une règle",
            onchange: {
                let code = code.clone();
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
                    let title = value
                        .split_once('/')
                        .map(|(_, title)| title.to_string())
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
                "entente…"
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

// The « + H28 » chips (design 9a): which sessions could host the course if
// pinned there — answered by the worker (`core::admissible_sessions`), on
// demand, cached until the plan changes. Clicking a chip is the pin the
// probe simulated (immediate + undoable); on an already-placed course the
// same chip is a move.
#[component]
fn SessionChips(code: String) -> Element {
    let plan = use_context::<Signal<Plan>>();
    let history = use_context::<Signal<History>>();
    let solver = use_context::<Signal<super::SolverState>>();
    let handle = use_context::<super::SolverHandle>();
    let cached = solver.read().admissible.get(&code).cloned();
    let ready = solver.read().ready;
    match cached {
        None => rsx! {
            button {
                class: "panel-chip panel-chip--ask",
                disabled: !ready,
                title: "Chercher les sessions qui peuvent accueillir {code}",
                onclick: {
                    let code = code.clone();
                    let handle = handle.clone();
                    move |_| {
                        let plan_read = plan.read();
                        super::request_admissible(
                            &handle, solver, &plan_read, &code,
                        );
                    }
                },
                "où le placer ?"
            }
        },
        Some(sessions) if sessions.is_empty() => rsx! {
            span { class: "panel-chip-none",
                "aucune session ne peut l'accueillir"
            }
        },
        Some(sessions) => {
            let (labels, held) = {
                let plan = plan.read();
                let seasons = ulaval_scheduler_core::horizon_sessions(
                    plan.start.season,
                    plan.study_sessions,
                );
                let semesters = state::session_semesters(plan.start, &seasons);
                let labels: Vec<(usize, String)> = sessions
                    .iter()
                    .map(|&session| {
                        (
                            session,
                            state::session_label(&semesters, session - 1),
                        )
                    })
                    .collect();
                (labels, holding_session(&plan, &code))
            };
            rsx! {
                span { class: "panel-chips",
                    for (session, label) in labels {
                        if Some(session) == held {
                            // its current home is not a destination — a
                            // « + » here promised an ajout qui n'en est
                            // pas un (rapport étudiante)
                            span { class: "panel-chip panel-chip--here",
                                "ici : {label}"
                            }
                        } else {
                            button {
                                class: "panel-chip",
                                title: "Placer {code} en {label}",
                                onclick: {
                                    let code = code.clone();
                                    let label = label.clone();
                                    move |_| {
                                        place_course(
                                            plan, history, &code, session,
                                            &label,
                                        );
                                    }
                                },
                                "+ {label}"
                            }
                        }
                    }
                    button {
                        class: "panel-chip panel-chip--ask",
                        title: "Refermer les sessions proposées",
                        onclick: {
                            let code = code.clone();
                            move |_| {
                                let mut solver = solver;
                                solver.write().admissible.remove(&code);
                            }
                        },
                        "✕"
                    }
                }
            }
        }
    }
}

// Shared by the chips and the ribbon drop (note 16): place — or move —
// `code` into `session`, any previous location cleared first; one
// labelled, undoable step.
pub fn place_course(
    plan: Signal<Plan>,
    history: Signal<History>,
    code: &str,
    session: usize,
    label: &str,
) {
    // the read borrow must die before edit_plan opens the write
    let held = holding_session(&plan.read(), code);
    let action = if held.is_some() {
        format!("{code} déplacé vers {label}")
    } else {
        format!("{code} épinglé en {label}")
    };
    let code = code.to_string();
    edit_plan(plan, history, &action, |plan| {
        if let Some(held) = held {
            remove_course(plan, held, &code);
        }
        // placing a catalogue course IS choosing it: without the elective
        // the solver would see a pin with no Course behind it (rapport
        // étudiante : « MED-1100 is passed or pinned but has no Course »)
        if !plan.electives.contains(&code) {
            plan.electives.push(code.clone());
        }
        plan.pinned_sessions.insert(code.clone(), session);
        plan.displayed_placement.insert(code, session);
    });
}

// removal touches every trace of the course in that session — manual
// list, elective, placement, pin and chosen sections — one labelled,
// undoable step (a leftover elective would float back at the next solve)
fn remove_course(plan: &mut Plan, session: usize, code: &str) {
    if let Some(manual) = plan.manual.get_mut(&session) {
        manual.retain(|kept| kept != code);
    }
    if plan.displayed_placement.get(code) == Some(&session) {
        plan.displayed_placement.remove(code);
        plan.pinned_sessions.remove(code);
        plan.electives.retain(|kept| kept != code);
    }
    if let Some(chosen) = plan.chosen.get_mut(&session) {
        chosen.remove(code);
    }
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
    let alerts = use_context::<Signal<Vec<super::Alert>>>();
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
            edit_plan(
                plan,
                history,
                &format!("Cours manuel {course_code} ajouté"),
                |plan| {
                    if let Some(key) = granted_rule {
                        plan.rule_grants.insert(course_code.clone(), key);
                    }
                    plan.manual.entry(session).or_default().push(course_code)
                },
            );
            // the worker's catalogue must learn the new course too
            super::cancel_search(
                &handle, solver, plan, history, alerts, manual,
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
        details { class: "panel-manual",
            summary { "Cours absent du catalogue ?" }
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
                        panel::chosen_program(snapshot, &plan.read())
                    })
                    .map(panel::grantable_rules)
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
    let alerts = use_context::<Signal<Vec<super::Alert>>>();
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

// INP-7: validated on commit, inline, non-blocking — and the field is
// never cleared on a rejection (ERR-6)
#[component]
fn AddByCode() -> Element {
    let plan = use_context::<Signal<Plan>>();
    let view = use_context::<Signal<View>>();
    let history = use_context::<Signal<History>>();
    let snapshot = use_context::<Signal<Option<Snapshot>>>();
    let alerts = use_context::<Signal<Vec<super::Alert>>>();
    let mut typed = use_signal(String::new);
    let mut rejection = use_signal(|| None::<String>);
    let mut submit = move || {
        let read = snapshot.read();
        let Some(snapshot) = read.as_ref() else {
            return;
        };
        let session = view.read().session;
        let raw = typed.read().clone();
        // the read borrow must die before `edit_plan` takes the write one
        let verdict = {
            let plan = plan.read();
            solve::validate_new_code(snapshot, &plan, session, &raw)
        };
        match verdict {
            Ok(accepted) => {
                let code = accepted.code;
                edit_plan(
                    plan,
                    history,
                    &format!("Ajout de {code}"),
                    |plan| plan.manual.entry(session).or_default().push(code),
                );
                if let Some(warning) = accepted.warning {
                    super::push_alert(alerts, super::AlertBody::Note(warning));
                }
                typed.write().clear();
                rejection.set(None);
            }
            Err(why) => rejection.set(Some(why)),
        }
    };
    rsx! {
        div { class: "panel-add",
            div { class: "panel-add-row",
                input {
                    class: "panel-add-input",
                    r#type: "text",
                    placeholder: "Ajouter par code…",
                    aria_label: "Code du cours à ajouter",
                    value: "{typed}",
                    oninput: move |event| {
                        typed.set(event.value());
                        // a fresh keystroke clears the old verdict, the
                        // next commit re-judges (reject on commit, INP-7)
                        rejection.set(None);
                    },
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            submit();
                        }
                    },
                }
                button {
                    class: "panel-add-button",
                    onclick: move |_| submit(),
                    "Ajouter"
                }
            }
            if let Some(why) = rejection.read().as_ref() {
                p { class: "panel-add-error", role: "alert", "{why}" }
            }
        }
    }
}
