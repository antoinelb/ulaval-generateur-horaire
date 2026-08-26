// The view layer — wasm32-only, mechanical, no business rule: every value
// it renders comes from `core`, `ui-calculations` or the pure modules.

pub mod grid;
pub mod header;
pub mod panel;
pub mod print;
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
const PRINT_CSS: Asset = asset!("/assets/print.css");
const PRINT_ORGANIGRAMME_CSS: Asset = asset!("/assets/print-organigramme.css");
const PRINT_HORAIRE_CSS: Asset = asset!("/assets/print-horaire.css");

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
    // what the alert reports on — a caused alert also retires by itself
    // when its cause disappears (ADR
    // `2026-08-peremption-des-toasts-par-cause`)
    pub cause: AlertCause,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum AlertCause {
    // lives until dismissed (ALR-4)
    #[default]
    Sticky,
    // this code could not be seated — stale once it sits somewhere or
    // leaves the plan
    LeftOut(String),
    // nothing could be placed at all — stale once anything is
    EmptyGrid,
    // the worker refused a query — stale the moment a later one answers:
    // the refusal described an input that no longer stands (rapport
    // étudiante-gex 2026-08-20, « ANL-1010 is pinned outside 1..=4 »
    // survivait au retour à 8 sessions)
    SolverError,
    // an announcement about the current document (injection, étés forcés,
    // acquis présumés…) — it leaves with it at a swap: « GMC-3020 en
    // été » under B-GIN named a course of another program (contre-test
    // étudiante-cegep 2026-08-20)
    Document,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AlertBody {
    Note(String),
    // a confirmation, styled apart from the warnings (INP-3: the glyph
    // carries the difference, not the colour alone)
    Success(String),
    Error(UiError),
    // a destructive act with no confirmation dialog, undoable from the
    // toast itself (pattern ACT, plan item 9) — not a `Success`: it must
    // outlive the 5 s auto-clear or the undo would vanish with it, so it
    // carries the removed program instead of already-worded text
    LocalProgramRemoved(crate::import::LocalProgram),
}

// the course whose alternatives (ghosts) the grid currently shows —
// selected by click or keyboard, never by hover alone (INP-5)
#[derive(Clone, Copy, PartialEq)]
pub struct SelectedCourse(pub Signal<Option<String>>);

// the course a drag — grid block or ribbon code — is carrying toward a
// session card (note 16) — the payload rides in a signal, never in
// DataTransfer (a token is still written there: Firefox refuses to carry
// an empty drag); the chips stay the keyboard-reachable equivalent (INP-4)
#[derive(Clone, Copy, PartialEq)]
pub struct DraggedCourse(pub Signal<Option<String>>);

// the session card the drag is currently over — the darker border that
// says where the course would land if released (retour d'Antoine,
// 2026-08-19)
#[derive(Clone, Copy, PartialEq)]
pub struct DropHover(pub Signal<Option<usize>>);

// the student's hand-entered Courses, persisted apart from the plan
#[derive(Clone, Copy, PartialEq)]
pub struct ManualCourses(pub Signal<Vec<ulaval_scheduler_core::Course>>);

// the student's programs imported by URL, persisted apart from the plan
// (plan item 5/6)
#[derive(Clone, Copy, PartialEq)]
pub struct LocalPrograms(pub Signal<Vec<crate::import::LocalProgram>>);

// which print sheet, if any, `print::PrintView` should mount right now —
// `None` outside a print (EXP-4, export-pdf plan item 2)
#[derive(Clone, Copy, PartialEq)]
pub struct PrintTarget(pub Signal<Option<print::PrintKind>>);

// --- solver B, off the main thread ----------------------------------------

#[derive(Clone, Debug, PartialEq, Default)]
pub struct SolverState {
    pub ready: bool,
    // the one blocking query (proposition/vérification)
    pub running: Option<Running>,
    // the last verify answer, shown until the plan changes
    pub verification: Option<crate::solve::PlacementAnswer>,
    // the last automatic verify errored: do not refire until the plan
    // changes, or the effect would loop on the same failure
    pub verify_failed: bool,
    // the request of the last automatic proposal, recorded at send: the
    // same query is never sent twice, which is both the convergence guard
    // of `auto_propose` and what makes a cancel stick until the next edit
    pub proposed: Option<String>,
    // the last proposal was a best-effort filling and these are the courses
    // it could not seat — so the panel stops telling the student to propose
    // an organigramme he has just proposed (ADR
    // `2026-08-placement-au-mieux-en-repli`). Overwritten whole by every
    // propose answer, purged code by code once a course stops floating
    // (`retire_stale_left_out`, ADR
    // `2026-08-peremption-des-toasts-par-cause`)
    pub left_out: std::collections::BTreeSet<String>,
    // Derived from the latest proposal or verification report. Never
    // persisted; a document swap clears it with the other solver facts.
    pub credit_shortfalls: Vec<crate::solve::CreditShortfallAnswer>,
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
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    // the worker's catalogue must hold exactly what the panel's does: the
    // repo's hand-maintained courses, the student's own, and the
    // prerequisites his program vintage rewrote
    let (shared, overrides) = match snapshot.peek().as_ref() {
        Some(snapshot_ref) => (
            snapshot_ref.manual.courses.clone(),
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
            handle_worker_answer(&text, state, plan, alerts, snapshot);
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
    alerts: Signal<Vec<Alert>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    match crate::solve::parse_worker_answer(text) {
        Err(message) => push_alert(alerts, AlertBody::Note(message)),
        Ok(crate::solve::WorkerAnswer::Ready { .. }) => {
            state.write().ready = true;
        }
        Ok(crate::solve::WorkerAnswer::Error { id, message }) => {
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
                push_caused_alert(
                    alerts,
                    AlertBody::Note(format!(
                        "Le solveur n'a pas pu répondre — détail \
                         technique : {message}"
                    )),
                    AlertCause::SolverError,
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
            // a query just answered: whatever refusal was on screen
            // described an input that no longer stands
            if alerts
                .peek()
                .iter()
                .any(|alert| alert.cause == AlertCause::SolverError)
            {
                let mut alerts = alerts;
                alerts
                    .write()
                    .retain(|alert| alert.cause != AlertCause::SolverError);
            }
            match running.kind {
                QueryKind::Propose => {
                    apply_proposal(&report, state, plan, alerts, snapshot);
                }
                QueryKind::Verify => {
                    let mut state = state.write();
                    state.verification = Some(report.placement.clone());
                    state.credit_shortfalls = report.credit_shortfalls;
                }
            }
        }
    }
}

// The proposal is a derived correction, not a student act: it lands by a
// direct write, never through `edit_plan` — undoing the act that made the
// solver move restores the plan whole, and the placement recomputes (same
// reasoning as `heal_acquired`). Everything the solver presumed or set
// aside is still said.
fn apply_proposal(
    report: &crate::solve::PlacementReport,
    mut state: Signal<SolverState>,
    plan: Signal<Plan>,
    alerts: Signal<Vec<Alert>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    state.write().credit_shortfalls = report.credit_shortfalls.clone();
    if let Some(note) = crate::solve::completion_note(&report.placement) {
        push_caused_alert(alerts, AlertBody::Note(note), AlertCause::Document);
    }
    // a best-effort answer words every culprit itself, blocked ones
    // included — the per-blocked loop would say each of them twice
    match report.placement.solutions.first() {
        Some(solution) if !solution.left_out.is_empty() => {
            if solution.placement.is_empty() {
                // one aggregate verdict, not one toast per code: the
                // whole grid failing is a single fact
                push_caused_alert(
                    alerts,
                    AlertBody::Note(crate::solve::empty_grid_note()),
                    AlertCause::EmptyGrid,
                );
            } else {
                let plan_read = plan.peek();
                let snapshot_read = snapshot.peek();
                for code in &solution.left_out {
                    let blocked = report
                        .placement
                        .blocked
                        .iter()
                        .find(|blocked| &blocked.code == code);
                    push_caused_alert(
                        alerts,
                        AlertBody::Note(crate::solve::left_out_line(
                            code,
                            blocked,
                            &plan_read,
                            snapshot_read.as_ref(),
                        )),
                        AlertCause::LeftOut(code.clone()),
                    );
                }
            }
        }
        _ => {
            for blocked in &report.placement.blocked {
                push_caused_alert(
                    alerts,
                    AlertBody::Note(crate::solve::blocked_note(blocked)),
                    AlertCause::LeftOut(blocked.code.clone()),
                );
            }
        }
    }
    for code in &report.set_aside {
        push_caused_alert(
            alerts,
            AlertBody::Note(format!(
                "{code} : au programme mais absent du catalogue — mis de \
                 côté, à suivre à la main."
            )),
            AlertCause::Document,
        );
    }
    if !report.summers_forced.is_empty() {
        push_caused_alert(
            alerts,
            AlertBody::Note(crate::solve::summers_forced_note(
                &report.summers_forced,
            )),
            AlertCause::Document,
        );
    }
    // every answer overwrites `left_out` whole — an answer with no
    // solution at all must clear it, not leave the previous one frozen
    state.write().left_out = report
        .placement
        .solutions
        .first()
        .map(|solution| solution.left_out.clone())
        .unwrap_or_default();
    let Some(solution) = report.placement.solutions.first() else {
        return;
    };
    if !solution.assumed.is_empty() {
        let assumed: Vec<&str> =
            solution.assumed.iter().map(String::as_str).collect();
        push_caused_alert(
            alerts,
            AlertBody::Note(format!(
                "Le cheminement présume ces acquis (préalables que \
                 l'outil ne peut pas vérifier) : {}. Assurez-vous que \
                 cela vous décrit.",
                assumed.join(", ")
            )),
            AlertCause::Document,
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
        push_caused_alert(
            alerts,
            AlertBody::Note(format!("{} {added}.", injected.join(", "))),
            AlertCause::Document,
        );
    }
    let mut placement = solution.placement.clone();
    // a stale answer (solved before the latest pins) must not evict them
    // from the grid — `displayed ⊇ pinned`, always
    crate::state::overlay_pins(&plan.peek(), &mut placement);
    let injected = report.injected.clone();
    // a no-op answer (best-effort that moved nothing, or the re-solve of
    // an already-applied proposal) must not write: the write is what
    // re-arms every plan effect, `auto_propose` included
    let unchanged = {
        let read = plan.peek();
        read.displayed_placement == placement
            && injected.iter().all(|code| read.electives.contains(code))
    };
    if unchanged {
        return;
    }
    let mut plan = plan;
    let mut write = plan.write();
    write.displayed_placement = placement;
    // the plan must hold what the grid shows: an injected course left
    // out of `electives` would vanish from the very next request (the
    // preparatory-purge lesson)
    for code in injected {
        if !write.electives.contains(&code) {
            write.electives.push(code);
        }
    }
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
    alerts: Signal<Vec<Alert>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    snapshot: Signal<Option<Snapshot>>,
) {
    if let Some(solver) = handle.0.borrow_mut().take() {
        solver.terminate();
    }
    // `proposed` deliberately survives: the student's cancel must hold
    // until the next edit, or `auto_propose` would relaunch the very
    // search just killed
    state.write().running = None;
    boot_solver(handle, state, plan, alerts, manual, snapshot);
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
    let local_programs = use_signal(|| restored.local_programs.clone());
    let print_target = use_signal(|| None::<print::PrintKind>);
    use_context_provider(|| ManualCourses(manual));
    use_context_provider(|| LocalPrograms(local_programs));
    use_context_provider(|| PrintTarget(print_target));
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
        boot_solver(&handle, solver_state, plan, alerts, manual, snapshot);
        crate::browser::register_service_worker();
        handle
    });
    use_context_provider(|| handle.clone());
    use_future(move || {
        load(snapshot, load_state, alerts, manual, local_programs)
    });
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
    // a plan change stales the verify answer — `left_out` has its own
    // retirement: `retire_stale_left_out` purges entry and toast together
    // when the cause disappears, and keeps what an answer just reported
    // (its codes still float)
    use_effect(move || {
        let _ = plan.read();
        let mut solver_state = solver_state;
        let mut state = solver_state.write();
        state.verification = None;
        state.verify_failed = false;
    });
    retire_stale_left_out(plan, snapshot, manual, solver_state, alerts);
    apply_corrections(
        plan,
        snapshot,
        alerts,
        handle.clone(),
        solver_state,
        manual,
    );
    heal_acquired(plan, snapshot, alerts);
    auto_propose(plan, snapshot, solver_state, handle.clone());
    auto_verify(plan, snapshot, solver_state, handle.clone());
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: PRINT_CSS }
        document::Stylesheet { href: PRINT_ORGANIGRAMME_CSS }
        document::Stylesheet { href: PRINT_HORAIRE_CSS }
        shell::Screen {}
        print::PrintView {}
    }
}

struct RestoredState {
    plan: Plan,
    view: View,
    manual: Vec<ulaval_scheduler_core::Course>,
    local_programs: Vec<crate::import::LocalProgram>,
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
    let local_programs = persist::restore_local_programs(
        crate::browser::local_get(persist::LOCAL_PROGRAMS_KEY).as_deref(),
    );
    // stash what would otherwise be overwritten by the next save
    for backup in [
        &plan.backup,
        &view.backup,
        &manual.backup,
        &local_programs.backup,
    ]
    .into_iter()
    .flatten()
    {
        crate::browser::stash_backup(backup);
    }
    let mut notes = plan.notes;
    notes.extend(view.notes);
    notes.extend(manual.notes);
    notes.extend(local_programs.notes);
    RestoredState {
        plan: plan.state,
        view: view.state,
        manual: manual.state,
        local_programs: local_programs.state,
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
            cause: AlertCause::Sticky,
        })
        .collect()
}

pub fn push_alert(alerts: Signal<Vec<Alert>>, body: AlertBody) {
    push_caused_alert(alerts, body, AlertCause::Sticky);
}

pub fn push_caused_alert(
    mut alerts: Signal<Vec<Alert>>,
    body: AlertBody,
    cause: AlertCause,
) {
    let mut list = alerts.write();
    // never the same message twice (ALR-3) — but a repeat is refreshed to
    // the front instead of swallowed: relaunching a search that ends on
    // the same verdict must still visibly answer (rapport 2026-08-14)
    list.retain(|alert| alert.body != body);
    list.push(Alert {
        key: next_alert_key(),
        body,
        cause,
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

// The second door beside `edit_plan`: swapping documents — « changer »,
// « Choisir » — replaces the document instead of editing it. The history
// dies with its document (an undo across documents would fork the shelf
// copy); reversibility comes from the shelf itself (ADR
// `2026-08-historique-par-document-vide-a-la-bascule`). All the decisions
// are in `swap` (`persist::leave_document`/`enter_document`, tested
// natively) — this only writes localStorage and the signals, in an order
// that never lets a stale answer land in the next document.
#[allow(clippy::too_many_arguments)]
pub fn swap_document(
    plan: Signal<Plan>,
    view: Signal<View>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
    solver_state: Signal<SolverState>,
    handle: &SolverHandle,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    snapshot: Signal<Option<Snapshot>>,
    swap: persist::DocumentSwap,
) {
    // a search in flight must not land its proposal in the next document
    if solver_state.peek().running.is_some() {
        cancel_search(handle, solver_state, plan, alerts, manual, snapshot);
    }
    // the shelf write is synchronous — never behind the save debounce, so
    // a tab closed right after the swap loses nothing
    if let Some((key, encoded)) = &swap.stash {
        crate::browser::local_set(key, encoded);
    }
    if let Some(backup) = &swap.backup {
        crate::browser::stash_backup(backup);
    }
    for note in &swap.notes {
        push_alert(alerts, AlertBody::Note(note.clone()));
    }
    let mut plan = plan;
    plan.set(swap.next);
    let mut history = history;
    history.set(History::default());
    let mut view = view;
    view.set(View::default());
    // the plan-change effect clears the verification; these two belong to
    // the propose answers of the *previous* document and would otherwise
    // survive into the new one
    let mut solver_state = solver_state;
    let mut state = solver_state.write();
    state.left_out.clear();
    state.credit_shortfalls.clear();
    state.proposed = None;
    drop(state);
    // the document's own announcements and verdicts leave with it — only
    // the Sticky alerts (load warnings, confirmations) outlive a swap
    if alerts
        .peek()
        .iter()
        .any(|alert| alert.cause != AlertCause::Sticky)
    {
        let mut alerts = alerts;
        alerts
            .write()
            .retain(|alert| alert.cause == AlertCause::Sticky);
    }
}

// Suppression d'un programme importé par URL (plan item 9, pattern ACT) :
// no confirmation dialog, but fully undoable from the toast it pushes.
// Local programs live outside the `Plan`, so this does not go through
// `edit_plan`/`History` — `Ctrl+Z` cannot reach them — except for the one
// case where the removed program was the active document: that leaves the
// document exactly like « changer » does (`swap_document`/
// `persist::leave_document`), never a bare `edit_plan(|plan| plan.program =
// None)`, or every guarantee the swap provides is skipped — the shelf
// write that lets the toast's Annuler-then-Choisir round-trip find the
// placements again, the placements themselves wiped, solver state and
// Document alerts purged, and History cleared so `Ctrl+Z` cannot land a
// step naming a program no longer in `snapshot.programs` (ADR
// `2026-08-historique-par-document-vide-a-la-bascule`).
#[allow(clippy::too_many_arguments)]
pub fn remove_local_program(
    mut snapshot: Signal<Option<Snapshot>>,
    mut local_programs: Signal<Vec<crate::import::LocalProgram>>,
    plan: Signal<Plan>,
    view: Signal<View>,
    history: Signal<History>,
    alerts: Signal<Vec<Alert>>,
    solver_state: Signal<SolverState>,
    handle: &SolverHandle,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    code: &str,
    semester: &str,
) {
    let removed = {
        let mut write = snapshot.write();
        let Some(snapshot_mut) = write.as_mut() else {
            return;
        };
        crate::data::remove_local_program(snapshot_mut, code, semester)
    };
    let Some(removed) = removed else {
        return;
    };
    local_programs.write().retain(|local| {
        !(local.program.code == code
            && local.program.semester.to_string() == semester)
    });
    crate::browser::local_set(
        persist::LOCAL_PROGRAMS_KEY,
        &persist::encode_local_programs(&local_programs.read()),
    );
    let was_active = plan.peek().program.as_ref().is_some_and(|choice| {
        choice.code == code && choice.semester == semester
    });
    if was_active {
        let swap = persist::leave_document(&plan.peek());
        swap_document(
            plan,
            view,
            history,
            alerts,
            solver_state,
            handle,
            manual,
            snapshot,
            swap,
        );
    }
    push_alert(alerts, AlertBody::LocalProgramRemoved(removed));
}

// The exact inverse of `remove_local_program`, driven by the toast's
// « Annuler » — restores the program to both the snapshot and the
// persisted list, but never the plan's program choice: `state::undo`
// remains the tool for the plan, and the toast text already says what it
// restores.
pub fn restore_local_program(
    mut snapshot: Signal<Option<Snapshot>>,
    mut local_programs: Signal<Vec<crate::import::LocalProgram>>,
    alerts: Signal<Vec<Alert>>,
    local: crate::import::LocalProgram,
) {
    let added = {
        let mut write = snapshot.write();
        let Some(snapshot_mut) = write.as_mut() else {
            return;
        };
        crate::data::add_local_program(snapshot_mut, local.clone())
    };
    if let Err(detail) = added {
        push_alert(
            alerts,
            AlertBody::Error(crate::present::present_local_program_conflict(
                &detail,
            )),
        );
        return;
    }
    local_programs.write().push(local);
    crate::browser::local_set(
        persist::LOCAL_PROGRAMS_KEY,
        &persist::encode_local_programs(&local_programs.read()),
    );
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
            // the current document goes to its shelf first — unless the
            // link is the same (program, vintage): a later « changer »
            // would overwrite the shelf with the shared version. The
            // history restarts so the one undoable step below is the way
            // back to the student's own document.
            if let Some((key, encoded)) =
                persist::import_stash(&plan.peek(), &shared)
            {
                crate::browser::local_set(&key, &encoded);
            }
            let mut history = history;
            history.set(History::default());
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

// An acquired course is an invariant, not a request filter: no code the
// checked « scolarité préparatoire » box or an entente credits may occupy
// a session. Whatever slipped one in (an old save, a shared link, an act
// done before the mark) is purged here — loudly, never silently. A write,
// not `edit_plan`: a derived correction is no student act; undoing the
// checkbox itself restores the pre-toggle plan whole, placements included.
fn heal_acquired(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    alerts: Signal<Vec<Alert>>,
) {
    use_effect(move || {
        // materialize before any write: the read borrows must die first
        let (credited, preparatory) = {
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
            crate::solve::acquired_leftovers(&plan_read, &program)
                .into_iter()
                .partition::<Vec<String>, _>(|code| {
                    plan_read.credited.contains(code)
                })
        };
        let leftovers = [&credited[..], &preparatory[..]].concat();
        if leftovers.is_empty() {
            return;
        }
        let mut plan = plan;
        crate::state::purge_codes(&mut plan.write(), &leftovers);
        // one note per family: the way out named is the control that
        // actually undoes it
        for (codes, credited) in [(&credited, true), (&preparatory, false)] {
            if !codes.is_empty() {
                push_alert(
                    alerts,
                    AlertBody::Note(crate::solve::purge_note(codes, credited)),
                );
            }
        }
    });
}

// A left-out warning retires with its cause: a code seated since (by a
// later answer or by hand) or gone from the plan stops floating, and its
// toast and `SolverState.left_out` entry go together. `solver_state` and
// `alerts` are read by peek — the effect must not re-run on its own
// purges — and the codes an answer has *just* reported still float by
// construction (its auto-application writes them nowhere), so they
// survive: the bug this replaces was the answer erasing its own report
// (ADR `2026-08-peremption-des-toasts-par-cause`).
fn retire_stale_left_out(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    manual: Signal<Vec<ulaval_scheduler_core::Course>>,
    solver_state: Signal<SolverState>,
    alerts: Signal<Vec<Alert>>,
) {
    use_effect(move || {
        let plan_read = plan.read();
        let read = snapshot.read();
        let _ = manual.read();
        let Some(snapshot_ref) = read.as_ref() else {
            return;
        };
        let program =
            crate::panel::effective_program(snapshot_ref, &plan_read);
        let Ok(floating) = crate::solve::unplaced_codes(
            snapshot_ref,
            &plan_read,
            program.as_ref(),
        ) else {
            return;
        };
        let stale = crate::solve::stale_left_out(
            &solver_state.peek().left_out,
            &floating,
        );
        if !stale.is_empty() {
            let mut solver_state = solver_state;
            solver_state
                .write()
                .left_out
                .retain(|code| !stale.contains(code));
        }
        let something_placed = !plan_read.displayed_placement.is_empty()
            || plan_read.manual.values().any(|codes| !codes.is_empty());
        // judged against what floats *now*, never against `left_out`: a
        // document swap clears `left_out` first, and the old document's
        // toasts would otherwise never match anything again
        let expired = |alert: &Alert| match &alert.cause {
            AlertCause::Sticky => false,
            AlertCause::LeftOut(code) => {
                !floating.iter().any(|float| float == code)
            }
            AlertCause::EmptyGrid => something_placed || floating.is_empty(),
            // retired by the next worker answer, not by the plan
            AlertCause::SolverError => false,
            // retired by the document swap, not by the plan
            AlertCause::Document => false,
        };
        if alerts.peek().iter().any(expired) {
            let mut alerts = alerts;
            alerts.write().retain(|alert| !expired(alert));
        }
    });
}

// The corrections in force follow the plan — the student's admission
// vintage and his own edits — so the catalogue is rewritten here and not at
// parse time, when no program is picked yet. Same shape as
// `heal_acquired`: derived state, a direct write (no student act to
// undo), and a guard that makes it converge — once applied, `applied`
// matches and the next run returns.
fn apply_corrections(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    alerts: Signal<Vec<Alert>>,
    handle: SolverHandle,
    solver_state: Signal<SolverState>,
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
        // learns the corrections — and an already-sent proposal may answer
        // differently against them, so its fingerprint no longer counts
        let mut solver_state = solver_state;
        solver_state.write().proposed = None;
        cancel_search(&handle, solver_state, plan, alerts, manual, snapshot);
    });
}

// The proposal is not a button either (décision 2026-08-19, ADR
// `2026-08-organigramme-en-continu-sans-bouton`): whenever the plan
// settles with floating courses — or a verify just failed and the grid
// needs repairing around the student's pins — the placement re-runs by
// itself, seeded by the displayed placement so it moves as little as
// possible. Convergence is the `proposed` fingerprint: the same request
// is never sent twice, so a best-effort answer whose `left_out` persists,
// a repair that cannot improve, or a cancelled search all stop the loop
// until the plan changes.
fn auto_propose(
    plan: Signal<Plan>,
    snapshot: Signal<Option<Snapshot>>,
    solver_state: Signal<SolverState>,
    handle: SolverHandle,
) {
    let mut generation = use_signal(|| 0u64);
    use_effect(move || {
        // subscribe to everything that can call for a placement
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
                state.ready && state.running.is_none()
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
            // floating courses to seat, or a broken grid to repair — a
            // verify that answered « no solution » (empty `solutions`,
            // never a worker error: that one would refuse the repair too)
            // means a student act broke a constraint, and the placement
            // reorganizes the unpinned courses around it; an unreadable
            // input is the verdict area's to explain
            let needed = match crate::solve::unplaced_codes(
                snapshot_ref,
                &plan_read,
                program.as_ref(),
            ) {
                Ok(unplaced) if !unplaced.is_empty() => true,
                Ok(_) => solver_state
                    .peek()
                    .verification
                    .as_ref()
                    .is_some_and(|answer| answer.solutions.is_empty()),
                Err(_) => false,
            };
            if !needed {
                return;
            }
            // the request itself is the fingerprint (id 0 is nobody's):
            // it captures exactly what the solver would see
            let fingerprint = crate::solve::place_request(
                0,
                &plan_read,
                program.as_ref(),
                crate::solve::PROPOSE_MAX_NODES,
            );
            if solver_state.peek().proposed.as_deref()
                == Some(fingerprint.as_str())
            {
                return;
            }
            let mut solver_state = solver_state;
            solver_state.write().proposed = Some(fingerprint);
            request_place(
                &handle,
                solver_state,
                &plan_read,
                program.as_ref(),
                crate::solve::PROPOSE_MAX_NODES,
            );
        });
    });
}

// Note 6 (2026-08-13): verification is not a button — it re-runs by
// itself, debounced, whenever the plan settles with a program chosen and
// every requested course placed. The generation counter keeps a burst of
// edits down to one query; the guards make the effect converge (a fired
// query sets `running`, its answer sets `verification`, both stop it).
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
    local_programs: Signal<Vec<crate::import::LocalProgram>>,
) {
    match crate::browser::fetch_raw_data().await {
        Err(error) => state.set(LoadState::Failed(present_data_error(&error))),
        Ok(raw) => {
            state.set(LoadState::Parsing);
            // paint the phase line before the parse blocks the thread
            crate::browser::next_frame().await;
            match crate::data::parse_data(
                &raw,
                manual.read().clone(),
                local_programs.read().clone(),
            ) {
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
