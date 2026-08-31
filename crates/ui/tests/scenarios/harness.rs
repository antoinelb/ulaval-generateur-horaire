// The app's own sequence, natively.
//
// `crates/ui/src/components/mod.rs` is wasm32-only: it can be neither run
// nor measured here. What it *does* is chain the pure modules — restore,
// edit, save, ask the solver, adopt or refuse its answer — and every one
// of those doors is a native function. This session replays that chain in
// the component's own order, with a `BTreeMap` where the browser has
// `localStorage` and a direct `protocol::handle` call where it has a Web
// Worker. It is a mirror, and its fidelity is the ADR's own caveat
// (`docs/conception/adr/2026-08-tests-de-scenario-dans-ui.md`): a rule
// that leaves a pure module for the view is a rule this file stops seeing.
// So nothing here decides anything — every verdict comes from `state`,
// `solve`, `persist`, `capsule`, `panel` or the solver itself.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use ulaval_scheduler_core::{Program, Season, Semester};
use ulaval_scheduler_ui::data::{self, RawData, Snapshot};
use ulaval_scheduler_ui::solve::{
    PlacementAnswer, PlacementReport, QueryKind,
};
use ulaval_scheduler_ui::state::{History, Plan, ProgramChoice};
use ulaval_scheduler_ui::{panel, persist, solve, state};

// The clock every scenario reads. Fixed, not `now()`: `restore_plan`,
// `fresh_plan` and `start_year_window` all take `today` as an argument
// precisely so a test may name it — a wall clock would make these
// scenarios expire on their own.
pub const TODAY: &str = "A26";

pub fn semester(raw: &str) -> Semester {
    raw.parse()
        .unwrap_or_else(|error| panic!("{raw} : {error}"))
}

pub fn today() -> Semester {
    semester(TODAY)
}

// The two shipped snapshots these scenarios need — the flagship bac and
// the one the directeur du B-GCI reported on. Loading the whole
// `data/programmes/` directory would only add picker noise.
const PROGRAM_FILES: [(&str, &str); 2] = [
    (
        "B-GEX-A26.json",
        include_str!("../../../../data/programmes/B-GEX-A26.json"),
    ),
    (
        "B-GCI-A26.json",
        include_str!("../../../../data/programmes/B-GCI-A26.json"),
    ),
];

// Parsed once: `parse_data` reads the committed 8 800-course catalogue,
// which costs seconds. Each session clones it, since applying a vintage's
// prerequisite corrections rewrites the catalogue in place.
fn shared_catalogue() -> &'static Snapshot {
    static CATALOGUE: OnceLock<Snapshot> = OnceLock::new();
    CATALOGUE.get_or_init(|| {
        let raw = RawData {
            courses: include_str!("../../../../data/cours.json").to_string(),
            meta: Some(include_str!("../../../../data/meta.json").to_string()),
            manual: Some(
                include_str!("../../../../data/cours.manuel.json").to_string(),
            ),
            programs: PROGRAM_FILES
                .iter()
                .map(|(name, contents)| {
                    ((*name).to_string(), (*contents).to_string())
                })
                .collect(),
        };
        data::parse_data(&raw, Vec::new(), Vec::new()).unwrap_or_else(
            |error| panic!("the committed data parses: {error}"),
        )
    })
}

// One query in flight: what `components::Running` records at send, plus
// the answer the worker will hand back. Splitting the send from the
// delivery is the whole point — a scenario can slip a plan change between
// the two, exactly as a student does.
pub struct Pending {
    pub id: u64,
    pub kind: QueryKind,
    // `SolverState::plan_generation` at send time
    pub plan_generation: u64,
    pub text: String,
}

// The app's signals, as plain fields.
pub struct Session {
    pub snapshot: Snapshot,
    pub plan: Plan,
    pub history: History,
    pub storage: BTreeMap<String, String>,
    pub today: Semester,
    // `SolverState`, minus the fields no native sequence can observe
    pub verification: Option<PlacementAnswer>,
    pub verification_stale: bool,
    pub left_out: BTreeSet<String>,
    pub last_error: Option<String>,
    plan_generation: u64,
    next_id: u64,
}

impl Session {
    // `components::restore_state` then `load`: the plan comes back from
    // storage, the catalogue is parsed, and the corrections the student's
    // vintage carries are applied to it (`boot_solver`).
    pub fn boot(storage: BTreeMap<String, String>) -> Session {
        let today = today();
        let restored = persist::restore_plan(
            storage.get(persist::PLAN_KEY).map(String::as_str),
            today,
        );
        let mut session = Session {
            snapshot: shared_catalogue().clone(),
            plan: restored.state,
            history: History::default(),
            storage,
            today,
            verification: None,
            verification_stale: false,
            left_out: BTreeSet::new(),
            last_error: None,
            plan_generation: 0,
            next_id: 0,
        };
        session.refresh_overrides();
        session
    }

    pub fn empty() -> Session {
        Session::boot(BTreeMap::new())
    }

    // « Choisir » : the panel's own resolution of the click's defaults,
    // then `persist::enter_document` and `components::swap_document`.
    pub fn open(&mut self, code: &str, vintage: &str) {
        let concentration =
            panel::default_concentration(&self.snapshot, code, vintage);
        let study_sessions =
            panel::program_credits_required(&self.snapshot, code, vintage)
                .map(state::default_study_sessions)
                .unwrap_or(state::DEFAULT_STUDY_SESSIONS);
        let choice = ProgramChoice {
            code: code.to_string(),
            semester: vintage.to_string(),
            concentration,
            profile: None,
        };
        let stored =
            self.storage.get(&persist::snapshot_key(&choice)).cloned();
        let swap = persist::enter_document(
            &self.plan,
            choice,
            stored.as_deref(),
            study_sessions,
            self.today,
        );
        self.swap(swap);
    }

    // « changer » : back to the picker, the document shelved.
    pub fn leave(&mut self) {
        let swap = persist::leave_document(&self.plan);
        self.swap(swap);
    }

    // `components::swap_document`, minus the alerts it pushes.
    fn swap(&mut self, swap: persist::DocumentSwap) {
        if let Some((key, encoded)) = swap.stash {
            self.storage.insert(key, encoded);
        }
        self.plan = swap.next;
        self.history = History::default();
        self.verification = None;
        self.verification_stale = false;
        self.left_out.clear();
        self.bump();
        self.refresh_overrides();
    }

    // `components::edit_plan` followed by `track_plan_change`: one
    // labelled, reversible act, and every verdict on screen goes stale.
    pub fn edit(&mut self, label: &str, edit: impl FnOnce(&mut Plan)) {
        state::apply(&mut self.plan, &mut self.history, label, edit);
        self.bump();
        self.heal();
    }

    pub fn undo(&mut self) -> Option<String> {
        let label = state::undo(&mut self.plan, &mut self.history);
        self.bump();
        label
    }

    // `components::save_on_change`, without its 300 ms debounce.
    pub fn save(&mut self) {
        self.storage.insert(
            persist::PLAN_KEY.to_string(),
            persist::encode_plan(&self.plan),
        );
    }

    // What a reload of the tab would put back on screen.
    pub fn reloaded_plan(&self) -> Plan {
        persist::restore_plan(
            self.storage.get(persist::PLAN_KEY).map(String::as_str),
            self.today,
        )
        .state
    }

    pub fn program(&self) -> Option<Program> {
        panel::effective_program(&self.snapshot, &self.plan)
    }

    // `boot_solver`: the worker's catalogue carries the student's own
    // prerequisite corrections, layered over his vintage's.
    fn refresh_overrides(&mut self) {
        let overrides = data::effective_overrides(&self.snapshot, &self.plan);
        data::set_prereq_overrides(&mut self.snapshot, &overrides);
    }

    // `heal_acquired`: no code the checked « scolarité préparatoire » box
    // or an entente credits may occupy a session. A derived correction, so
    // a direct write — never an undo step of its own.
    fn heal(&mut self) {
        let Some(program) = self.program() else {
            return;
        };
        let leftovers = solve::acquired_leftovers(&self.plan, &program);
        if !leftovers.is_empty() {
            state::purge_codes(&mut self.plan, &leftovers);
        }
    }

    fn bump(&mut self) {
        self.plan_generation += 1;
        self.verification_stale = true;
    }

    // --- the solver loop ---------------------------------------------------

    // `request_place`/`request_verify` plus the worker's own answer, held
    // instead of delivered. The catalogue is the one the worker holds.
    pub fn ask(&mut self, kind: QueryKind) -> Pending {
        self.next_id += 1;
        let id = self.next_id;
        let program = self.program();
        let request = match kind {
            QueryKind::Propose => solve::place_request(
                id,
                &self.plan,
                program.as_ref(),
                solve::PROPOSE_MAX_NODES,
            ),
            QueryKind::Verify => {
                solve::verify_request(id, &self.plan, program.as_ref())
            }
        };
        Pending {
            id,
            kind,
            plan_generation: self.plan_generation,
            text: ulaval_scheduler_wasm::protocol::handle(
                &request,
                &self.snapshot.courses,
            ),
        }
    }

    // `handle_worker_answer`.
    pub fn deliver(&mut self, pending: Pending) {
        let answer = solve::parse_worker_answer(&pending.text)
            .unwrap_or_else(|error| panic!("{error}"));
        match answer {
            solve::WorkerAnswer::Ready { .. } => {}
            solve::WorkerAnswer::Error { id, message } => {
                assert_eq!(id, pending.id, "an answer under a foreign id");
                self.last_error = Some(message);
            }
            solve::WorkerAnswer::Report { id, report } => {
                assert_eq!(id, pending.id, "an answer under a foreign id");
                self.last_error = None;
                match pending.kind {
                    QueryKind::Propose => self.adopt(&report),
                    QueryKind::Verify => {
                        // TRU-3 applied to a verdict: it judges the plan
                        // that was *sent*, and only settles if that plan
                        // is still the one on screen
                        let settles = solve::verdict_settles(
                            pending.plan_generation,
                            self.plan_generation,
                        );
                        self.verification = Some(report.placement);
                        self.verification_stale = !settles;
                    }
                }
            }
        }
    }

    pub fn propose(&mut self) {
        let pending = self.ask(QueryKind::Propose);
        self.deliver(pending);
    }

    pub fn verify(&mut self) {
        let pending = self.ask(QueryKind::Verify);
        self.deliver(pending);
    }

    // `components::apply_proposal`, its write path alone: the answer is
    // refused whole when it would unseat a displayed course, the pins are
    // overlaid on what is kept, and a no-op answer writes nothing.
    fn adopt(&mut self, report: &PlacementReport) {
        let Some(solution) = report.placement.solutions.first() else {
            self.left_out.clear();
            return;
        };
        let regressions = solve::adoption_regressions(
            &self.plan.displayed_placement,
            &solution.left_out,
        );
        self.left_out = solution
            .left_out
            .iter()
            .filter(|code| !regressions.contains(code))
            .cloned()
            .collect();
        if !regressions.is_empty() {
            return;
        }
        let mut placement = solution.placement.clone();
        state::overlay_pins(&self.plan, &mut placement);
        let unchanged = self.plan.displayed_placement == placement
            && report
                .injected
                .iter()
                .all(|code| self.plan.electives.contains(code));
        if unchanged {
            return;
        }
        self.plan.displayed_placement = placement;
        for code in &report.injected {
            if !self.plan.electives.contains(code) {
                self.plan.electives.push(code.clone());
            }
        }
        // the write is a plan change like any other: it re-arms every
        // effect, the automatic verification included
        self.bump();
    }

    // Propose until the grid stops moving — the `proposed` fingerprint of
    // `auto_propose`, expressed as a bounded loop rather than a signal.
    pub fn settle(&mut self) {
        let mut previous = self.plan.displayed_placement.clone();
        for _ in 0..8 {
            self.propose();
            if self.plan.displayed_placement == previous {
                return;
            }
            previous = self.plan.displayed_placement.clone();
        }
        panic!("the automatic placement never converged");
    }

    // --- the gestures the scenarios make -----------------------------------

    // The chip strip, the ribbon drag and the grid drop all land here
    // (`components::panel::place_course`): the pin is written, and the
    // session it lands in is judged out loud.
    pub fn pin(&mut self, code: &str, session: usize) -> Option<String> {
        let warning =
            solve::pin_warning(&self.snapshot, &self.plan, session, code);
        let owned = code.to_string();
        self.edit(&format!("{code} épinglé"), |plan| {
            plan.pinned_sessions.insert(owned.clone(), session);
            plan.displayed_placement.insert(owned, session);
        });
        warning
    }

    // The « Début » select: `solve::placed_offerings` then
    // `state::set_start`, in one undoable act.
    pub fn set_start(&mut self, start: &str) -> state::StartMove {
        let offerings = solve::placed_offerings(&self.snapshot, &self.plan);
        let start = semester(start);
        let mut moved = state::StartMove::default();
        self.edit(&format!("Début déplacé à {start}"), |plan| {
            moved = state::set_start(plan, start, &offerings);
        });
        moved
    }

    // What the grid draws for one session, in credits — the number the
    // directeur du B-GCI read off the H4 column.
    pub fn session_credits(&self, session: usize) -> u32 {
        state::session_codes(&self.plan, session)
            .iter()
            .filter_map(|code| self.snapshot.by_code.get(code))
            .map(|&index| self.snapshot.courses[index].credits.planning())
            .sum()
    }

    pub fn seasons(&self) -> Vec<Season> {
        ulaval_scheduler_core::horizon_sessions(
            self.plan.start.season,
            self.plan.study_sessions,
        )
    }
}

pub fn placed(plan: &Plan, code: &str) -> usize {
    *plan
        .displayed_placement
        .get(code)
        .unwrap_or_else(|| panic!("{code} is not on the grid"))
}
