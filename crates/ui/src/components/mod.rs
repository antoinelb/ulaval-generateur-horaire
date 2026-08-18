// The view layer — wasm32-only, mechanical, no business rule: every value
// it renders comes from `core`, `ui-calculations` or the pure modules.

pub mod grid;
pub mod header;
pub mod panel;
pub mod ribbon;
pub mod shell;

use std::rc::Rc;

use dioxus::prelude::*;

use crate::data::Snapshot;
use crate::persist;
use crate::present::{present_data_error, UiError};
use crate::state::{self, History, Plan, View};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

// The one loading gate: the app renders nothing data-bearing before the
// snapshot is whole — and says so explicitly (LAT-5: no skeleton).
#[derive(Clone, Debug, PartialEq)]
pub enum LoadState {
    Downloading,
    Parsing,
    Ready,
    Failed(UiError),
}

// The persistent status region's content (ALR-4: persists until
// dismissed; ALR-6: a reserved region, never a modal).
#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub key: u64,
    pub body: AlertBody,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlertBody {
    Note(String),
    // a confirmation, styled apart from the warnings (INP-3: the glyph
    // carries the difference, not the colour alone)
    Success(String),
    Error(UiError),
}

// the course whose alternatives (ghosts) the grid currently shows —
// selected by click or keyboard, never by hover alone (INP-5)
#[derive(Clone, Copy, PartialEq)]
pub struct SelectedCourse(pub Signal<Option<String>>);

// the course a grid block is currently dragging toward the ribbon (note
// 16) — the payload rides in a signal, never in DataTransfer; the chips
// stay the keyboard-reachable equivalent (INP-4)
#[derive(Clone, Copy, PartialEq)]
pub struct DraggedCourse(pub Signal<Option<String>>);

// the student's hand-entered Courses, persisted apart from the plan
#[derive(Clone, Copy, PartialEq)]
pub struct ManualCourses(pub Signal<Vec<ulaval_scheduler_core::Course>>);

// --- solver B, off the main thread ----------------------------------------

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SolverState {
    pub ready: bool,
    // the one blocking query (proposition/vérification); admissible chip
    // probes are fire-and-forget
    pub running: Option<Running>,
    // the last verify answer, shown until the plan changes
    pub verification: Option<crate::solve::PlacementAnswer>,
    // the last automatic verify errored: do not refire until the plan
    // changes, or the effect would loop on the same failure
    pub verify_failed: bool,
    // code → sessions that could host it (the « + H28 » chips)
    pub admissible:
        std::collections::BTreeMap<String, std::collections::BTreeSet<usize>>,
    // chip probes in flight (id → code): their errors must surface too —
    // a mute button is a broken button
    pending_probes: std::collections::BTreeMap<u64, String>,
    // the last proposal hit the node budget: offer « chercher plus
    // longtemps »
    pub truncated: bool,
    // the last proposal was a best-effort filling and these are the courses
    // it could not seat — so the panel stops telling the student to propose
    // an organigramme he has just proposed (ADR
    // `2026-08-placement-au-mieux-en-repli`)
    pub left_out: std::collections::BTreeSet<String>,
    next_id: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Running {
    pub id: u64,
    pub kind: QueryKind,
    pub started_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryKind {
    Propose,
    Verify,
}

// the worker handle: one live worker, replaced whole on cancel
#[derive(Clone)]
pub struct SolverHandle(
    pub Rc<std::cell::RefCell<Option<crate::browser::Solver>>>,
);

impl PartialEq for SolverHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub fn boot_solver(
    handle: &SolverHandle,
    mut state: Signal<SolverState>,
    plan: Signal<Plan>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    // the worker's catalogue must hold exactly what the panel's does: the
    // repo's hand-maintained courses, the student's own, and the
    // prerequisites his program vintage rewrote
    let (shared, overrides) = match snapshot.peek().as_ref() {
        Some(snapshot_ref) => (
            snapshot_ref.shared_manual.clone(),
            crate::data::effective_overrides(snapshot_ref, &plan.peek()),
        ),
        None => (Vec::new(), std::collections::BTreeMap::new()),
    };
    let courses: Vec<ulaval_scheduler_core::Course> = shared
        .into_iter()
        .chain(manual.peek().iter().cloned())
        .collect();
    // expect over `?`: serializing Courses and corrections provably cannot
    // fail
    let manual_json = serde_json::to_string(&courses)
        .expect("Course serialization always succeeds");
    let overrides_json = serde_json::to_string(&overrides)
        .expect("Override serialization always succeeds");
    let solver = crate::browser::spawn_solver(
        &manual_json,
        &overrides_json,
        move |text| {
            handle_worker_answer(&text, state, plan, history, alerts);
        },
    );
    if solver.is_none() {
        push_alert(
            alerts,
            AlertBody::Note(
                "Le solveur d'organigramme n'a pas pu démarrer — la \
                 proposition automatique est indisponible; le reste de \
                 l'application fonctionne."
                    .to_string(),
            ),
        );
    } else {
        state.write().ready = false;
    }
    *handle.0.borrow_mut() = solver;
}

fn handle_worker_answer(
    text: &str,
    mut state: Signal<SolverState>,
    plan: Signal<Plan>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
) {
    match crate::solve::parse_worker_answer(text) {
        Err(message) => push_alert(alerts, AlertBody::Note(message)),
        Ok(crate::solve::WorkerAnswer::Ready { .. }) => {
            state.write().ready = true;
        }
        Ok(crate::solve::WorkerAnswer::Admissible { id, code, sessions }) => {
            let mut state = state.write();
            state.pending_probes.remove(&id);
            state.admissible.insert(code, sessions);
        }
        Ok(crate::solve::WorkerAnswer::Error { id, message }) => {
            if let Some(code) = state.write().pending_probes.remove(&id) {
                push_alert(
                    alerts,
                    AlertBody::Note(format!(
                        "Sessions admissibles de {code} : {message}"
                    )),
                );
                return;
            }
            let running = state.read().running;
            let matches_running =
                running.is_some_and(|running| running.id == id);
            if matches_running || id == 0 {
                let mut state = state.write();
                state.running = None;
                if running
                    .is_some_and(|running| running.kind == QueryKind::Verify)
                {
                    state.verify_failed = true;
                }
                drop(state);
                push_alert(
                    alerts,
                    AlertBody::Note(format!(
                        "Le solveur n'a pas pu répondre — détail \
                         technique : {message}"
                    )),
                );
            }
        }
        Ok(crate::solve::WorkerAnswer::Report { id, report }) => {
            let running = state.read().running;
            let Some(running) = running.filter(|running| running.id == id)
            else {
                // a superseded answer: the student cancelled it — dropped
                return;
            };
            state.write().running = None;
            match running.kind {
                QueryKind::Propose => {
                    apply_proposal(&report, state, plan, history, alerts);
                }
                QueryKind::Verify => {
                    state.write().verification =
                        Some(report.placement.clone());
                }
            }
        }
    }
}

// the proposal is applied through the normal undoable door — the solver
// never owns state, and everything it presumed or set aside is said
fn apply_proposal(
    report: &crate::solve::PlacementReport,
    mut state: Signal<SolverState>,
    plan: Signal<Plan>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
) {
    state.write().truncated =
        report.placement.completion.as_str() == "node-budget";
    if let Some(note) = crate::solve::completion_note(&report.placement) {
        push_alert(alerts, AlertBody::Note(note));
    }
    // a best-effort answer words every culprit itself, blocked ones
    // included — the per-blocked loop would say each of them twice
    match crate::solve::left_out_note(&report.placement) {
        Some(note) => push_alert(alerts, AlertBody::Note(note)),
        None => {
            for blocked in &report.placement.blocked {
                push_alert(
                    alerts,
                    AlertBody::Note(crate::solve::blocked_note(blocked)),
                );
            }
        }
    }
    for code in &report.set_aside {
        push_alert(
            alerts,
            AlertBody::Note(format!(
                "{code} : au programme mais absent du catalogue — mis de \
                 côté, à suivre à la main."
            )),
        );
    }
    let Some(solution) = report.placement.solutions.first() else {
        return;
    };
    state.write().left_out = solution.left_out.clone();
    if !solution.assumed.is_empty() {
        let assumed: Vec<&str> =
            solution.assumed.iter().map(String::as_str).collect();
        push_alert(
            alerts,
            AlertBody::Note(format!(
                "Le cheminement présume ces acquis (préalables que \
                 l'outil ne peut pas vérifier) : {}. Assurez-vous que \
                 cela vous décrit.",
                assumed.join(", ")
            )),
        );
    }
    if !report.injected.is_empty() {
        let injected: Vec<&str> =
            report.injected.iter().map(String::as_str).collect();
        let added = if injected.len() == 1 {
            "ajouté aux cours à option : un cours obligatoire l'exige \
             comme préalable"
        } else {
            "ajoutés aux cours à option : des cours obligatoires les \
             exigent comme préalables"
        };
        push_alert(
            alerts,
            AlertBody::Note(format!("{} {added}.", injected.join(", "))),
        );
    }
    let placement = solution.placement.clone();
    let injected = report.injected.clone();
    edit_plan(plan, history, "Organigramme proposé appliqué", |plan| {
        plan.displayed_placement = placement;
        // the plan must hold what the grid shows: an injected course left
        // out of `electives` would vanish from the very next request (the
        // preparatory-purge lesson)
        for code in injected {
            if !plan.electives.contains(&code) {
                plan.electives.push(code);
            }
        }
    });
}

// --- sending queries -------------------------------------------------------

pub fn request_place(
    handle: &SolverHandle,
    mut state: Signal<SolverState>,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
    max_nodes: u64,
) {
    let id = next_query(&mut state, QueryKind::Propose);
    send(
        handle,
        &crate::solve::place_request(id, plan, program, max_nodes),
    );
}

pub fn request_verify(
    handle: &SolverHandle,
    mut state: Signal<SolverState>,
    plan: &Plan,
    program: Option<&ulaval_scheduler_core::Program>,
) {
    let id = next_query(&mut state, QueryKind::Verify);
    send(handle, &crate::solve::verify_request(id, plan, program));
}

pub fn request_admissible(
    handle: &SolverHandle,
    mut state: Signal<SolverState>,
    plan: &Plan,
    code: &str,
) {
    // fire-and-forget: cached on answer, no running marker — but tracked,
    // so a probe that errors says so instead of leaving a mute button
    let id = {
        let mut state = state.write();
        state.next_id += 1;
        let id = state.next_id;
        state.pending_probes.insert(id, code.to_string());
        id
    };
    send(handle, &crate::solve::admissible_request(id, plan, code));
}

fn next_query(state: &mut Signal<SolverState>, kind: QueryKind) -> u64 {
    let mut state = state.write();
    state.next_id += 1;
    state.running = Some(Running {
        id: state.next_id,
        kind,
        started_ms: crate::browser::now_epoch_ms(),
    });
    state.next_id
}

fn send(handle: &SolverHandle, request: &str) {
    if let Some(solver) = handle.0.borrow().as_ref() {
        solver.send(request);
    }
}

// LAT-4's cancel is real: the crunching worker dies and a fresh one boots
pub fn cancel_search(
    handle: &SolverHandle,
    mut state: Signal<SolverState>,
    plan: Signal<Plan>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    if let Some(solver) = handle.0.borrow_mut().take() {
        solver.terminate();
    }
    state.write().running = None;
    boot_solver(handle, state, plan, history, alerts, manual, snapshot);
}

#[component]
pub fn App() -> Element {
    // restore once, before anything renders: the student lands exactly
    // where he was (ACT-7), and anything tolerated becomes an alert
    let restored = use_hook(|| Rc::new(restore_state()));
    let plan = use_signal(|| restored.plan.clone());
    let view = use_signal(|| restored.view.clone());
    let history = use_signal(History::default);
    let alerts = use_signal(|| seed_alerts(&restored.notes));
    let snapshot = use_signal(|| None::<Snapshot>);
    let load_state = use_signal(|| LoadState::Downloading);
    let solver_state = use_signal(SolverState::default);
    let manual = use_signal(|| restored.manual.clone());
    use_context_provider(|| ManualCourses(manual));
    use_context_provider(|| plan);
    use_context_provider(|| view);
    use_context_provider(|| history);
    use_context_provider(|| alerts);
    use_context_provider(|| snapshot);
    use_context_provider(|| load_state);
    use_context_provider(|| solver_state);
    // a shared organigramme lands before anything boots: its manual
    // courses must already sit in `manual` when the worker and the
    // catalogue parse read it
    use_hook(|| import_organigramme(plan, view, history, alerts, manual));
    let handle = use_hook(|| {
        let handle = SolverHandle(Rc::new(std::cell::RefCell::new(None)));
        boot_solver(
            &handle,
            solver_state,
            plan,
            history,
            alerts,
            manual,
            snapshot,
        );
        crate::browser::register_service_worker();
        handle
    });
    use_context_provider(|| handle.clone());
    use_future(move || load(snapshot, load_state, alerts, manual));
    save_on_change(plan, view);
    // the debounce's last ~300 ms: flush when the page goes away, so a
    // reload right after an edit never loses it (rapport 2026-08-14)
    use_hook(|| {
        crate::browser::on_page_hide(move || {
            crate::browser::local_set(
                persist::PLAN_KEY,
                &persist::encode_plan(&plan.peek()),
            );
            crate::browser::local_set(
                persist::VIEW_KEY,
                &persist::encode_view(&view.peek()),
            );
        });
    });
    // a plan change stales the verify answer and the admissible chips
    use_effect(move || {
        let _ = plan.read();
        let mut solver_state = solver_state;
        let mut state = solver_state.write();
        state.verification = None;
        state.verify_failed = false;
        state.admissible.clear();
        state.left_out.clear();
    });
    apply_corrections(
        plan,
        snapshot,
        alerts,
        handle.clone(),
        solver_state,
        history,
        manual,
    );
    heal_preparatory(plan, snapshot, alerts);
    auto_verify(plan, snapshot, solver_state, handle.clone());
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        shell::Screen {}
    }
}

struct RestoredState {
    plan: Plan,
    view: View,
    manual: Vec<ulaval_scheduler_core::Course>,
    notes: Vec<String>,
}

fn restore_state() -> RestoredState {
    let plan = persist::restore_plan(
        crate::browser::local_get(persist::PLAN_KEY).as_deref(),
    );
    let view = persist::restore_view(
        crate::browser::local_get(persist::VIEW_KEY).as_deref(),
    );
    let manual = persist::restore_manual(
        crate::browser::local_get(persist::MANUAL_KEY).as_deref(),
    );
    // stash what would otherwise be overwritten by the next save
    for backup in [&plan.backup, &view.backup, &manual.backup]
        .into_iter()
        .flatten()
    {
        crate::browser::stash_backup(backup);
    }
    let mut notes = plan.notes;
    notes.extend(view.notes);
    notes.extend(manual.notes);
    RestoredState {
        plan: plan.state,
        view: view.state,
        manual: manual.state,
        notes,
    }
}

// Keys are unique for the whole page life, never recycled: the Toasts
// auto-dismiss timers hold keys past an alert's death, and a recycled key
// would let a stale timer kill an unrelated fresh message.
thread_local! {
    static NEXT_ALERT_KEY: std::cell::Cell<u64> =
        const { std::cell::Cell::new(0) };
}

fn next_alert_key() -> u64 {
    NEXT_ALERT_KEY.with(|next| {
        let key = next.get();
        next.set(key + 1);
        key
    })
}

fn seed_alerts(notes: &[String]) -> Vec<Alert> {
    notes
        .iter()
        .map(|note| Alert {
            key: next_alert_key(),
            body: AlertBody::Note(note.clone()),
        })
        .collect()
}

pub fn push_alert(mut alerts: Signal<Vec<Alert>>, body: AlertBody) {
    let mut list = alerts.write();
    // never the same message twice (ALR-3) — but a repeat is refreshed to
    // the front instead of swallowed: relaunching a search that ends on
    // the same verdict must still visibly answer (rapport 2026-08-14)
    list.retain(|alert| alert.body != body);
    list.push(Alert {
        key: next_alert_key(),
        body,
    });
}

// ACT-2: the one door every Plan mutation walks through — labelled,
// reversible, no confirmation dialog anywhere
pub fn edit_plan(
    mut plan: Signal<Plan>,
    mut history: Signal<History>,
    label: &str,
    edit: impl FnOnce(&mut Plan),
) {
    let mut plan = plan.write();
    let mut history = history.write();
    state::apply(&mut plan, &mut history, label, edit);
}

// A `#…` address imports a whole shared organigramme (note 9): the plan
// replaces the student's — one undoable step gives theirs back — and the
// link's manual courses join the local list *before* the catalogue parse
// and the worker boot, so the recipient does strictly nothing.
fn import_organigramme(
    plan: Signal<Plan>,
    mut view: Signal<View>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
) {
    let Some(payload) = crate::browser::location_share() else {
        return;
    };
    match persist::decode_organigramme(&payload) {
        Err(error) => push_alert(
            alerts,
            AlertBody::Note(format!(
                "Lien de partage illisible ({error}) — rien n'a été importé."
            )),
        ),
        Ok((shared, courses)) => {
            let mut manual = manual;
            {
                let mut list = manual.write();
                for course in courses {
                    // an already-known code: the local copy wins
                    if list.iter().all(|held| held.code != course.code) {
                        list.push(course);
                    }
                }
            }
            crate::browser::local_set(
                persist::MANUAL_KEY,
                &persist::encode_manual(&manual.read()),
            );
            edit_plan(
                plan,
                history,
                "Organigramme partagé importé",
                |plan| {
                    *plan = shared;
                },
            );
            view.write().session = 1;
            crate::browser::strip_query();
            push_alert(
                alerts,
                AlertBody::Success(
                    "Organigramme partagé importé — « Annuler » restaure \
                     le vôtre."
                        .to_string(),
                ),
            );
        }
    }
}

// Note 6 (2026-08-13): verification is not a button — it re-runs by
// itself, debounced, whenever the plan settles with a program chosen and
// every requested course placed. The generation counter keeps a burst of
// edits down to one query; the guards make the effect converge (a fired
// query sets `running`, its answer sets `verification`, both stop it).
// The checked « scolarité préparatoire » box is an invariant, not a
// request filter: no préparatoire code may occupy a session. Whatever
// slipped one in (an old save, a shared link, an act done while the box
// was unchecked) is purged here — loudly, never silently. A direct write,
// not `edit_plan`: a derived correction is no student act; undoing the
// checkbox itself restores the pre-toggle plan whole, placements included.
fn heal_preparatory(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    alerts: Signal<Vec<Alert>>,
) {
    use_effect(move || {
        // materialize before any write: the read borrows must die first
        let leftovers = {
            let read = snapshot.read();
            let Some(snapshot_ref) = read.as_ref() else {
                return;
            };
            let plan_read = plan.read();
            let Some(program) =
                crate::panel::effective_program(snapshot_ref, &plan_read)
            else {
                return;
            };
            crate::solve::preparatory_leftovers(&plan_read, &program)
        };
        if leftovers.is_empty() {
            return;
        }
        let mut plan = plan;
        crate::state::purge_codes(&mut plan.write(), &leftovers);
        let note = if leftovers.len() == 1 {
            format!(
                "{} retiré des sessions : la scolarité préparatoire est \
                 marquée « déjà faite ». Décochez la case pour le replacer.",
                leftovers[0]
            )
        } else {
            format!(
                "{} retirés des sessions : la scolarité préparatoire est \
                 marquée « déjà faite ». Décochez la case pour les replacer.",
                leftovers.join(", ")
            )
        };
        push_alert(alerts, AlertBody::Note(note));
    });
}

// The corrections in force follow the plan — the student's admission
// vintage and his own edits — so the catalogue is rewritten here and not at
// parse time, when no program is picked yet. Same shape as
// `heal_preparatory`: derived state, a direct write (no student act to
// undo), and a guard that makes it converge — once applied, `applied`
// matches and the next run returns.
fn apply_corrections(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    alerts: Signal<Vec<Alert>>,
    handle: SolverHandle,
    solver_state: Signal<SolverState>,
    history: Signal<History>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
) {
    use_effect(move || {
        // materialize before any write: the read borrows must die first
        let overrides = {
            let read = snapshot.read();
            let Some(snapshot_ref) = read.as_ref() else {
                return;
            };
            let overrides =
                crate::data::effective_overrides(snapshot_ref, &plan.read());
            if snapshot_ref.applied == overrides {
                return;
            }
            overrides
        };
        let mut snapshot = snapshot;
        let notes = {
            let mut write = snapshot.write();
            let Some(snapshot_mut) = write.as_mut() else {
                return;
            };
            crate::data::set_prereq_overrides(snapshot_mut, &overrides)
        };
        for note in &notes {
            push_alert(
                alerts,
                AlertBody::Note(crate::present::present_override_note(note)),
            );
        }
        // the worker keeps its own copy of the catalogue: only a fresh one
        // learns the corrections
        cancel_search(
            &handle,
            solver_state,
            plan,
            history,
            alerts,
            manual,
            snapshot,
        );
    });
}

fn auto_verify(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    solver_state: Signal<SolverState>,
    handle: SolverHandle,
) {
    let mut generation = use_signal(|| 0u64);
    use_effect(move || {
        // subscribe to everything that can unlock a verification
        let _ = plan.read();
        let _ = snapshot.read();
        let _ = solver_state.read();
        // peek: the effect must not re-run on its own bookkeeping
        let current = *generation.peek() + 1;
        generation.set(current);
        let handle = handle.clone();
        spawn(async move {
            crate::browser::sleep_ms(500).await;
            if *generation.peek() != current {
                return;
            }
            let idle = {
                let state = solver_state.peek();
                state.ready
                    && state.running.is_none()
                    && state.verification.is_none()
                    && !state.verify_failed
            };
            if !idle {
                return;
            }
            let read = snapshot.peek();
            let Some(snapshot_ref) = read.as_ref() else {
                return;
            };
            let plan_read = plan.peek();
            if plan_read.program.is_none() {
                return;
            }
            let program =
                crate::panel::effective_program(snapshot_ref, &plan_read);
            // an intake error or a floating course: the verdict area
            // explains it — nothing to ask the solver yet
            let ready_to_prove = matches!(
                crate::solve::unplaced_codes(
                    snapshot_ref,
                    &plan_read,
                    program.as_ref(),
                ),
                Ok(unplaced) if unplaced.is_empty()
            );
            if ready_to_prove {
                request_verify(
                    &handle,
                    solver_state,
                    &plan_read,
                    program.as_ref(),
                );
            }
        });
    });
}

// ACT-7: persisted within 500 ms of the last change — each edit arms a
// fresh generation and only the newest one writes, so a burst of edits
// costs one write and no timer type juggling
fn save_on_change(plan: Signal<Plan>, view: Signal<View>) {
    let mut generation = use_signal(|| 0u64);
    use_effect(move || {
        let encoded_plan = persist::encode_plan(&plan.read());
        let encoded_view = persist::encode_view(&view.read());
        // peek: the effect must not re-run on its own bookkeeping
        let current = *generation.peek() + 1;
        generation.set(current);
        spawn(async move {
            crate::browser::sleep_ms(300).await;
            if *generation.peek() == current {
                crate::browser::local_set(persist::PLAN_KEY, &encoded_plan);
                crate::browser::local_set(persist::VIEW_KEY, &encoded_view);
            }
        });
    });
}

async fn load(
    mut snapshot: Signal<Option<Snapshot>>,
    mut state: Signal<LoadState>,
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
) {
    match crate::browser::fetch_raw_data().await {
        Err(error) => state.set(LoadState::Failed(present_data_error(&error))),
        Ok(raw) => {
            state.set(LoadState::Parsing);
            // paint the phase line before the parse blocks the thread
            crate::browser::next_frame().await;
            match crate::data::parse_data(&raw, manual.read().clone()) {
                Ok(parsed) => {
                    // what the load had to tolerate is shown, not logged
                    for warning in &parsed.warnings {
                        push_alert(alerts, AlertBody::Note(warning.clone()));
                    }
                    for collision in &parsed.collisions {
                        push_alert(
                            alerts,
                            AlertBody::Note(format!(
                                "Cours manuel « {collision} » masqué par le \
                                 catalogue (le cours officiel prime)."
                            )),
                        );
                    }
                    snapshot.set(Some(parsed));
                    state.set(LoadState::Ready);
                }
                Err(error) => {
                    state.set(LoadState::Failed(present_data_error(&error)));
                }
            }
        }
    }
}
