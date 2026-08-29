use std::collections::{BTreeMap, BTreeSet};

use ulaval_scheduler_core::{
    PrereqOverride, Season, Semester, MAX_STUDY_SESSIONS,
};

pub const HISTORY_CAP: usize = 100;
// 17 gives every bac headroom out of the box: the B-GMC packs its full
// 120 cr of mandatories and rules into 8 sessions, which 8 × 15 cannot
// hold, and its official cheminement itself opens at 16 cr (ADR
// `2026-08-plafond-par-defaut-17-credits`). Still a setting, never a wall.
pub const DEFAULT_CREDIT_CAP: u32 = 17;
// A1→H8 : the bac's eight study sessions (étés come on top, in core)
pub const DEFAULT_STUDY_SESSIONS: usize = 8;

// The student's whole document — only codes, indices and choices, never
// Course data (the snapshot owns that). One struct so undo is one clone
// and persistence one write. Field-level `serde(default)` is the restore
// tolerance: an old save missing a field starts that field fresh.
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct Plan {
    pub program: Option<ProgramChoice>,
    // the A1 identity — « A26 » = automne 2026
    pub start: Semester,
    pub study_sessions: usize,
    // 1-based indices des sessions gelées (import Capsule, ou la bascule
    // « geler » de l'étudiant) : le solveur n'y ajoute rien et n'en
    // déplace rien — l'étudiant, lui, y touche librement (ADRs
    // `2026-08-sessions-completees-fermees-au-solveur`,
    // `2026-08-sessions-gelees-generalisent-les-completees`)
    pub frozen: BTreeSet<usize>,
    pub summers_open: bool,
    pub credit_cap: u32,
    pub concomitant: bool,
    // « scolarité préparatoire faite » — the 0xxx courses ride as
    // `PlaceQuery.passed` so the solver neither places nor counts them; a
    // course the student really took lives in its past session instead (ADR
    // `2026-08-retrait-de-la-notion-de-cours-reussi`)
    pub preparatory_done: bool,
    // typed additions to the program's list, in the order they were added
    pub electives: Vec<String>,
    // code → the block it was chosen under: « c » (concentration) or
    // « f » (profil). Changing that block takes its electives away with
    // it, even ones the new scope still covers (ADR
    // `2026-08-electifs-choisis-sous-le-bloc-partent-avec-lui`). Absent
    // from an older save and from a shared link: those electives fall back
    // to coverage alone, which is what `panel::scope_orphans` decides.
    pub elective_origins: BTreeMap<String, String>,
    // code → 1-based session: the student's explicit organigramme acts
    pub pinned_sessions: BTreeMap<String, usize>,
    // the organigramme as shown: pins plus the last accepted proposal —
    // also the seed of the next solve, so proposals stay close to it
    pub displayed_placement: BTreeMap<String, usize>,
    // session → code → pinned NRC set (the weekly « forcer une section »)
    pub chosen: BTreeMap<usize, BTreeMap<String, BTreeSet<String>>>,
    // session → codes added by hand outside the placement (v0 flow)
    pub manual: BTreeMap<usize, Vec<String>>,
    // session → free label (« à l'étranger ») — pure annotation
    pub special: BTreeMap<usize, String>,
    // ententes avec la direction : code → section key (« p/Règle 2 ») of
    // the rule the course counts toward; pure data, applied by
    // `panel::granted_program` before any coverage call
    pub rule_grants: BTreeMap<String, String>,
    // cours crédités par entente : acquired in advance, so they ride as
    // `PlaceQuery.passed` like the préparatoire ones — counted in the
    // credits and in the coverage, never given a session (ADR
    // `2026-08-cours-credite-hors-session`)
    pub credited: BTreeSet<String>,
    // code → the prerequisites as the student's own program vintage wrote
    // them, when they differ from today's répertoire. Layered *over* the
    // vintage file's own corrections, and applied to the catalogue before
    // the solver ever reads it (ADR
    // `2026-08-correction-des-prealables-par-millesime`)
    pub prereq_overrides: BTreeMap<String, PrereqOverride>,
}

impl Default for Plan {
    fn default() -> Self {
        Plan {
            program: None,
            start: Semester {
                season: Season::Fall,
                year: 2026,
            },
            study_sessions: DEFAULT_STUDY_SESSIONS,
            frozen: BTreeSet::new(),
            summers_open: false,
            credit_cap: DEFAULT_CREDIT_CAP,
            concomitant: false,
            preparatory_done: true,
            electives: Vec::new(),
            elective_origins: BTreeMap::new(),
            pinned_sessions: BTreeMap::new(),
            displayed_placement: BTreeMap::new(),
            chosen: BTreeMap::new(),
            manual: BTreeMap::new(),
            special: BTreeMap::new(),
            rule_grants: BTreeMap::new(),
            credited: BTreeSet::new(),
            prereq_overrides: BTreeMap::new(),
        }
    }
}

#[derive(
    Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct ProgramChoice {
    // official code, « B-GEX »
    pub code: String,
    // the vintage the student enrolled under, « A26 » — names the snapshot
    pub semester: String,
    pub concentration: Option<String>,
    pub profile: Option<String>,
}

// The credit-based default horizon (ADR
// `2026-08-sessions-par-defaut-derivees-des-credits`) : 15 credits is a
// full-time session's load, so ceil(credits_required / 15) is how many
// study sessions the program's own weight suggests — clamped to the range
// the « Sessions » number input already enforces (components/panel.rs). A
// non-positive `credits_required` (the field is `i64`, the file could carry
// anything) falls back to `DEFAULT_STUDY_SESSIONS` before any cast, so no
// negative value ever wraps through `usize`.
pub fn default_study_sessions(credits_required: i64) -> usize {
    if credits_required <= 0 {
        return DEFAULT_STUDY_SESSIONS;
    }
    let sessions = (credits_required + 14) / 15;
    (sessions as usize).clamp(2, 16)
}

// The seed of a new document: everything at its default, only the
// student's calendar identity (`start`) and study-sessions horizon carried
// over — the cap and the étés are facts of the program being opened (ADR
// `2026-08-reglages-transversaux-dans-linstantane`); `study_sessions`
// starts from the program's own credit weight instead (ADR
// `2026-08-sessions-par-defaut-derivees-des-credits`), still a
// student-editable knob afterwards.
pub fn fresh_plan(
    start: Semester,
    choice: ProgramChoice,
    study_sessions: usize,
    today: Semester,
) -> Plan {
    Plan {
        program: Some(choice),
        // a document nobody has dated yet never opens in the past (ADR
        // `2026-08-debut-ancre-sur-lhorloge`) — an explicit past start
        // (relevé Capsule, lien partagé, réglage manuel) never comes here
        start: floor_start(start, today),
        study_sessions,
        ..Plan::default()
    }
}

// Ephemeral-but-restored navigation state: never undoable (undo acts on
// the document, not on where the student is looking), fully persisted so a
// reload lands exactly where he was (ACT-7).
#[derive(
    Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct View {
    // 1-based index in the horizon
    pub session: usize,
    pub search: String,
    pub subject: Option<String>,
    pub expanded_rule: Option<String>,
}

impl Default for View {
    fn default() -> Self {
        View {
            session: 1,
            search: String::new(),
            subject: None,
            expanded_rule: None,
        }
    }
}

// --- the session identity walk -------------------------------------------

// the calendar arithmetic lives in core, so every surface — the worker, the
// view — walks the sessions the same way, none of it in the view
pub use ulaval_scheduler_core::session_semesters;

// the `schedule_intake` naming — « a2026 », « h2027 », « e2027 »
pub fn session_key(semester: Semester) -> String {
    let letter = match semester.season {
        Season::Fall => 'a',
        Season::Winter => 'h',
        Season::Summer => 'e',
    };
    format!("{letter}{}", semester.year)
}

// The ribbon label: « A3-A27 » — the ordinal counts study sessions only,
// an été carries its vintage alone (« É27 »). Total on any index: an
// out-of-range one answers « ? » instead of panicking.
pub fn session_label(semesters: &[Semester], index: usize) -> String {
    let Some(&semester) = semesters.get(index) else {
        return "?".to_string();
    };
    if semester.season == Season::Summer {
        // « É27 » on screen; the plain-ascii « E27 » stays a file/key thing
        return format!("É{:02}", semester.year % 100);
    }
    let ordinal = semesters[..=index]
        .iter()
        .filter(|semester| semester.season != Season::Summer)
        .count();
    // the été returned above: only automne and hiver carry an ordinal
    let letter = if semester.season == Season::Fall {
        'A'
    } else {
        'H'
    };
    format!("{letter}{ordinal}-{semester}")
}

// « A3 » / « H4 » — the ordinal alone, for lines that already spell the
// semester out (« Horaire - H4 — Hiver 2026 », note 15) ; an été keeps
// its bare « É27 » label
pub fn session_short(semesters: &[Semester], index: usize) -> String {
    let label = session_label(semesters, index);
    label
        .split_once('-')
        .map(|(short, _)| short.to_string())
        .unwrap_or(label)
}

// The semester the real-world clock is in — month 1–4 is an hiver, 5–8 an
// été, 9–12 an automne. Days-to-civil is Howard Hinnant's algorithm,
// exact over the whole epoch range.
pub fn semester_of_epoch_ms(epoch_ms: u64) -> Semester {
    let days = (epoch_ms / 86_400_000) as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        / 365;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let mut year = year_of_era + era * 400;
    if month <= 2 {
        year += 1;
    }
    let season = match month {
        1..=4 => Season::Winter,
        5..=8 => Season::Summer,
        _ => Season::Fall,
    };
    Semester {
        season,
        year: year.clamp(0, i64::from(u16::MAX)) as u16,
    }
}

// Where a semester falls in real time — hiver, été, automne within a civil
// year. Ordering by this beats ordering by the « A26 » spelling, which sorts
// every automne before every hiver.
pub fn semester_rank(semester: Semester) -> (u16, u8) {
    let season = match semester.season {
        Season::Winter => 0u8,
        Season::Summer => 1,
        Season::Fall => 2,
    };
    (semester.year, season)
}

// strictly before, in real time
pub fn semester_precedes(before: Semester, after: Semester) -> bool {
    semester_rank(before) < semester_rank(after)
}

// The session a student deciding today would be admitted into. No bac
// admits in an été (`possible_semester_start` only ever reads « A » or
// « H »), so a summer clock snaps to the automne of its own civil year;
// an automne and an hiver are already their own admission session.
pub fn next_admission_semester(today: Semester) -> Semester {
    match today.season {
        Season::Summer => Semester {
            season: Season::Fall,
            year: today.year,
        },
        _ => today,
    }
}

// Raise-only: a « Début » already at or after the clock is left exactly as
// it is — only a start nobody chose, inherited from the factory default,
// gets pulled up to the next admission session.
pub fn floor_start(start: Semester, today: Semester) -> Semester {
    let floor = next_admission_semester(today);
    if semester_precedes(start, floor) {
        floor
    } else {
        start
    }
}

// The « Début » selector's span, in the two-digit years the options are
// spelled with. It always covers the plan's own start — a relevé Capsule
// can anchor it years back, and an option list missing it would leave the
// select silently showing the wrong session — and always reaches five
// years past the clock, so there is room to plan ahead.
pub fn start_year_window(start: Semester, today: Semester) -> (u16, u16) {
    let start_year = start.year % 100;
    let today_year = today.year % 100;
    (start_year.min(today_year), start_year.max(today_year + 5))
}

// --- what one session holds ----------------------------------------------

// The codes the weekly schedule of `session` draws: the displayed
// placement's courses there, then the hand-added ones (v0 flow), in a
// stable order, without duplicates.
pub fn session_codes(plan: &Plan, session: usize) -> Vec<String> {
    let mut codes: Vec<String> = plan
        .displayed_placement
        .iter()
        .filter(|(_, &placed)| placed == session)
        .map(|(code, _)| code.clone())
        .collect();
    for code in plan.manual.get(&session).into_iter().flatten() {
        if !codes.contains(code) {
            codes.push(code.clone());
        }
    }
    codes
}

// Créditer : the course is held by agreement, so it leaves every session —
// but keeps its entente: the credit says the course is held, the grant says
// which rule it counts toward.
pub fn credit_code(plan: &mut Plan, code: &str) {
    let code = code.to_string();
    purge_codes(plan, std::slice::from_ref(&code));
    plan.credited.insert(code);
}

// Décréditer : the useful inverse of the credit. The pre-credit session is
// gone with the purge, so the course re-enters as an elective — counted
// again at once, the solver gives it a session on the next propose (ADR
// `2026-08-decrediter-reprend-le-cours-en-electif`).
pub fn uncredit_code(plan: &mut Plan, code: &str) {
    plan.credited.remove(code);
    if !plan.electives.iter().any(|held| held == code) {
        plan.electives.push(code.to_string());
    }
}

// The voluntary ✕ : every placement trace goes, and the entente with it —
// unlike the credit purge, dropping the course drops the agreement too.
pub fn remove_course(plan: &mut Plan, code: &str) {
    let code = code.to_string();
    purge_codes(plan, std::slice::from_ref(&code));
    plan.rule_grants.remove(&code);
}

// `displayed_placement` is « pins plus the last accepted proposal » : a
// proposal computed before the latest pins (a solve in flight while a
// relevé Capsule import landed) must not evict them from the shown grid —
// the pin keeps its seat unless the proposal itself seats the code.
pub fn overlay_pins(
    plan: &Plan,
    placement: &mut std::collections::BTreeMap<String, usize>,
) {
    for (code, &session) in &plan.pinned_sessions {
        placement.entry(code.clone()).or_insert(session);
    }
}

// Shrinking the horizon must leave nothing seated beyond it, or the next
// verify — which pins everything displayed — refuses the plan (« GEX-2001
// is pinned to session 11, outside 1..=9 », 2026-08-26). Explicit acts are
// sovereign and floor the shrink instead of being evicted: pins, manual
// courses, and the sessions a relevé closed (plus one still open to study
// in). Automatic seats past the new edge fall back to « automatique » and
// the next propose re-seats them inside.
pub fn horizon_floor(plan: &Plan) -> usize {
    study_sessions_for_slot(plan.start.season, binding_slot(plan))
}

// Pins, hand-added sessions and the relevé's completed count are 1-based
// indices into the *expanded* horizon — core inserts an été after each
// hiver — while `study_sessions` counts only the A/H alternation. Reading
// a slot index as a session count is what kept an organigramme at 7
// sessions when 5 held its last pin (retour d'Antoine 2026-08-26).
fn binding_slot(plan: &Plan) -> usize {
    let pinned = plan.pinned_sessions.values().copied().max().unwrap_or(0);
    let manual = plan
        .manual
        .iter()
        .filter(|(_, codes)| !codes.is_empty())
        .map(|(&session, _)| session)
        .max()
        .unwrap_or(0);
    // the highest frozen session binds: it must stay on the horizon, plus
    // one open slot beyond the settled past for what remains to place
    let frozen = plan.frozen.iter().max().copied().unwrap_or(0);
    pinned.max(manual).max(frozen.saturating_add(1))
}

// The smallest alternation count whose expanded horizon reaches `slot`:
// `horizon_sessions` grows monotonically with the count, so the first hit
// is the smallest — the bounded walk of `core::transcript::grow_horizon`,
// never a `while`. Falls back to MAX so a corrupt save cannot yield a
// floor above the ceiling and make `set_horizon`'s clamp panic; the stray
// seats get evicted instead.
fn study_sessions_for_slot(start: Season, slot: usize) -> usize {
    (2..=MAX_STUDY_SESSIONS)
        .find(|&count| slots(start, count) >= slot)
        .unwrap_or(MAX_STUDY_SESSIONS)
}

// how many session columns a horizon of `count` study sessions holds —
// the inserted étés included
fn slots(start: Season, count: usize) -> usize {
    ulaval_scheduler_core::horizon_sessions(start, count).len()
}

// The one entry point for changing `study_sessions`: clamps to
// [floor, MAX] and evicts the automatic seats the new horizon can no
// longer hold — pins all sit at or below the floor, so `displayed ⊇
// pinned` survives the eviction. Returns the horizon actually set, for
// the caller's undo label. Also the self-heal for saves from before this
// rule: re-asserting the current value repairs any stray seat.
pub fn set_horizon(plan: &mut Plan, requested: usize) -> usize {
    let horizon = requested.clamp(horizon_floor(plan), MAX_STUDY_SESSIONS);
    plan.study_sessions = horizon;
    // the seats speak slots, not study sessions
    let held = slots(plan.start.season, horizon);
    plan.displayed_placement
        .retain(|_, &mut session| session <= held);
    horizon
}

// Why the horizon refuses to shrink below its floor, the binding fact
// named with the gesture that frees it — a knob that clamps in silence is
// the refusal AIR forbids (retour d'Antoine 2026-08-26 : « un bogue qui
// m'empêche de réduire le nombre de sessions »).
pub fn horizon_floor_note(plan: &Plan) -> String {
    let floor = horizon_floor(plan);
    // the session that binds is a slot; the horizon that results is a
    // count of study sessions — the note names both, in their own units
    let slot = binding_slot(plan);
    let seasons = ulaval_scheduler_core::horizon_sessions(
        plan.start.season,
        plan.study_sessions,
    );
    let semesters =
        ulaval_scheduler_core::session_semesters(plan.start, &seasons);
    let label = session_label(&semesters, slot.wrapping_sub(1));
    let pinned_at_floor: Vec<&str> = plan
        .pinned_sessions
        .iter()
        .filter(|(_, &session)| session == slot)
        .map(|(code, _)| code.as_str())
        .collect();
    if !pinned_at_floor.is_empty() {
        return format!(
            "L'horizon reste à {floor} sessions : {} en {label} — \
             dépinglez pour réduire davantage.",
            if pinned_at_floor.len() > 1 {
                format!("{} sont épinglés", pinned_at_floor.join(", "))
            } else {
                format!("{} est épinglé", pinned_at_floor[0])
            }
        );
    }
    if plan
        .manual
        .get(&slot)
        .is_some_and(|codes| !codes.is_empty())
    {
        return format!(
            "L'horizon reste à {floor} sessions : des cours manuels \
             occupent {label} — retirez-les pour réduire davantage."
        );
    }
    // `> 0`: an empty plan's binding slot is the single open session, no
    // frozen past behind it — that is the bare minimum, not a closed past
    let last_frozen = plan.frozen.iter().max().copied().unwrap_or(0);
    if last_frozen > 0 && last_frozen.saturating_add(1) == slot {
        let frozen_label =
            session_label(&semesters, last_frozen.wrapping_sub(1));
        return format!(
            "L'horizon reste à {floor} sessions : {frozen_label} est \
             gelée, et une session reste ouverte pour la suite."
        );
    }
    format!("L'horizon garde au moins {floor} sessions.")
}

// Strip the given codes from every placement structure — the derived
// correction behind « scolarité préparatoire faite » and « crédité » : a
// code acquired by hypothesis must not occupy any session, wherever it
// slipped in from.
pub fn purge_codes(plan: &mut Plan, codes: &[String]) {
    plan.electives.retain(|code| !codes.contains(code));
    // the tag dies with the course: re-taking it later is a fresh choice,
    // under whatever block the student is looking at then
    plan.elective_origins
        .retain(|code, _| !codes.contains(code));
    plan.pinned_sessions.retain(|code, _| !codes.contains(code));
    plan.displayed_placement
        .retain(|code, _| !codes.contains(code));
    for held in plan.manual.values_mut() {
        held.retain(|code| !codes.contains(code));
    }
    for pins in plan.chosen.values_mut() {
        pins.retain(|code, _| !codes.contains(code));
    }
}

// Remember which block an elective was chosen under, so changing that
// block can take it away again. `None` — a ribbon drag, a grid move, an
// elective the solver injected — is a no-op that never erases an existing
// tag: moving a course is not choosing it again.
pub fn tag_elective_origin(plan: &mut Plan, code: &str, origin: Option<char>) {
    let Some(origin) = origin else {
        return;
    };
    plan.elective_origins
        .insert(code.to_string(), origin.to_string());
}

// The tag a course carries right now — for a caller that purges its
// traces and lays it down again in one act (`place_course`): the purge
// takes the tag with everything else, and a move must give it back.
pub fn elective_origin(plan: &Plan, code: &str) -> Option<char> {
    plan.elective_origins
        .get(code)
        .and_then(|origin| origin.chars().next())
}

// the codes chosen under one block, sorted (a `BTreeMap` walk)
pub fn scoped_electives(plan: &Plan, prefix: char) -> Vec<String> {
    let prefix = prefix.to_string();
    plan.elective_origins
        .iter()
        .filter(|(_, origin)| **origin == prefix)
        .map(|(code, _)| code.clone())
        .collect()
}

// Changing the concentration (prefix 'c') or the profile ('f') retires the
// ententes attached to the outgoing block: an entente names a rule of that
// block, and the same title under another block is another rule. The
// dropped codes come back for the caller to announce — never a silent loss
// (décision 2026-08-19).
pub fn purge_scope_grants(plan: &mut Plan, prefix: char) -> Vec<String> {
    let scoped = format!("{prefix}/");
    let dropped: Vec<String> = plan
        .rule_grants
        .iter()
        .filter(|(_, key)| key.starts_with(&scoped))
        .map(|(code, _)| code.clone())
        .collect();
    plan.rule_grants.retain(|_, key| !key.starts_with(&scoped));
    dropped
}

// --- undo/redo ------------------------------------------------------------

// ACT-2: every mutation of the Plan goes through `apply`, so every one is
// reversible and labelled — no confirmation dialog anywhere.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    undo: Vec<(String, Plan)>,
    redo: Vec<(String, Plan)>,
    // the plan on screen was put back by an undo/redo, not built by an
    // edit: the automatic repair must leave it exactly as restored, or
    // « Annuler » takes two clicks (ADR
    // `2026-08-annuler-fige-l-ecran-restaure`). Any real edit re-arms it.
    restored: bool,
}

impl History {
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|(label, _)| label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|(label, _)| label.as_str())
    }

    pub fn restored(&self) -> bool {
        self.restored
    }
}

pub fn apply(
    plan: &mut Plan,
    history: &mut History,
    label: &str,
    edit: impl FnOnce(&mut Plan),
) {
    history.undo.push((label.to_string(), plan.clone()));
    if history.undo.len() > HISTORY_CAP {
        history.undo.remove(0);
    }
    history.redo.clear();
    history.restored = false;
    edit(plan);
}

pub fn undo(plan: &mut Plan, history: &mut History) -> Option<String> {
    let (label, previous) = history.undo.pop()?;
    history.redo.push((label.clone(), plan.clone()));
    *plan = previous;
    history.restored = true;
    Some(label)
}

pub fn redo(plan: &mut Plan, history: &mut History) -> Option<String> {
    let (label, next) = history.redo.pop()?;
    history.undo.push((label.clone(), plan.clone()));
    *plan = next;
    history.restored = true;
    Some(label)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    fn semester(raw: &str) -> Semester {
        raw.parse().unwrap_or_else(|e| panic!("{e}"))
    }

    const HORIZON: [Season; 6] = [
        Season::Fall,
        Season::Winter,
        Season::Summer,
        Season::Fall,
        Season::Winter,
        Season::Summer,
    ];

    #[test]
    fn overlay_pins_restores_a_pin_a_stale_proposal_dropped() {
        let mut plan = Plan::default();
        plan.pinned_sessions.insert("GEX-1000".to_string(), 2);
        plan.pinned_sessions.insert("GEX-2000".to_string(), 3);
        let mut placement = std::collections::BTreeMap::from([
            ("GEX-1000".to_string(), 5),
            ("GLO-1000".to_string(), 1),
        ]);
        overlay_pins(&plan, &mut placement);
        assert_eq!(
            placement["GEX-1000"], 5,
            "a seat the proposal itself gives wins — the next solve \
             re-honours the pin"
        );
        assert_eq!(placement["GEX-2000"], 3, "the dropped pin is restored");
        assert_eq!(placement["GLO-1000"], 1, "untouched");
    }

    #[test]
    fn the_horizon_floor_holds_every_explicit_act() {
        let mut plan = Plan::default();
        assert_eq!(horizon_floor(&plan), 2, "an empty plan floors at 2");
        plan.pinned_sessions.insert("GEX-1000".to_string(), 5);
        plan.manual.insert(7, vec!["MAN-1000".to_string()]);
        plan.manual.insert(9, Vec::new());
        plan.frozen.insert(3);
        assert_eq!(
            horizon_floor(&plan),
            5,
            "the highest of pins, non-empty manual sessions and the \
             frozen past — an emptied manual session holds nothing — \
             read back in study sessions: slot 7 of an automne start is \
             A1-H1-É1-A2-H2-É2-A3, five study sessions"
        );
        plan.pinned_sessions.insert("GEX-2000".to_string(), 99);
        assert_eq!(
            horizon_floor(&plan),
            MAX_STUDY_SESSIONS,
            "a corrupt floor caps at MAX instead of panicking clamp"
        );
    }

    #[test]
    fn shrinking_the_horizon_evicts_automatic_seats_never_a_pin() {
        let mut plan = Plan {
            study_sessions: 12,
            pinned_sessions: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            displayed_placement: BTreeMap::from([
                ("GEX-1000".to_string(), 2),
                ("GEX-2001".to_string(), 17),
            ]),
            ..Plan::default()
        };
        assert_eq!(set_horizon(&mut plan, 9), 9);
        assert_eq!(plan.study_sessions, 9);
        assert_eq!(
            plan.displayed_placement,
            BTreeMap::from([("GEX-1000".to_string(), 2)]),
            "the automatic seat at 17 — past the 13 slots nine study \
             sessions hold — falls back to « automatique », the pinned \
             one keeps its seat"
        );
        assert_eq!(
            set_horizon(&mut plan, 1),
            2,
            "the shrink stops at the floor"
        );
        assert_eq!(
            set_horizon(&mut plan, 99),
            MAX_STUDY_SESSIONS,
            "the growth stops at MAX"
        );
    }

    #[test]
    fn the_floor_note_names_the_binding_fact_and_the_gesture() {
        // a pin at the floor: named, with its session label
        let mut plan = Plan {
            study_sessions: 8,
            ..Plan::default()
        };
        plan.pinned_sessions.insert("GEX-3333".to_string(), 7);
        let note = horizon_floor_note(&plan);
        assert!(note.contains("5 sessions"), "{note}");
        assert!(note.contains("GEX-3333 est épinglé"), "{note}");
        assert!(note.contains("dépinglez"), "{note}");
        // several pins there: plural, all named
        plan.pinned_sessions.insert("GEX-2003".to_string(), 7);
        let note = horizon_floor_note(&plan);
        assert!(note.contains("GEX-2003, GEX-3333 sont épinglés"), "{note}");

        // manual courses hold the floor
        let mut plan = Plan {
            study_sessions: 8,
            ..Plan::default()
        };
        plan.manual.insert(5, vec!["MAN-1000".to_string()]);
        let note = horizon_floor_note(&plan);
        assert!(note.contains("cours manuels"), "{note}");
        assert!(note.contains("retirez-les"), "{note}");

        // the frozen sessions hold it — the highest one binds
        let plan = Plan {
            study_sessions: 8,
            frozen: BTreeSet::from([2, 5]),
            ..Plan::default()
        };
        let note = horizon_floor_note(&plan);
        assert!(note.contains("est gelée"), "{note}");
        assert!(note.contains("reste ouverte pour la suite"), "{note}");

        // nothing explicit: the bare minimum speaks for itself
        let note = horizon_floor_note(&Plan::default());
        assert!(note.contains("au moins 2 sessions"), "{note}");
    }

    // The organigramme Antoine could not shrink below 7 (2026-08-26): its
    // last pins sit in slot 7 and its last seats in slot 8, both of which
    // five study sessions already hold — the floor used to read the slot
    // index as a session count.
    #[test]
    fn the_floor_and_the_eviction_speak_slots_not_study_sessions() {
        let mut plan = Plan {
            study_sessions: 7,
            pinned_sessions: BTreeMap::from([("GEX-3333".to_string(), 7)]),
            displayed_placement: BTreeMap::from([
                ("GEX-3333".to_string(), 7),
                ("GEX-2001".to_string(), 8),
            ]),
            ..Plan::default()
        };
        assert_eq!(horizon_floor(&plan), 5, "slot 7 needs five sessions");
        assert_eq!(set_horizon(&mut plan, 5), 5);
        assert_eq!(
            plan.displayed_placement.len(),
            1,
            "seven slots hold the pin at 7; the seat at 8 is evicted"
        );
        assert_eq!(set_horizon(&mut plan, 6), 6);
        assert_eq!(
            slots(plan.start.season, 6),
            9,
            "six study sessions hold nine slots"
        );
    }

    #[test]
    fn a_fresh_plan_carries_the_start_the_choice_and_the_sessions() {
        let start = semester("H27");
        let choice = ProgramChoice {
            code: "B-GIN".to_string(),
            semester: "H27".to_string(),
            concentration: Some("Approche généraliste".to_string()),
            profile: None,
        };
        let fresh = fresh_plan(start, choice.clone(), 6, semester("A26"));
        assert_eq!(fresh.program, Some(choice.clone()));
        assert_eq!(fresh.start, start);
        assert_eq!(fresh.study_sessions, 6, "the passed count lands");
        // everything else is the default — field by field, so a new Plan
        // field forgetting this contract fails here
        assert_eq!(
            Plan {
                program: None,
                start: Plan::default().start,
                study_sessions: Plan::default().study_sessions,
                ..fresh
            },
            Plan::default()
        );
        // a start the clock has walked past is nobody's choice: the fresh
        // document opens on the next admission session instead
        let dated = fresh_plan(semester("A26"), choice, 6, semester("H28"));
        assert_eq!(dated.start, semester("H28"));
    }

    #[test]
    fn next_admission_semester_snaps_a_summer_to_its_fall() {
        assert_eq!(
            next_admission_semester(semester("E27")),
            semester("A27"),
            "no bac admits in an été — the automne of the same civil year"
        );
        assert_eq!(next_admission_semester(semester("A27")), semester("A27"));
        assert_eq!(next_admission_semester(semester("H27")), semester("H27"));
    }

    #[test]
    fn floor_start_raises_only_a_past_start() {
        let today = semester("A27");
        assert_eq!(floor_start(semester("A26"), today), today, "raised");
        assert_eq!(floor_start(semester("H27"), today), today, "raised");
        assert_eq!(
            floor_start(semester("H28"), today),
            semester("H28"),
            "a start already ahead of the clock is left alone"
        );
        assert_eq!(floor_start(today, today), today, "the clock itself");
        // an été clock floors on its own automne, not on the été
        assert_eq!(
            floor_start(semester("H27"), semester("E27")),
            semester("A27")
        );
    }

    #[test]
    fn start_year_window_always_covers_start_and_clock() {
        assert_eq!(
            start_year_window(semester("A26"), semester("A26")),
            (26, 31),
            "five years of headroom past the clock"
        );
        assert_eq!(
            start_year_window(semester("H22"), semester("A27")),
            (22, 32),
            "a relevé anchored years back widens the low end"
        );
        assert_eq!(
            start_year_window(semester("A40"), semester("A27")),
            (27, 40),
            "a start past the headroom widens the high end"
        );
    }

    #[test]
    fn default_study_sessions_follows_credits_ceiling_and_clamps() {
        for (credits, expected) in [
            (90, 6),
            (120, 8),
            (89, 6),
            (91, 7),
            (0, DEFAULT_STUDY_SESSIONS),
            (-30, DEFAULT_STUDY_SESSIONS),
            (400, 16),
            (15, 2),
        ] {
            assert_eq!(
                default_study_sessions(credits),
                expected,
                "{credits} credits"
            );
        }
    }

    #[test]
    fn session_keys_speak_the_intake_naming() {
        let semesters = session_semesters(semester("A26"), &HORIZON);
        let keys: Vec<String> =
            semesters.iter().map(|&s| session_key(s)).collect();
        assert_eq!(
            keys,
            ["a2026", "h2027", "e2027", "a2027", "h2028", "e2028"]
        );
    }

    #[test]
    fn labels_number_study_sessions_and_leave_etes_bare() {
        let semesters = session_semesters(semester("A26"), &HORIZON);
        let labels: Vec<String> = (0..semesters.len())
            .map(|i| session_label(&semesters, i))
            .collect();
        assert_eq!(
            labels,
            ["A1-A26", "H2-H27", "É27", "A3-A27", "H4-H28", "É28"]
        );
        assert_eq!(session_label(&semesters, 99), "?", "total, no panic");
        let shorts: Vec<String> = (0..semesters.len())
            .map(|i| session_short(&semesters, i))
            .collect();
        assert_eq!(shorts, ["A1", "H2", "É27", "A3", "H4", "É28"]);
        assert_eq!(session_short(&semesters, 99), "?", "total, no panic");
    }

    #[test]
    fn the_clock_semester_names_the_three_seasons_and_leap_days() {
        let at = |ms: u64| semester_of_epoch_ms(ms).to_string();
        assert_eq!(at(0), "H70", "1970-01-01");
        assert_eq!(at(1_786_579_200_000), "E26", "2026-08-13");
        assert_eq!(at(1_788_220_800_000), "A26", "2026-09-01");
        assert_eq!(at(1_798_675_200_000), "A26", "2026-12-31");
        assert_eq!(at(1_799_107_200_000), "H27", "2027-01-05");
        assert_eq!(at(1_709_164_800_000), "H24", "2024-02-29 (bissextile)");
    }

    #[test]
    fn semesters_precede_in_civil_time_hiver_ete_automne() {
        let order = ["H26", "E26", "A26", "H27"].map(semester);
        for (i, &earlier) in order.iter().enumerate() {
            assert!(!semester_precedes(earlier, earlier), "irreflexive");
            for &later in &order[i + 1..] {
                assert!(semester_precedes(earlier, later));
                assert!(!semester_precedes(later, earlier));
            }
        }
    }

    #[test]
    fn purge_codes_strips_the_named_codes_from_every_structure() {
        let mut plan = Plan {
            electives: vec!["MAT-0130".to_string(), "GEX-1000".to_string()],
            pinned_sessions: BTreeMap::from([("MAT-0130".to_string(), 1)]),
            displayed_placement: BTreeMap::from([
                ("MAT-0130".to_string(), 1),
                ("GEX-1000".to_string(), 2),
            ]),
            chosen: BTreeMap::from([(
                1,
                BTreeMap::from([
                    ("MAT-0130".to_string(), BTreeSet::new()),
                    ("GEX-1000".to_string(), BTreeSet::new()),
                ]),
            )]),
            manual: BTreeMap::from([(
                1,
                vec!["MAT-0130".to_string(), "GEX-1000".to_string()],
            )]),
            ..Plan::default()
        };
        purge_codes(&mut plan, &["MAT-0130".to_string()]);
        assert_eq!(plan.electives, ["GEX-1000"]);
        assert!(plan.pinned_sessions.is_empty());
        assert!(!plan.displayed_placement.contains_key("MAT-0130"));
        assert_eq!(plan.displayed_placement["GEX-1000"], 2);
        assert_eq!(plan.manual[&1], ["GEX-1000"]);
        assert!(!plan.chosen[&1].contains_key("MAT-0130"));
        assert!(plan.chosen[&1].contains_key("GEX-1000"));
    }

    #[test]
    fn purge_codes_drops_the_origin_tags() {
        let mut plan = Plan {
            electives: vec!["FOR-2020".to_string(), "GEX-1000".to_string()],
            elective_origins: BTreeMap::from([
                ("FOR-2020".to_string(), "c".to_string()),
                ("GEX-1000".to_string(), "f".to_string()),
            ]),
            ..Plan::default()
        };
        purge_codes(&mut plan, &["FOR-2020".to_string()]);
        assert_eq!(
            plan.elective_origins,
            BTreeMap::from([("GEX-1000".to_string(), "f".to_string())]),
            "re-taking the course later is a fresh choice"
        );
    }

    #[test]
    fn tag_elective_origin_keeps_an_existing_tag_on_none() {
        let mut plan = Plan::default();
        tag_elective_origin(&mut plan, "FOR-2020", Some('c'));
        assert_eq!(plan.elective_origins["FOR-2020"], "c");
        // a ribbon drag or a grid move: a move is not a re-choice
        tag_elective_origin(&mut plan, "FOR-2020", None);
        assert_eq!(plan.elective_origins["FOR-2020"], "c");
        // an untagged course dragged around stays untagged
        tag_elective_origin(&mut plan, "GEX-1000", None);
        assert!(!plan.elective_origins.contains_key("GEX-1000"));
        // choosing it again under another block moves the tag
        tag_elective_origin(&mut plan, "FOR-2020", Some('f'));
        assert_eq!(plan.elective_origins["FOR-2020"], "f");
        // read back for the purge-then-lay-down move (`place_course`)
        assert_eq!(elective_origin(&plan, "FOR-2020"), Some('f'));
        assert_eq!(elective_origin(&plan, "GEX-1000"), None);
        // a tag corrupted to the empty string names no block
        plan.elective_origins
            .insert("GEX-1000".to_string(), String::new());
        assert_eq!(elective_origin(&plan, "GEX-1000"), None);
    }

    #[test]
    fn scoped_electives_reads_only_its_prefix() {
        let plan = Plan {
            elective_origins: BTreeMap::from([
                ("GEX-2000".to_string(), "c".to_string()),
                ("FOR-2020".to_string(), "c".to_string()),
                ("ANL-2020".to_string(), "f".to_string()),
            ]),
            ..Plan::default()
        };
        assert_eq!(scoped_electives(&plan, 'c'), ["FOR-2020", "GEX-2000"]);
        assert_eq!(scoped_electives(&plan, 'f'), ["ANL-2020"]);
        assert!(scoped_electives(&plan, 'p').is_empty());
    }

    #[test]
    fn an_old_save_without_origins_restores_untagged() {
        // rétro-compat gh.v1.plan : the field simply defaults
        let bare: Plan = serde_json::from_str(
            r#"{"electives":["FOR-2020"],"credit_cap":15}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bare.electives, ["FOR-2020"]);
        assert!(bare.elective_origins.is_empty());
        assert_eq!(bare.credit_cap, 15);
    }

    #[test]
    fn changing_a_scope_purges_its_grants_and_names_them() {
        let mut plan = Plan {
            rule_grants: BTreeMap::from([
                ("GEX-1000".to_string(), "c/Règle C1".to_string()),
                ("GEX-2000".to_string(), "p/Règle 2".to_string()),
                ("GEX-3000".to_string(), "f/Règle P1".to_string()),
            ]),
            ..Plan::default()
        };
        let dropped = purge_scope_grants(&mut plan, 'c');
        assert_eq!(dropped, ["GEX-1000"], "the outgoing block's only");
        assert_eq!(plan.rule_grants.len(), 2, "p/ and f/ grants survive");
        assert!(
            purge_scope_grants(&mut plan, 'c').is_empty(),
            "nothing left to purge, nothing announced"
        );
    }

    #[test]
    fn crediting_purges_the_placement_but_keeps_the_entente() {
        let mut plan = Plan {
            electives: vec!["GEX-1000".to_string()],
            pinned_sessions: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            displayed_placement: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            rule_grants: BTreeMap::from([(
                "GEX-1000".to_string(),
                "p/Règle 2".to_string(),
            )]),
            ..Plan::default()
        };
        credit_code(&mut plan, "GEX-1000");
        assert!(plan.credited.contains("GEX-1000"));
        assert!(plan.electives.is_empty());
        assert!(plan.pinned_sessions.is_empty());
        assert!(plan.displayed_placement.is_empty());
        assert_eq!(
            plan.rule_grants["GEX-1000"], "p/Règle 2",
            "the credit stacks with the entente"
        );
    }

    #[test]
    fn uncrediting_takes_the_course_back_as_an_elective() {
        let mut plan = Plan {
            displayed_placement: BTreeMap::from([("GEX-1000".to_string(), 2)]),
            ..Plan::default()
        };
        credit_code(&mut plan, "GEX-1000");
        uncredit_code(&mut plan, "GEX-1000");
        assert!(!plan.credited.contains("GEX-1000"));
        assert_eq!(
            plan.electives,
            ["GEX-1000"],
            "taken again, session left to the solver"
        );
        assert!(plan.displayed_placement.is_empty(), "no ghost session");
        // a code already elective is not duplicated
        uncredit_code(&mut plan, "GEX-1000");
        assert_eq!(plan.electives, ["GEX-1000"]);
    }

    #[test]
    fn removing_a_course_drops_its_entente_with_it() {
        let mut plan = Plan {
            electives: vec!["GLG-1001".to_string()],
            rule_grants: BTreeMap::from([(
                "GLG-1001".to_string(),
                "p/Règle 5".to_string(),
            )]),
            ..Plan::default()
        };
        remove_course(&mut plan, "GLG-1001");
        assert!(plan.electives.is_empty());
        assert!(plan.rule_grants.is_empty(), "the ✕ purges the entente");
    }

    #[test]
    fn session_codes_join_placement_and_manual_without_duplicates() {
        let plan = Plan {
            displayed_placement: BTreeMap::from([
                ("GEX-1000".to_string(), 1),
                ("GCI-1007".to_string(), 2),
            ]),
            manual: BTreeMap::from([(
                1,
                vec!["ANL-2020".to_string(), "GEX-1000".to_string()],
            )]),
            ..Plan::default()
        };
        assert_eq!(session_codes(&plan, 1), ["GEX-1000", "ANL-2020"]);
        assert_eq!(session_codes(&plan, 2), ["GCI-1007"]);
        assert!(session_codes(&plan, 3).is_empty());
    }

    #[test]
    fn apply_undo_redo_walk_the_same_labelled_steps() {
        let mut plan = Plan::default();
        let mut history = History::default();
        apply(&mut plan, &mut history, "Ajout de GEX-1000", |plan| {
            plan.electives.push("GEX-1000".to_string());
        });
        apply(&mut plan, &mut history, "Ajout de GCI-1007", |plan| {
            plan.electives.push("GCI-1007".to_string());
        });
        assert_eq!(history.undo_label(), Some("Ajout de GCI-1007"));

        assert_eq!(
            undo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GCI-1007")
        );
        assert_eq!(plan.electives, ["GEX-1000"]);
        assert_eq!(history.redo_label(), Some("Ajout de GCI-1007"));

        assert_eq!(
            redo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GCI-1007")
        );
        assert_eq!(plan.electives, ["GEX-1000", "GCI-1007"]);

        undo(&mut plan, &mut history);
        undo(&mut plan, &mut history);
        assert_eq!(plan, Plan::default());
        assert_eq!(undo(&mut plan, &mut history), None, "nothing left");
        assert_eq!(
            redo(&mut plan, &mut history).as_deref(),
            Some("Ajout de GEX-1000")
        );
    }

    #[test]
    fn undo_marks_the_screen_restored_until_the_next_edit() {
        let mut plan = Plan::default();
        let mut history = History::default();
        assert!(!history.restored(), "a fresh history restored nothing");
        apply(&mut plan, &mut history, "A", |plan| plan.credit_cap = 12);
        assert!(!history.restored(), "an edit builds the screen");
        assert_eq!(undo(&mut plan, &mut history).as_deref(), Some("A"));
        assert!(history.restored(), "the screen comes from the history");
        apply(&mut plan, &mut history, "B", |plan| plan.credit_cap = 18);
        assert!(!history.restored(), "a real edit re-arms the repair");
        undo(&mut plan, &mut history);
        redo(&mut plan, &mut history);
        assert!(history.restored(), "redo restores a screen too");
        // an empty pop changes nothing: the flag only follows a real move
        let mut empty = History::default();
        assert_eq!(undo(&mut plan, &mut empty), None);
        assert!(!empty.restored());
        assert_eq!(redo(&mut plan, &mut empty), None);
        assert!(!empty.restored());
    }

    #[test]
    fn a_document_swap_rearms_the_repair() {
        let mut plan = Plan::default();
        let mut history = History::default();
        apply(&mut plan, &mut history, "A", |plan| plan.credit_cap = 12);
        undo(&mut plan, &mut history);
        assert!(history.restored());
        // `swap_document` and `import_organigramme` both install a fresh
        // History: the next document's screen is nobody's restoration
        history = History::default();
        assert!(!history.restored(), "the swap re-arms the repair");
    }

    #[test]
    fn a_new_edit_clears_the_redo_branch() {
        let mut plan = Plan::default();
        let mut history = History::default();
        apply(&mut plan, &mut history, "A", |plan| plan.credit_cap = 12);
        undo(&mut plan, &mut history);
        apply(&mut plan, &mut history, "B", |plan| plan.credit_cap = 18);
        assert_eq!(redo(&mut plan, &mut history), None, "the branch died");
        assert_eq!(plan.credit_cap, 18);
    }

    #[test]
    fn the_history_is_bounded_and_drops_its_oldest_step() {
        let mut plan = Plan::default();
        let mut history = History::default();
        for i in 0..(HISTORY_CAP + 10) {
            apply(&mut plan, &mut history, &format!("étape {i}"), |plan| {
                plan.study_sessions = i;
            });
        }
        let mut undone = 0;
        for _ in 0..(HISTORY_CAP + 10) {
            if undo(&mut plan, &mut history).is_none() {
                break;
            }
            undone += 1;
        }
        assert_eq!(undone, HISTORY_CAP, "the cap held");
        assert_eq!(plan.study_sessions, 9, "the ten oldest steps are gone");
    }

    #[test]
    fn plan_and_view_round_trip_and_tolerate_absent_fields() {
        let plan = Plan::default();
        let json = serde_json::to_string(&plan)
            .unwrap_or_else(|e| panic!("serialize: {e}"));
        let back: Plan = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("round trip: {e}"));
        assert_eq!(back, plan);
        // an empty object restores every field to its default
        let bare: Plan =
            serde_json::from_str("{}").unwrap_or_else(|e| panic!("bare: {e}"));
        assert_eq!(bare, Plan::default());
        let bare: View = serde_json::from_str("{}")
            .unwrap_or_else(|e| panic!("bare view: {e}"));
        assert_eq!(bare, View::default());
    }
}
